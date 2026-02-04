use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{fs, io};
use url::{ParseError, Url};

use crate::pattern::{GrabPattern, GrabPatternComponent, GrabPatternPlaceholder};
use crate::Error;

const HTTPS: &str = "https://";

pub fn grab(
    pattern: &GrabPattern,
    home: Option<&PathBuf>,
    url: OsString,
    dry_run: bool,
    git_args: &[OsString],
) -> Result<PathBuf, Error> {
    let str = url.to_str().ok_or("invalid url")?;
    let url: Url = parse_url(str)?;

    let dest_path = clone_path(pattern, home, &url);

    if dry_run {
        println!("Grab {} to {}", url, dest_path.display());
        return Ok(dest_path);
    }

    fs::create_dir_all(&dest_path)?;
    let status = clone(&url, &dest_path, git_args)?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| match status.code() {
            Some(code) => format!("git exited with status {}", code),
            None => String::from("git killed by signal"),
        })
        .map_err(|err| err.into())
        .map(|()| {
            println!("Grabbed {} to {}", url, dest_path.display());
            dest_path
        })
}

fn parse_url(url: &str) -> Result<Url, Error> {
    url.parse().or_else(|err| match err {
        ParseError::RelativeUrlWithoutBase => {
            if looks_like_ssh_url(url) {
                // Might be an ssh style URL like git@github.com:wezm/grab.git
                let newstr = normalise_ssh_url(url)
                    .ok_or_else(|| format!("unable to normalise '{}'", url))?;
                newstr.parse().map_err(Error::from)
            } else if url.contains('.') {
                // might be a URL without a scheme like github.com/wezm/grab
                let mut newstr = String::with_capacity(HTTPS.len() + url.len());
                newstr.push_str(HTTPS);
                newstr.push_str(url);
                newstr.parse().map_err(Error::from)
            } else {
                Err(format!("'{}': {}", url, err).into())
            }
        }
        _ => Err(format!("'{}': {}", url, err).into()),
    })
}

fn clone(url: &Url, dest_path: &Path, extra_args: &[OsString]) -> Result<ExitStatus, io::Error> {
    Command::new("git")
        .arg("clone")
        .args(extra_args)
        .arg(url.as_str())
        .arg(dest_path)
        .status()
}

struct UrlComponents {
    path: Option<String>,
    host: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
}

fn extract_url_components(url: &Url) -> UrlComponents {
    let host = url.host_str().map(|s| s.to_string());

    let path = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>().join("/"));

    let segments: Vec<&str> = url.path_segments().map_or(Vec::new(), |s| s.collect());

    let (owner, repo) = if segments.len() >= 2
        && (url.host_str() == Some("github.com")
            || url.host_str() == Some("gitlab.com")
            || url.host_str() == Some("bitbucket.org")
            || (url.host_str() == Some("git.sr.ht")))
    {
        // Known hosts that follow a standard owner/repo pattern in the first two segments of the URL path
        // E.g. https://github.com/owner/repo
        let owner = segments.get(0).map(|s| s.to_string());
        let repo = segments.get(1).map(|s| s.to_string());
        (owner, repo)
    } else {
        (None, None)
    };

    let repo = repo.map(|r| {
        // Remove .git extension from repo name if present
        if r.ends_with(".git") {
            r[..r.len() - 4].to_string()
        } else {
            r
        }
    });

    UrlComponents {
        path,
        host,
        owner,
        repo,
    }
}

fn clone_path(pattern: &GrabPattern, home: Option<&PathBuf>, url: &Url) -> PathBuf {
    let mut result = String::new();
    let mut remaining = &pattern.0[..];

    if let Some(component) = remaining.first() {
        if let GrabPatternComponent::Literal(lit) = component {
            if let Some(stripped) = lit.strip_prefix('~') {
                // Expand leading ~
                if let Some(home) = &home {
                    result.push_str(&home.to_string_lossy());
                    result.push_str(stripped);
                    remaining = &remaining[1..];
                }
            }
        }
    }

    // Pre-compute URL components once
    let url_components = extract_url_components(url);
    let home_string = home.as_ref().map(|p| p.to_string_lossy().to_string());

    while !remaining.is_empty() {
        let component = &remaining[0];

        match component {
            GrabPatternComponent::Literal(lit) => result.push_str(lit),
            GrabPatternComponent::Placeholder {
                placeholder,
                leading_slash,
                trailing_slash,
            } => {
                let value = match placeholder {
                    GrabPatternPlaceholder::Home => home_string.as_deref(),
                    GrabPatternPlaceholder::Host => url_components.host.as_deref(),
                    GrabPatternPlaceholder::Path => url_components.path.as_deref(),
                    GrabPatternPlaceholder::Owner => url_components.owner.as_deref(),
                    GrabPatternPlaceholder::Repo => url_components.repo.as_deref(),
                };

                if let Some(val) = value {
                    if *leading_slash {
                        result.push('/');
                    }
                    result.push_str(val);
                    if *trailing_slash {
                        result.push('/');
                    }
                }
            }
        }

        remaining = &remaining[1..];
    }

    let mut result = PathBuf::from(result);

    if result.extension() == Some(std::ffi::OsStr::new("git")) {
        result.set_extension("");
    }

    result
}

fn looks_like_ssh_url(url: &str) -> bool {
    // if there's an @ before the : maybe it's an ssh url
    split_once(url, ':').map_or(false, |(before, _after)| before.contains('@'))
}

fn normalise_ssh_url(url: &str) -> Option<String> {
    let colons = url.as_bytes().iter().filter(|&&b| b == b':').count();
    let mut p = url.split(':');

    match colons {
        // ssh url
        1 => Some(format!("ssh://{}/{}", p.next()?, p.next()?)),
        // ssh url with port
        2 => Some(format!("ssh://{}:{}/{}", p.next()?, p.next()?, p.next()?)),
        _ => None,
    }
}

// Use std lib when CI builds are on Rust >= 1.52
fn split_once(s: &str, pat: char) -> Option<(&str, &str)> {
    let (before, after) = s.split_at(s.find(pat)?);
    Some((before, &after[1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_ssh_url() {
        // true
        assert!(looks_like_ssh_url("git@github.com:wezm/grab.git"));
        assert!(looks_like_ssh_url("git@github.com:2222:wezm/grab.git"));
        assert!(looks_like_ssh_url("git@git.sr.ht:~wezm/lobsters"));

        // false
        assert!(!looks_like_ssh_url("https://github.com/wezm/grab.git"));
    }

    #[test]
    fn test_normalise_ssh_url() {
        assert_eq!(
            normalise_ssh_url("git@github.com:wezm/grab.git").unwrap(),
            "ssh://git@github.com/wezm/grab.git"
        );
        assert_eq!(
            normalise_ssh_url("git@github.com:2222:wezm/grab.git").unwrap(),
            "ssh://git@github.com:2222/wezm/grab.git"
        );

        assert!(normalise_ssh_url("github.com/wezm/grab.git").is_none());
    }

    #[test]
    fn test_split_once() {
        assert_eq!(split_once("a:b", ':'), Some(("a", "b")));
        assert_eq!(split_once(":b", ':'), Some(("", "b")));
        assert_eq!(split_once("a:", ':'), Some(("a", "")));
        assert_eq!(split_once(":", ':'), Some(("", "")));
        assert_eq!(split_once("abc", ':'), None);
        assert_eq!(split_once("", ':'), None);
    }

    #[test]
    fn test_clone_path_valid() {
        [
            // `/src/{host/}{path/}` pattern (default)
            (
                "/src/{host/}{path/}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "/src/{host/}{path/}",
                "https://github.com/influxdata/influxdb2-sample-data",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "/src/{host/}{path/}",
                "github.com/zesterer/tao",
                "/src/github.com/zesterer/tao",
            ),
            (
                "/src/{host/}{path/}",
                "github.com/denoland/deno/",
                "/src/github.com/denoland/deno/",
            ),
            (
                "/src/{host/}{path/}",
                "git@github.com:wezm/git-grab.git",
                "/src/github.com/wezm/git-grab",
            ),
            (
                "/src/{host/}{path/}",
                "git.sr.ht/~wezm/lobsters",
                "/src/git.sr.ht/~wezm/lobsters",
            ),
            (
                "/src/{host/}{path/}",
                "git@git.sr.ht:~wezm/lobsters",
                "/src/git.sr.ht/~wezm/lobsters",
            ),
            (
                "/src/{host/}{path/}",
                "bitbucket.org/egrange/dwscript",
                "/src/bitbucket.org/egrange/dwscript",
            ),
            (
                "/src/{host/}{path/}",
                "git://c9x.me/qbe.git",
                "/src/c9x.me/qbe",
            ),
            // `/src/{host/}{owner/}{repo}` pattern
            (
                "/src/{host/}{owner/}{repo}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "https://github.com/influxdata/influxdb2-sample-data",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "github.com/zesterer/tao",
                "/src/github.com/zesterer/tao",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "github.com/denoland/deno/",
                "/src/github.com/denoland/deno/",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "git@github.com:wezm/git-grab.git",
                "/src/github.com/wezm/git-grab",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "git.sr.ht/~wezm/lobsters",
                "/src/git.sr.ht/~wezm/lobsters",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "git@git.sr.ht:~wezm/lobsters",
                "/src/git.sr.ht/~wezm/lobsters",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "bitbucket.org/egrange/dwscript",
                "/src/bitbucket.org/egrange/dwscript",
            ),
            (
                "/src/{host/}{owner/}{repo}",
                "git://c9x.me/qbe.git",
                "/src/c9x.me/",
            ),
            (
                "/src/{host}/{owner}/{repo}",
                "git://c9x.me/qbe.git",
                "/src/c9x.me//",
            ),
            // Individual placeholders
            (
                "{host}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "github.com",
            ),
            (
                "{owner}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "influxdata",
            ),
            (
                "{repo}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "influxdb2-sample-data",
            ),
            (
                "{home}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/",
            ),
            (
                "{path}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "influxdata/influxdb2-sample-data/",
            ),
            // Leading and trailing slashes
            (
                "{/owner}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/influxdata",
            ),
            (
                "{owner/}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "influxdata/",
            ),
            ("{owner/}", "git://c9x.me/qbe.git", ""),
            ("{/owner}", "git://c9x.me/qbe.git", ""),
            (
                "/{/owner}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "//influxdata",
            ),
            (
                "{owner/}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "influxdata/",
            ),
            // Tilde not at start
            (
                "/test/~/{repo}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/test/~/influxdb2-sample-data",
            ),
            // Escaping braces
            (
                "/test/{{owner}}/{repo}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/test/{owner}/influxdb2-sample-data",
            ),
        ]
        .iter()
        .for_each(|(pattern, url, expected)| {
            assert_eq!(
                clone_path(
                    &GrabPattern::try_parse(pattern).unwrap(),
                    None,
                    &parse_url(url).unwrap()
                ),
                PathBuf::from(expected)
            )
        });
    }

    #[test]
    fn test_clone_path_with_custom_home_directory() {
        let custom_home = PathBuf::from("/custom/home");
        [
            (
                "~/src/{host/}{owner/}{repo}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/custom/home/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "{home}/src/{host/}{owner/}{repo}",
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/custom/home/src/github.com/influxdata/influxdb2-sample-data",
            ),
        ]
        .iter()
        .for_each(|(pattern, url, expected)| {
            assert_eq!(
                clone_path(
                    &GrabPattern::try_parse(pattern).unwrap(),
                    Some(&custom_home),
                    &parse_url(url).unwrap()
                ),
                PathBuf::from(expected)
            )
        });
    }
}
