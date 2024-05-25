mod args;
#[cfg(feature = "clipboard")]
mod clipboard;
mod grab;

use std::ffi::OsString;
use std::{io, process};

use crate::args::parse_args;
use crate::grab::grab;

type Error = Box<dyn std::error::Error>;

fn main() {
    match try_main() {
        Ok(0) => {}
        Ok(status) => process::exit(status),
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn try_main() -> Result<i32, Error> {
    let config = match parse_args()? {
        Some(config) => config,
        None => return Ok(0),
    };
    if let Some(unrecognised) = config
        .grab_urls
        .iter()
        .find(|url| url.to_str().map_or(false, |s| s.starts_with("--")))
    {
        eprintln!("Unrecognised option: {}", unrecognised.to_string_lossy());
        let _ = args::print_usage();
        return Ok(2);
    }

    let clipboard_url = if config.clipboard {
        match clipboard_url() {
            Ok(clipboard) if clipboard.is_some() => clipboard,
            Ok(_) => {
                eprintln!("Error: no URL on clipboard");
                return Ok(1);
            }
            Err(err) => {
                eprintln!("Error: failed to paste from clipboard: {}", err);
                return Ok(1);
            }
        }
    } else {
        None
    };

    if config.grab_urls.is_empty() && clipboard_url.is_none() {
        let _ = args::print_usage();
        return Ok(2);
    }

    let mut success = true;
    for url in config
        .grab_urls
        .into_iter()
        .chain(clipboard_url.into_iter())
    {
        match grab(&config.home, url, config.dry_run, &config.git_args) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Error: {}", err);
                success = false;
            }
        }
    }

    Ok(if success { 0 } else { 1 })
}

#[cfg(feature = "clipboard")]
fn clipboard_url() -> io::Result<Option<OsString>> {
    clipboard::provider().and_then(|cb| cb.paste()).map(|s| {
        let arg = s.trim();
        if !arg.is_empty() {
            Some(OsString::from(arg))
        } else {
            None
        }
    })
}

#[cfg(not(feature = "clipboard"))]
fn clipboard_url() -> io::Result<Option<OsString>> {
    Ok(None)
}
