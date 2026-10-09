# shallgraph Developer Guide

This page collects developer-focused documentation that was moved out of `README.md`.

## Current Components

- **CLI** - Discover requirements, validate schema, format requirement YAML, and generate static HTML and Markdown reports. Run from a project root (`shallgraph.yaml` / `shallgraph.yml`) or pass `--project-dir`. The Rust `shallgraph` binary provides bootstrap, validate, format, html, markdown, and schema. `shallgraph -v` and `shallgraph --version` print the semantic version and exit.
- **VS Code extension** - Navigate `satisfies` links, use Go to Definition on requirement IDs, preview requirements, and scaffold new requirements. The extension calls `@shallgraph/core`, a JS facade over the `shallgraph-core` WASM module.
- **Pre-commit hook** - Optional hook script under `scripts/` to run `shallgraph validate` on commit.

## Planned Consumers

Future integrations are tracked here until formal requirements are added:

- **GitHub CI** - Publish a GitHub Action or reusable workflow for requirement validation in pull requests.

## Project Layout

Workspace TypeScript packages:

- `packages/core` - `@shallgraph/core`: WASM JS facade (parse, validate, format, HTML) plus a thin Node filesystem adapter for desktop VS Code.
- `packages/vscode` - `shallgraph-vscode`: extension source and packaging.

Rust crates:

- `crates/shallgraph-core` - core engine (discovery, parse, validate, format, full and single-requirement HTML, full Markdown report, schema export, source-link collection). Single-requirement HTML and the Markdown report can optionally turn implementation and verification file paths into GitHub blob links when repository owner, name, commit, and project-root path are provided.
- `crates/shallgraph-wasm` - wasm-bindgen JSON ABI over `shallgraph-core` for JavaScript hosts.
- `crates/shallgraph-macros` - proc-macro attributes for tagging Rust items as source links.
- `crates/shallgraph` - `shallgraph` binary with bootstrap, validate, format, html, markdown, and schema.

Shared product data:

- `requirements/` - product requirements for shallgraph itself.
- `sample_projects/` - test data and examples (`basic` YAML tree; `rust` crate with `implements` / `verifies` source links).

## Build From Source

CLI (bootstrap, validate, format, html, markdown, schema):

```bash
cargo build --workspace
cargo test --workspace
cargo run -p shallgraph -- validate --project-dir sample_projects/basic
cargo run -p shallgraph -- html --project-dir sample_projects/rust --output ./out
cargo run -p shallgraph -- markdown --project-dir sample_projects/rust --output ./out
```

Install a local Rust binary onto PATH:

```bash
cargo install --path crates/shallgraph
```

Workspace npm scripts `validate`, `format`, `html`, `markdown`, and `bootstrap` invoke that same binary via `cargo run -p shallgraph`.

VS Code extension and WASM core facade:

The WASM module needs `wasm32-unknown-unknown` and `wasm-bindgen-cli` 0.2.100:

```bash
rustup target add wasm32-unknown-unknown
./scripts/install-wasm-bindgen.sh
npm install
npm run build
```

`npm run build` compiles `shallgraph-wasm` into `packages/core/wasm/` (Node, gitignored) and `packages/core/wasm-web/` (browser, gitignored), then copies the Node `.wasm` next to the VS Code extension bundle.

Browser and other bundler consumers should import `@shallgraph/core/html` (and `@shallgraph/core/types`) rather than the package root. The root entry includes a Node filesystem adapter (`glob`, `node:fs`). In the browser, await `initShallgraphWasm()` once before calling `generateSingleRequirementHtml`; the renderer stays synchronous after that.

## Distribution

### CLI and core package

Build the `@shallgraph/core` npm tarball locally:

```bash
./scripts/package.sh
```

That writes `release/shallgraph-core-X.Y.Z.tgz` (the WASM JS facade used by the VS Code extension).

Build the Linux x86_64 native CLI binary locally:

```bash
./scripts/package-native-cli.sh
```

The script writes `release/shallgraph-linux-x86_64`. Copy it onto `PATH` (for example `/usr/local/bin/shallgraph`). Publishing a GitHub Release uploads this asset via the CLI release workflow. Pull request CI also uploads that binary as a downloadable Actions artifact.

Install a local CLI from source with `./scripts/install-shallgraph.sh` (`cargo install --path crates/shallgraph`).

For GitHub-based releases, branch artifact download, and how to dry-run packaging, use `release.md`.

### Extension

Publishing a GitHub Release packages the extension, uploads a `.vsix` to the release, and publishes that VSIX to the Visual Studio Marketplace and to Open VSX. Cursor installs third-party extensions from Open VSX.

Package a local VSIX from the repo root:

```bash
npm run build
npm run package -w shallgraph-vscode
```

The package bundles `@shallgraph/core` and the wasm glue, so the VSIX is self-contained. The file is `packages/vscode/shallgraph-vscode-X.Y.Z.vsix`.

## Advanced CLI Usage

Specify a project explicitly:

```bash
shallgraph validate --project-dir /path/to/project
shallgraph format --project-dir /path/to/project
shallgraph html --project-dir /path/to/project --output ./out
shallgraph markdown --project-dir /path/to/project --output ./out
shallgraph schema --project-dir /path/to/project --format yaml -o ./schema.yaml
```

Schema export:

```bash
shallgraph schema
shallgraph schema --format json-schema --output ./requirement.schema.json
```

## Pre-commit Hook

Install hook into a target repo:

```bash
./scripts/install-pre-commit.sh [REPO_DIR]
```

- `REPO_DIR` is optional; if omitted, the current git repo is used.
- The hook uses git root as project root unless `SHALLGRAPH_PROJECT_DIR` is set.
- Ensure `shallgraph` is on `PATH` (for example via `cargo install --path crates/shallgraph` or a copied `shallgraph-linux-x86_64` binary).
