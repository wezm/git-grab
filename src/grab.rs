use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{fs, io};
use url::{ParseError, Url};

use crate::Error;

const HTTPS: &str = "https://";

pub fn grab(home: &Path, url: OsString) -> Result<(), Error> {
    let str = url.to_str().ok_or_else(|| "invalid url")?;
    let url: Url = parse_url(str)?;

    let dest_path = clone_path(home, &url)?;
    println!("Grab {} to {}", url, dest_path.display());
    fs::create_dir_all(&dest_path)?;
    let status = clone(&url, &dest_path)?;
    status
        .success()
        .then(|| ())
        .ok_or_else(|| match status.code() {
            Some(code) => format!("git exited with status {}", code),
            None => String::from("git killed by signal"),
        })
        .map_err(|err| err.into())
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

fn clone(url: &Url, dest_path: &Path) -> Result<ExitStatus, io::Error> {
    // TODO: Support other version control systems
    Command::new("git")
        .arg("clone")
        .arg(url.as_str())
        .arg(dest_path)
        .status()
}

fn clone_path(home: &Path, url: &Url) -> Result<PathBuf, Error> {
    let mut path = home.to_path_buf();
    path.push(url.host_str().ok_or_else(|| "invalid hostname")?);
    url.path_segments()
        .ok_or_else(|| "missing path in url")?
        .for_each(|seg| path.push(seg));

    // Strip trailing .git from clone path
    if path.extension() == Some(OsStr::new("git")) {
        path.set_extension("");
    }

    Ok(path)
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
    fn test_clone_path() {
        let home = Path::new("/src");
        [
            (
                "https://github.com/influxdata/influxdb2-sample-data.git",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            (
                "https://github.com/influxdata/influxdb2-sample-data",
                "/src/github.com/influxdata/influxdb2-sample-data",
            ),
            ("github.com/zesterer/tao", "/src/github.com/zesterer/tao"),
            (
                "github.com/denoland/deno/",
                "/src/github.com/denoland/deno/",
            ),
            (
                "git@github.com:wezm/git-grab.git",
                "/src/github.com/wezm/git-grab",
            ),
            ("git.sr.ht/~wezm/lobsters", "/src/git.sr.ht/~wezm/lobsters"),
            (
                "git@git.sr.ht:~wezm/lobsters",
                "/src/git.sr.ht/~wezm/lobsters",
            ),
            (
                "bitbucket.org/egrange/dwscript",
                "/src/bitbucket.org/egrange/dwscript",
            ),
            ("git://c9x.me/qbe.git", "/src/c9x.me/qbe"),
        ]
        .iter()
        .for_each(|(url, expected)| {
            assert_eq!(
                clone_path(home, &parse_url(url).unwrap()).unwrap(),
                PathBuf::from(expected)
            )
        });
    }
}
