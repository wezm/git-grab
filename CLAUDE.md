# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

git-grab is a Rust CLI tool that clones Git repositories to a standardized directory structure organized by domain name and path (e.g., `~/src/github.com/user/repo`). It supports multiple URL formats and includes cross-platform clipboard integration.

## Essential Commands

### Build and Development
```bash
# Build for development
cargo build

# Build optimized release
cargo build --release --locked

# Run tests
cargo test

# Run tests without clipboard feature
cargo test --no-default-features

# Install locally
cargo install --path .
```

### Testing
```bash
# Run all tests with verbose output
cargo test --verbose

# Run specific test module
cargo test grab::tests
cargo test clipboard::providers::tests
```

## Architecture Overview

The codebase is organized into focused modules:

1. **main.rs**: Entry point with simple error handling and execution flow
2. **args.rs**: CLI argument parsing using pico-args (lightweight alternative to clap)
3. **grab.rs**: Core logic for URL parsing, path construction, and Git operations
4. **clipboard module**: Cross-platform clipboard abstraction with platform-specific providers

### Key Design Patterns

- **URL Normalization**: The `grab` module handles various URL formats (HTTPS, SSH, domain-only) and normalizes them for consistent repository paths
- **Platform Abstraction**: Clipboard functionality uses conditional compilation and runtime detection to support macOS, Linux, Windows, and WSL
- **Feature Flags**: Clipboard support is optional and can be disabled with `--no-default-features`

### Platform-Specific Considerations

- **macOS**: Uses pbcopy/pbpaste for clipboard
- **Linux**: Auto-detects xclip, xsel, or Wayland clipboard tools
- **Windows**: Uses native Windows clipboard API
- **WSL**: Bridges to Windows clipboard via PowerShell

## CI/CD Pipeline

The project uses Cirrus CI (.cirrus.yml) for multi-platform builds:
- Builds for Debian Linux (musl), FreeBSD, macOS (universal binary), and Windows
- Automatically publishes release artifacts to S3
- Runs tests on all platforms before building releases