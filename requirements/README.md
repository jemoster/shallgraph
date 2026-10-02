# shallgraph product requirements

Product requirements for the shallgraph requirement management tool are stored here.

## Numbering

- **GRD-SYS-*** – System / platform (core engine, discovery, schema).
- **GRD-CLI-*** – CLI behavior (validate, HTML export, Markdown export).
- **GRD-HTML-*** – HTML report content and presentation.
- **GRD-MD-*** – Markdown report content and presentation.
- **GRD-VSC-*** – VSCode plugin (IDE integration, link following).
- **GRD-DEVOPS-*** – DevOps / CI (GitHub Actions, tests on PRs).

Add new categories as needed (e.g. GRD-WEB-*) when you add formal requirements for other consumers.

## Adding requirements

- One YAML file per requirement.
- File name should match the requirement id (e.g. `GRD-SYS-001.req.yml` or `GRD-SYS-001.req.yaml`).
- Use the same schema as project requirements: `id`, `title`, `require`, `refinement`, `attributes`, `links`, `satisfied_by`, `verified_by`.
- In `links`, use `satisfies: <id>` to reference other requirements (e.g. `satisfies: GRD-SYS-001`).
