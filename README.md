<h1 align="center">
  ⤵️<br>
  Git Grab
</h1>

<div align="center">
  <strong>A small tool to clone git repositories to a standard location, organised
  by domain name and path. It runs on BSD, Linux, macOS, Windows, and
  more.</strong>
</div>

<br>

<div align="center">
  <a href="https://cirrus-ci.com/github/wezm/git-grab">
    <img src="https://api.cirrus-ci.com/github/wezm/git-grab.svg" alt="Build Status"></a>
  <a href="https://crates.io/crates/git-grab">
    <img src="https://img.shields.io/crates/v/git-grab.svg" alt="Version">
  </a>
  <img src="https://img.shields.io/crates/l/git-grab.svg" alt="License">
</div>

<br>

Git Grab clones a repo into `$GRAB_HOME`, organised by domain and path.
`GRAB_HOME` defaults to `~/src` if not set or supplied via the `--home`
argument. For example:

    $ git grab github.com/wezm/git-grab
    Cloning into '/home/wmoore/src/github.com/wezm/git-grab'...
    remote: Enumerating objects: 30, done.
    remote: Counting objects: 100% (30/30), done.
    remote: Compressing objects: 100% (20/20), done.
    remote: Total 30 (delta 9), reused 27 (delta 7), pack-reused 0
    Receiving objects: 100% (30/30), 12.50 KiB | 12.50 MiB/s, done.
    Resolving deltas: 100% (9/9), done.
    Grabbed https://github.com/wezm/git-grab to /home/wmoore/src/github.com/wezm/git-grab

    $ lsd --tree ~/src
    /home/wmoore/src
    └── github.com
       └── wezm
          └── git-grab
             ├── Cargo.lock
             ├── Cargo.toml
             └── src
                ├── args.rs
                ├── grab.rs
                └── main.rs

Install
-------

### Pre-compiled Binary

Pre-compiled binaries are available for a number of platforms:

* FreeBSD 13+ amd64
* Linux x86\_64
* MacOS Universal
* Windows x86\_64

Check the [latest release] for download links.

### Package Manager

`git-grab` is packaged in these package managers:

* Arch Linux: `git-grab`
* Brew: `brew install git-grab`
* Chimera Linux: `git-grab`

Usage
-----

Once `git-grab` in installed you can use it via `git grab`. `git` automatically
finds binaries named `git-*`, this also means that if you have a shell alias
like `alias g=git`, `g grab` will also work.

```
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

    -c, --clipboard
            Paste a URL to clone from the clipboard.

    -p, --copy-path
            Copy the local destination path to clipboard after cloning.

        --home [default: ~/src or $GRAB_HOME]
            The directory to use as "grab home", where the URLs will be
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
```

A man page is also available in the source distribution.

### With GitHub CLI

1. Configure an alias in the GitHub CLI:

       gh alias set --shell grab 'git grab "git@github.com:$1.git"'

2. You can now grab a GitHub repo. For example:

       gh grab wezm/git-grab

Build from Source
-----------------

**Minimum Supported Rust Version:** 1.82.0

`git-grab` is implemented in Rust. See the Rust website for [instructions on
installing the toolchain][rustup].

**Compile-time Options (Cargo Features)**

`git-grab` supports the following compile-time options:

* `clipboard`: enable support for cloning the URL on the clipboard
  * This feature is on by default
  * On UNIX and UNIX-like systems such as BSD and Linux one of the following
    tools must be installed:
    * [wl-clipboard](https://github.com/bugaevc/wl-clipboard) (Wayland)
    * [xclip](https://github.com/astrand/xclip) or
      [xsel](https://vergenet.net/~conrad/software/xsel/) (X11)

### From Git Checkout or Release Tarball

Build the binary with `cargo build --release --locked`. The binary will be in
`target/release/git-grab`.

### From crates.io

`cargo install git-grab`

Credits
-------

This tool is inspired by [grab by @jmhodges](https://github.com/jmhodges/grab).
A small comparison:

| Feature              | Original                               | This Version           |
|----------------------|----------------------------------------|------------------------|
| VCS Supported        | Git, Mercurial, Subversion, and Bazaar | Git                    |
| Dependencies         | None                                   | git                    |
| Progress Information | No                                     | Yes, provided by `git` |

`git-grab` incorporates clipboard code from [clipp] by [bendn] under the MIT licence.

Licence
-------

This project is dual licenced under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/wezm/git-grab/blob/master/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/wezm/git-grab/blob/master/LICENSE-MIT))

at your option.

[rustup]: https://www.rust-lang.org/tools/install
[clipp]: https://github.com/bend-n/clipp
[bendn]: https://github.com/bend-n
