# shallgraph

A requirements management CLI for repositories that store requirements as YAML files.

[![Tests](https://github.com/jemoster/shallgraph/actions/workflows/test.yml/badge.svg)](https://github.com/jemoster/shallgraph/actions/workflows/test.yml)

## Installation

On Linux x86_64, download `shallgraph-linux-x86_64` from the GitHub Releases page, mark it executable, and place it on your `PATH`. Unreleased branch builds attach the same binary as a Tests workflow artifact.

To build from source:

```bash
cargo install --path crates/shallgraph
```

See `dev.md` for developer builds.

## Quick Start

From your repository root:

```bash
shallgraph bootstrap
```

Run commands from the project root (where `shallgraph.yaml` or `shallgraph.yml` exists), or pass `--project-dir /path/to/project`.

## Cursor Workflow

Use Cursor [Agent Skills](https://cursor.com/docs/context/skills) in `.cursor/skills/` to move from requirement text to implementation:

- `/require` - draft or update requirement files in `requirements/`.
- `/implement` - generate code that satisfies selected requirements.

## Core Commands

- `shallgraph bootstrap` - initialize `shallgraph.yaml` and `requirements/`.
- `shallgraph validate` - check requirement YAML against the active schema.
- `shallgraph format` - rewrite requirement YAML into canonical format.
- `shallgraph html` - generate a static HTML report. When `origin` is GitHub, file paths link to the blob at `HEAD`.
- `shallgraph markdown` - generate a static Markdown report. When `origin` is GitHub, file paths link to the blob at `HEAD`.
- `shallgraph schema` - print or export the effective requirement schema.

Use `shallgraph --help` or `shallgraph <command> --help` for full options.

## Requirement File Basics

Each requirement is one file ending in `.req.yml` or `.req.yaml`.

Required fields:

- `id` - stable identifier, usually matching the filename (without suffix).
- `title` - short requirement name.

Required normative field:

- `require` - single Shall, Should, or May statement for this requirement ID.

Common optional fields:

- `refinement` - supporting detail (Markdown supported in reports).
- `attributes` - metadata such as status, owner, or rationale.
- `links` - traceability links (for example `satisfies: OTHER-ID`).
- `satisfied_by` - artifacts (file paths or URLs) that implement or satisfy the requirement, each with an optional description.
- `verified_by` - artifacts (file paths or URLs) that verify the requirement was met, each with an optional description.
- `parameters` - reusable named values for interpolation in string fields.

Example artifact entry:

```yaml
satisfied_by:
  - artifact: src/feature.ts
    description: Primary implementation.
verified_by:
  - artifact: test/feature.test.ts
```

Parameter placeholders in strings:

- `{{ :name }}` for local requirement parameters.
- `{{ other_id:name }}` for cross-requirement parameters.
- `{{ "fixed text" }}` for a quoted literal.

## Tracing Rust source

Tag implementation and test items so shallgraph can collect source links. Add the `shallgraph-macros` crate and alias it as `shallgraph`, then attach the attributes:

```rust
extern crate shallgraph_macros as shallgraph;

#[shallgraph::implements("REQ-001")]
pub fn render_report() {}

#[shallgraph::verifies("REQ-001")]
#[test]
fn report_contains_index() {}
```

`implements` marks implementation; `verifies` marks verification. Each attribute accepts one or more requirement ID strings. `#[shallgraph_macros::implements]` / `#[shallgraph_macros::verifies]` are equivalent without the alias.

The HTML and Markdown reports list matching source links on each requirement under the same headings as YAML artifacts:

- **Satisfied by** — YAML `satisfied_by` under **By comment**, `implements` tags under **Rust**
- **Verified by** — YAML `verified_by` under **By comment**, `verifies` tags under **Rust**

Each Rust entry shows the file path, the kind of language item, and the line range (for example `L10–L12`). A heading or origin group is omitted when that requirement has no matching items.

See `sample_projects/rust` for a complete example.

## Optional Configuration

### `requirement_dirs` in `shallgraph.yaml`

Bootstrap creates a `requirements/` folder and lists it under `requirement_dirs`. Each entry is a path relative to the project root; the tool collects every `*.req.yml` and `*.req.yaml` file under those directories (recursively), except under `node_modules`. Use a single entry of `.` to include the whole project tree from the root down.

### `profile` in `shallgraph.yaml`

Choose how requirement YAML is interpreted and rendered. If omitted, `standard` is used.

## Developer Documentation

See `dev.md` for workspace layout, building from source, packaging, the editor extension, and pre-commit setup.
