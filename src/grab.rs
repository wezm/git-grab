use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{fs, io};
use url::{ParseError, Url};

use crate::Error;

const HTTPS: &str = "https://";

pub fn grab(home: &Path, url: OsString) -> Result<(), Error> {
    let str = url.to_str().ok_or_else(|| String::from("invalid url"))?;
    let url: Url = str.parse().or_else(|err| match err {
        ParseError::RelativeUrlWithoutBase => {
            if looks_like_ssh_url(str) {
                // Might be an ssh style URL like git@github.com:wezm/grab.git
                let newstr =
                    normalise_ssh_url(str).ok_or_else(|| format!("unable to normalise '{}'", str))?;
                newstr.parse().map_err(Error::from)
            } else if str.contains('.') {
                // might be a URL without a scheme like github.com/wezm/grab
                let mut newstr = String::with_capacity(HTTPS.len() + str.len());
                newstr.push_str(HTTPS);
                newstr.push_str(str);
                newstr.parse().map_err(Error::from)
            }
            else {
                Err(format!("'{}': {}", str, err).into())
            }
        }
        _ => Err(format!("'{}': {}", str, err).into())
    })?;

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
    url.split_once(':')
        .map(|(before, _after)| before.contains('@'))
        .unwrap_or(false)
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

    /*
    https://github.com/influxdata/influxdb2-sample-data.git
    https://github.com/influxdata/influxdb2-sample-data
    https://github.com/nushell/nushell
    github.com/zesterer/tao
    github.com/mdg/leema
    github.com/alec-deason/wasm_plugin
    github.com/bytecodealliance/wasmtime
    github.com/denoland/deno/
    github.com/denoland/deno
    git@github.com:wezm/grab.git
    */
}
