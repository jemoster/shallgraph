# Rust tracing sample

A small Celsius/Fahrenheit library that shows how shallgraph collects **implementation** and **verification** links from Rust source and prints them in the HTML and Markdown reports.

Requirements live in `requirements/`. Implementation is tagged with `#[shallgraph::implements("…")]`. Tests that assert those requirements are tagged with `#[shallgraph::verifies("…")]`.

## Latest shallgraph release

This sample is meant to be used with the latest `shallgraph` CLI from [GitHub Releases](https://github.com/jemoster/shallgraph/releases/latest). On Linux x86_64:

```bash
curl -L -o shallgraph "https://github.com/jemoster/shallgraph/releases/latest/download/shallgraph-linux-x86_64"
chmod +x shallgraph
```

Place `shallgraph` on your `PATH`, or invoke it with `./shallgraph` from the download directory.

From this repository you can also run the same CLI from source:

```bash
cargo run -p shallgraph -- html --project-dir sample_projects/rust --output ./out
cargo run -p shallgraph -- markdown --project-dir sample_projects/rust --output ./out
```

The `shallgraph-macros` crate is a path dependency on this workspace (same macros the latest release collects). In your own project, pin the macros crate to that release:

```toml
shallgraph-macros = { git = "https://github.com/jemoster/shallgraph", tag = "v0.5.0" }
```

## Run the tests

From the repository root:

```bash
cargo test -p tempconv
```

Or from this directory:

```bash
cargo test
```

## Generate the HTML or Markdown report

From this directory, after `shallgraph` is on `PATH`:

```bash
shallgraph html --output ./out
shallgraph markdown --output ./out
```

Open `out/index.html` or `out/index.md`. Each requirement includes:

- **Satisfied by**
  - **By comment** — YAML `satisfied_by` artifact paths
  - **Rust** — `#[shallgraph::implements]` on `celsius_to_fahrenheit` / `fahrenheit_to_celsius`
- **Verified by**
  - **By comment** — YAML `verified_by` artifact paths
  - **Rust** — `#[shallgraph::verifies]` on the unit tests (including the round-trip test that lists both IDs)
