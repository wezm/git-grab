<h1 align="center">
  ⤵️<br>
  Git Grab
</h1>

<div align="center">
  <strong>A small tool to clone git repositories to a standard location, organised
  by domain name and path. Runs on BSD, Linux, macOS, Windows, and
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

Grab clones the repo into `$GRAB_HOME/github.com/wezm/git-grab` where `GRAB_HOME`
defaults to `~/src` if not set or supplied via the `--home` argument. For
example:

    $ git grab github.com/wezm/git-grab
    Grab https://github.com/wezm/git-grab to /home/wmoore/src/github.com/wezm/git-grab
    Cloning into '/home/wmoore/src/github.com/wezm/git-grab'...
    remote: Enumerating objects: 30, done.
    remote: Counting objects: 100% (30/30), done.
    remote: Compressing objects: 100% (20/20), done.
    remote: Total 30 (delta 9), reused 27 (delta 7), pack-reused 0
    Receiving objects: 100% (30/30), 12.50 KiB | 12.50 MiB/s, done.
    Resolving deltas: 100% (9/9), done.

    $ exa --tree ~/src
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

Download
--------

Pre-compiled binaries are available for a number of platforms.

* [FreeBSD 13 amd64](https://releases.wezm.net/git-grab/0.1.2/git-grab-0.1.2-amd64-unknown-freebsd.tar.gz)
* [Linux x86\_64](https://releases.wezm.net/git-grab/0.1.2/git-grab-0.1.2-x86_64-unknown-linux-musl.tar.gz)
* [MacOS x86\_64](https://releases.wezm.net/git-grab/0.1.2/git-grab-0.1.2-x86_64-apple-darwin.tar.gz)
* [Windows x86\_64](https://releases.wezm.net/git-grab/0.1.2/git-grab-0.1.2-x86_64-pc-windows-msvc.zip)

Example to download and extract a binary:

    curl https://releases.wezm.net/git-grab/0.1.2/git-grab-0.1.2-x86_64-unknown-linux-musl.tar.gz | tar zxf -

Usage
-----

```
USAGE:
    git grab [OPTIONS] [URL]...

ARGS:
    <URL>...
        One or more git URLs to clone. Any URL accepted by `git` is valid.
        In addition, URLs without a scheme such as
        github.com/wezm/git-grab are also accepted.

OPTIONS:
    -h, --help
            Prints help information

        --home [default: ~/src]
            The directory to use as "grab home", where the URLs will be
            cloned into. Overrides the GRAB_HOME environment variable if
            set.

    -n, --dry-run
            Don't clone the repository but print what would be done.

    -V, --version
            Prints version information

ENVIRONMENT
    GRAB_HOME
        See --home
```

Build from Source
-----------------

**Minimum Supported Rust Version:** 1.51.0

`git-grab` is implemented in Rust. See the Rust website for [instructions on
installing the toolchain][rustup].

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

Licence
-------

This project is dual licenced under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/wezm/git-grab/blob/master/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/wezm/git-grab/blob/master/LICENSE-MIT))

at your option.

[rustup]: https://www.rust-lang.org/tools/install
