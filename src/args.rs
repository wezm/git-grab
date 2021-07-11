use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use pico_args::Arguments;

use crate::Error;

pub struct Config {
    pub dry_run: bool,
    // Path to source home
    pub home: PathBuf,
    pub grab_urls: Vec<OsString>,
}

pub fn parse_args() -> Result<Option<Config>, Error> {
    let mut pargs = Arguments::from_env();
    if pargs.contains(["-V", "--version"]) {
        return print_version();
    } else if pargs.contains(["-h", "--help"]) {
        return print_usage();
    }

    let dry_run = pargs.contains(["-n", "--dry-run"]);
    let home = pargs
        .opt_value_from_os_str("--home", parse_path)?
        .or_else(|| {
            env::var_os("GRAB_HOME").map(PathBuf::from).or_else(|| {
                home::home_dir().map(|mut dir| {
                    dir.push("src");
                    dir
                })
            })
        })
        .ok_or_else(|| "unable to determine home directory")?;

    Ok(Some(Config {
        dry_run,
        home,
        grab_urls: pargs.finish(),
    }))
}

fn print_version() -> Result<Option<Config>, Error> {
    println!("{}", version_string());
    Ok(None)
}

fn version_string() -> String {
    format!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
}

pub fn print_usage() -> Result<Option<Config>, Error> {
    println!(
        "{}

Clone a git repository into a standard location organised by domain.

E.g. https://github.com/wezm/git-grab.git would be cloned to:

    $GRAB_HOME/github.com/wezm/grab

USAGE:
    git grab [OPTIONS] [URL]...

ARGS:
    <URL>...
        One or more git URLs to clone. Any URL accepted by `git` is valid.
        In addition, URLs without a scheme such as
        github.com/wezm/grab are also accepted.

OPTIONS:
    -h, --help
            Prints help information

        --home [default: ~/src]
            The directory to use as \"grab home\", where the URLs will be
            cloned into. Overrides the GRAB_HOME environment variable if
            set.

    -n, --dry-run
            Don't clone the repository but print what would be done.

    -V, --version
            Prints version information

ENVIRONMENT
    GRAB_HOME
        See --home

AUTHOR
    {}

SEE ALSO
    Project source code: https://github.com/wezm/git-grab ",
        version_string(),
        env!("CARGO_PKG_AUTHORS")
    );
    Ok(None)
}

fn parse_path(s: &OsStr) -> Result<PathBuf, &'static str> {
    Ok(s.into())
}
