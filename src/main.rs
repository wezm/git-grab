mod args;
mod grab;

use std::process;

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
    if config.grab_urls.is_empty() {
        let _ = args::print_usage();
        return Ok(2);
    }
    if let Some(unrecognised) = config
        .grab_urls
        .iter()
        .find(|url| url.to_str().map_or(false, |s| s.starts_with("--")))
    {
        eprintln!("Unrecognised option: {}", unrecognised.to_string_lossy());
        let _ = args::print_usage();
        return Ok(2);
    }

    let mut success = true;
    for url in config.grab_urls {
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
