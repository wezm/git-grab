use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use pico_args::Arguments;

use crate::Error;

#[cfg(feature = "clipboard")]
const SUPPORTS_CLIPBOARD: bool = true;
#[cfg(not(feature = "clipboard"))]
const SUPPORTS_CLIPBOARD: bool = false;

pub struct Config {
    pub dry_run: bool,
    /// Path to source home
    pub home: PathBuf,
    /// Paste the URL to clone from the clipboard
    pub clipboard: bool,
    /// Copy the local destination path to clipboard after cloning
    pub copy_path: bool,
    pub grab_urls: Vec<OsString>,
    /// Extra arguments to pass to git
    pub git_args: Vec<OsString>,
}

pub fn parse_args() -> Result<Option<Config>, Error> {
    let mut args: Vec<_> = env::args_os().skip(1).collect();

    // Handle '--'.
    let (args, git_args) = if let Some(dash_dash) = args.iter().position(|arg| arg == "--") {
        let git_args = args.split_off(dash_dash + 1);
        args.pop(); // Drop '--'
        (args, git_args)
    } else {
        (args, Vec::new())
    };

    let mut pargs = Arguments::from_vec(args);
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
        .ok_or("unable to determine home directory")?;
    let clipboard = pargs.contains(["-c", "--clipboard"]);
    let copy_path = pargs.contains(["-p", "--copy-path"]) || env::var_os("GRAB_COPY_PATH").is_some();

    if (clipboard || copy_path) && !SUPPORTS_CLIPBOARD {
        return Err("this git-grab was not built with clipboard support.")?;
    }

    Ok(Some(Config {
        dry_run,
        home,
        clipboard,
        copy_path,
        grab_urls: pargs.finish(),
        git_args,
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
    let clipboard = if SUPPORTS_CLIPBOARD {
        "\n    -c, --clipboard\n            Paste a URL to clone from the clipboard.\n"
    } else {
        ""
    };
    let copy_path = if SUPPORTS_CLIPBOARD {
        "\n    -p, --copy-path\n            Copy the local destination path to clipboard after cloning.\n"
    } else {
        ""
    };

    println!(
        "{}

Clone a git repository into a standard location organised by domain and path.

E.g. https://github.com/wezm/git-grab.git would be cloned to:

    $GRAB_HOME/github.com/wezm/git-grab

USAGE:
    git grab [OPTIONS] [URL]... [--] [GIT OPTIONS]

ARGS:
    <URL>...
        One or more git URLs to clone. Any URL accepted by `git` is valid.
        In addition, URLs without a scheme such as
        github.com/wezm/grab are also accepted.

OPTIONS:
    -h, --help
            Prints help information
{clipboard}{copy_path}
        --home [default: ~/src or $GRAB_HOME]
            The directory to use as \"grab home\", where the URLs will be
            cloned into. Overrides the GRAB_HOME environment variable if
            set.

    -n, --dry-run
            Don't clone the repository but print what would be done.

    -V, --version
            Prints version information

GIT OPTIONS:
    Arguments after `--` will be passed to the git clone invocation.
    This can be used supply arguments like `--recurse-submodules`.

ENVIRONMENT
    GRAB_HOME
        See --home
    
    GRAB_COPY_PATH
        If set, copy the local destination path to clipboard after cloning
        (equivalent to --copy-path)

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
