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
- `shallgraph -v` or `shallgraph --version` - print the semantic version and exit.

Use `shallgraph --help` or `shallgraph <command> --help` for full options.

## Requirement File Basics

Each requirement file ends in `.req.yml` or `.req.yaml`. A file holds one requirement, or several requirements separated by a line that contains only `---`. A `---` before the first requirement is allowed. A `---` after the last requirement is ignored.

`sample_projects/basic/system/streaming.req.yml` holds two requirements in one file. The filename is `streaming.req.yml`, which matches neither id.

Required fields:

- `id` - stable identifier. When the file holds one requirement, `id` matches the filename without the extension. When the file holds several requirements, the filename does not have to match any of those ids.
- `title` - short requirement name.

Required normative field:

- `require` - single Shall, Should, or May statement for this requirement ID.

Common optional fields:

- `refinement` - supporting detail. A string is Markdown prose. A list is an ordered outline of claims.
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

### Refinement claims

A string `refinement` is prose. Dashes and asterisks inside that string are text. Leave an existing string refinement as a string.

A list `refinement` is an outline. The requirement keeps its `id`, `title`, and `require` lead-in. Each claim is a string, or a mapping with `text` and an optional nested `refinement`. Claims have no id and no title. Position assigns the citation: `(a)`, then `(1)`, then `(A)`, then `(i)`, then `(I)`, and then the series repeats. The first claim of `SYS-003` is cited as `SYS-003(a)`.

```yaml
id: SYS-003
title: Recording retention
require: The system shall retain each recording,
refinement:
  - for the life of the account that created it.
  - text: until an operator deletes it.
    refinement:
      - The deletion shall be recorded in the audit log.
      - A deleted recording shall be removed from search.
  - in the encrypted store named by SYS-001.
```

`SYS-003(a)` continues the lead-in. `SYS-003(b)(1)` is "The deletion shall be recorded in the audit log." A fragment starts in lowercase and continues its parent. A complete sentence stands on its own. Reports number the list. A string refinement is still one Markdown block.

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

### `tags` in `shallgraph.yaml`

Optional. Defines custom tags that validation checks on every requirement file. Omit `tags`, or set it to `{}`, when the project does not use custom tags.

Each tag name maps to a definition:

- `values` lists the allowed strings. Matching is exact and case-sensitive.
- `required`, when true, means every requirement file includes that tag.
- `multiple`, when true, means the requirement value is a list. Otherwise it is one string.
- `allow_null`, on a single-value tag, allows YAML null.
- `allow_empty`, on a multi-value tag, allows an empty list.
- `allow_null_entry`, on a multi-value tag, allows a null item.
- `max_entries`, on a multi-value tag, is an upper bound on the number of items, including null items.

On a requirement file, `tags` is a separate mapping from `attributes`:

```yaml
tags:
  priority:
    values: [low, medium, high]
    required: true
  owner:
    values: [unassigned, alice, bob]
    allow_null: true
  labels:
    values: [safety, security, performance]
    multiple: true
    allow_null_entry: true
    max_entries: 3
```

```yaml
tags:
  priority: high
  owner: null
  labels: [safety, null, performance]
```

Validation reports a bad tag definition against the project root marker and the tag name. It reports a bad requirement entry against that requirement file and the tag name. The effective requirement schema uses the same limits.

`sample_projects/basic` is a worked example: `priority` is required, `component` may be null, and `concern` is a list that may be empty or contain a null item.

## Developer Documentation

See `dev.md` for workspace layout, building from source, packaging, the editor extension, and pre-commit setup.
