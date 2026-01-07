use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{fs, io};
use url::{ParseError, Url};

use crate::args::GrabPattern;
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

    let dest_path = clone_path(pattern, home, &url)?;

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
    host: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
}

fn extract_url_components(url: &Url) -> UrlComponents {
    let host = url.host_str().map(|s| s.to_string());
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
    } else if !segments.is_empty() {
        // Other hosts - just use first segment as repo
        let repo = segments.get(0).map(|s| s.to_string());
        (None, repo)
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

    UrlComponents { host, owner, repo }
}

fn clone_path(pattern: &GrabPattern, home: Option<&PathBuf>, url: &Url) -> Result<PathBuf, Error> {
    let mut result = String::new();
    let mut remaining = pattern.0.as_str();

    if let Some(stripped) = remaining.strip_prefix('~') {
        if let Some(home) = &home {
            // Expand ~ to user home
            result.push_str(&home.to_string_lossy());
            remaining = stripped;
        }
    }

    // Pre-compute URL components once
    let url_components = extract_url_components(url);
    let home_string = home.as_ref().map(|p| p.to_string_lossy().to_string());

    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix('{') {
            // Handle escaping braces
            if rest.starts_with('{') {
                let end = rest.find('}').ok_or("unclosed placeholder")?;
                result.push_str(&rest[..end]);
                remaining = &rest[end + 1..];
                continue;
            }

            let end = rest.find('}').ok_or("unclosed placeholder")?;
            let placeholder = &rest[..end];
            let (leading_slash, trailing_slash) =
                (placeholder.starts_with('/'), placeholder.ends_with('/'));
            let placeholder = placeholder.trim_matches('/');

            let value = resolve_placeholder_value(placeholder, &url_components, &home_string)?;

            if let Some(val) = value {
                if leading_slash {
                    result.push('/');
                }
                result.push_str(val);
                if trailing_slash {
                    result.push('/');
                }
            }

            remaining = &rest[end + 1..];
        } else {
            let next_brace = remaining.find('{').unwrap_or(remaining.len());
            result.push_str(&remaining[..next_brace]);
            remaining = &remaining[next_brace..];
        }
    }

    Ok(PathBuf::from(result))
}

#[derive(Debug)]
struct UnknownPlaceholderError(String);

impl std::fmt::Display for UnknownPlaceholderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown placeholder: {}", self.0)
    }
}

impl std::error::Error for UnknownPlaceholderError {}

fn resolve_placeholder_value<'a>(
    placeholder: &str,
    components: &'a UrlComponents,
    home_string: &'a Option<String>,
) -> Result<Option<&'a str>, UnknownPlaceholderError> {
    match placeholder {
        "host" => Ok(components.host.as_deref()),
        "home" => Ok(home_string.as_deref()),
        "owner" => Ok(components.owner.as_deref()),
        "repo" => Ok(components.repo.as_deref()),
        _ => Err(UnknownPlaceholderError(placeholder.to_string())),
    }
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
                "/src/c9x.me/qbe",
            ),
            (
                "/src/{host}/{owner}/{repo}",
                "git://c9x.me/qbe.git",
                "/src/c9x.me//qbe",
            ),
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
                "https://example.com/example_repo.git",
                "/test/~/example_repo",
            ),
            // Escaping braces
            (
                "/test/{{owner}}/{repo}",
                "https://example.com/example_repo.git",
                "/test/{owner}/example_repo",
            ),
        ]
        .iter()
        .for_each(|(pattern, url, expected)| {
            assert_eq!(
                clone_path(
                    &GrabPattern((*pattern).into()),
                    None,
                    &parse_url(url).unwrap()
                )
                .unwrap(),
                PathBuf::from(expected)
            )
        });
    }

    #[test]
    fn test_clone_path_invalid() {
        [
            (
                "{unknown}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
            ),
            (
                "{Host}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
            ),
            (
                "{HOST}/",
                "https://github.com/influxdata/influxdb2-sample-data.git",
            ),
        ]
        .iter()
        .for_each(|(pattern, url)| {
            let result = clone_path(
                &GrabPattern((*pattern).into()),
                None,
                &parse_url(url).unwrap(),
            );
            assert!(result.is_err());
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
                    &GrabPattern((*pattern).into()),
                    Some(&custom_home),
                    &parse_url(url).unwrap()
                )
                .unwrap(),
                PathBuf::from(expected)
            )
        });
    }
}
