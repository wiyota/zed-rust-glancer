# zed-rust-glancer

[rust-glancer](https://rust-glancer.github.io/) support for
[Zed](https://zed.dev) — a lightweight Rust language server that trades
incompleteness for performance and low memory usage.

The extension downloads a prebuilt `rust-glancer` binary from the
[latest GitHub release](https://github.com/rust-glancer/rust-glancer/releases)
(VSIX asset for your platform) into the extension's working directory and runs
it as an LSP server over stdio. Supported platforms: macOS (arm64/x86_64) and
Linux (arm64/x86_64). On other platforms, build the server yourself and point
Zed at it via settings (see below).

## Requirements

rust-glancer needs the Rust standard library sources to analyze projects:

```sh
rustup component add rust-src
```

## Installation (as a dev extension)

1. Clone this repository.
2. In Zed, run `extensions: install dev extension` from the command palette
   and select this directory.

## Usage

By default Zed runs both rust-analyzer (built-in) and rust-glancer for Rust
files. It is recommended to disable rust-analyzer while using rust-glancer:

```json
"languages": {
  "Rust": {
    "language_servers": ["!rust-analyzer", "rust-glancer"]
  }
}
```

## Configuration

### Use a local binary

If you built `rust-glancer` yourself (`cargo build --release -p rust-glancer`
in a clone of the upstream repository):

```json
"lsp": {
  "rust-glancer": {
    "binary": {
      "path": "/path/to/rust-glancer/target/release/rust-glancer",
      "arguments": ["lsp"]
    }
  }
}
```

Extra environment variables for the server can be set with
`lsp.rust-glancer.binary.env`.

### Server options

Server behavior is configured through `initialization_options` (the same
sections VS Code exposes as its settings). For example, to enable diagnostics
(off by default, see the [upstream configuration docs](https://rust-glancer.github.io/docs/usage/CONFIGURE.html)):

```json
"lsp": {
  "rust-glancer": {
    "initialization_options": {
      "diagnostics": {
        "onStartup": true,
        "onSave": true
      },
      "cargo": {
        "features": ["serde"],
        "overrides": [
          {
            "path": "firmware",
            "target": "riscv32imac-unknown-none-elf",
            "noDefaultFeatures": true,
            "features": ["board-v1"]
          }
        ]
      },
      "cfg": {
        "test": true,
        "atoms": []
      },
      "indexing": {
        "performancePreference": "lower-peak-memory"
      },
      "cache": {
        "packageResidency": "all-offloadable"
      }
    }
  }
}
```

Other accepted keys: `diagnostics.command` (default `check`),
`diagnostics.cargoArguments` (default `["--workspace"]`),
`diagnostics.extraEnv`, `cargo.target`, `cargo.allFeatures`,
`cargo.noDefaultFeatures`, and `sysroot.discovery`
(`"auto" | "disabled"`).

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.

To contribute to this extension:

1. Clone the repository
2. Make your changes
3. Test with Zed
4. Submit a pull request

## License

This project is licensed under either of [Apache License, Version 2.0](./LICENSE-APACHE) or [MIT License](./LICENSE-MIT) at your option.
