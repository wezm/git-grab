use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use pico_args::Arguments;

use crate::Error;

#[cfg(feature = "clipboard")]
const SUPPORTS_CLIPBOARD: bool = true;
#[cfg(not(feature = "clipboard"))]
const SUPPORTS_CLIPBOARD: bool = false;

pub struct GrabPattern(pub String);

impl Default for GrabPattern {
    fn default() -> Self {
        Self("~/src/{host/}{path/}".into())
    }
}

pub struct Config {
    pub dry_run: bool,
    /// Pattern to use for destination paths
    pub pattern: GrabPattern,
    /// Home directory to use for grabs
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
            env::var_os("GRAB_HOME")
                .map(PathBuf::from)
                .or_else(home::home_dir)
        })
        .ok_or("unable to determine home directory")?;
    let pattern = pargs
        .opt_value_from_os_str("--pattern", |s| {
            Ok::<Option<String>, &'static str>(s.to_str().map(|s| s.to_string()))
        })?
        .flatten()
        .or_else(|| {
            env::var_os("GRAB_PATTERN")
                .and_then(|s| s.to_str().map(|s| s.to_string()))
                .or_else(|| Some("~/src/{host/}{owner/}{repo}".into()))
        })
        .map(GrabPattern)
        .ok_or("unable to determine grab pattern")?;
    let clipboard = pargs.contains(["-c", "--clipboard"]);
    let copy_path =
        pargs.contains(["-p", "--copy-path"]) || env::var_os("GRAB_COPY_PATH").is_some();

    if (clipboard || copy_path) && !SUPPORTS_CLIPBOARD {
        return Err("this git-grab was not built with clipboard support.")?;
    }

    Ok(Some(Config {
        dry_run,
        home,
        pattern,
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

    ~/src/github.com/wezm/git-grab

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
        --pattern <PATTERN> [default: ~/src/{{host/}}{{path/}} or $GRAB_PATTERN]
            Destination path pattern for grabbed repositories with optional
            placeholders.

            Placeholders are enclosed in curly braces `{{}}`.
            Optionally, they may have leading and trailing `/` characters.
            If the placeholder is present, the slashes will be added to the
            path, otherwise they will be omitted.
            Placeholders can be escaped by doubling the curly braces, e.g.
            `{{{{owner}}}}` will render as `{{owner}}`.

            The tilde `~` at the start of the pattern is expanded to the
            home directory.

            The following placeholders are supported:
            - host  - the host part of the URL, e.g. github.com
            - path  - the path part of the URL, e.g. /wezm/git-grab
            - owner - the owner or organisation of the repo for supported urls, e.g. wezm
            - repo  - the repository name for supported urls, e.g. git-grab
            - home  - the user's home directory (can be overwritten by --home or $GRAB_HOME)

            Placeholders are case-sensitive, e.g. `{{Repo}}` or `{{REPO}}` is not valid.

        --home (deprecated) [default: $GRAB_HOME]
            Overrides the value that the ~ character or {{home}} placeholder
            will be expanded to, when evaluating the pattern.

    -n, --dry-run
            Don't clone the repository but print what would be done.

    -V, --version
            Prints version information

GIT OPTIONS:
    Arguments after `--` will be passed to the git clone invocation.
    This can be used to supply arguments like `--recurse-submodules`.

ENVIRONMENT
    GRAB_PATTERN
        See --pattern

    GRAB_COPY_PATH
        If set, copy the local destination path to clipboard after cloning
        (equivalent to --copy-path)

    GRAB_HOME (deprecated)
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
