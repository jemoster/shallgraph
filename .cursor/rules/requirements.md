# Requirements
Requirements for the project are stored in the `requirements` directory.

Ignore all requirements under the sample_projects directory.  These are test data for the project.

Code comments shall reference the requirement ID that they fullfill.

## Refinement claims

`refinement` is either a Markdown string or a YAML list of claims.

String refinements are the older standard and remain valid. Leave them as strings unless the user asks to upgrade them. Dashes inside a `|` block are text, not sub-claims.

A list is an outline. Each item is a string, or a mapping with `text` and an optional nested `refinement`. Do not add `id` or `title` on a claim. `require` stays one sentence with one shall, should, or may. Claim text does not need an RFC2119 keyword.

Position assigns the citation: `(a)`, then `(1)`, then `(A)`, then `(i)`, then `(I)`, then the series repeats. The first claim is cited as `{id}(a)`. A nested list uses the next series, as in `{id}(b)(1)`.

See `.cursor/skills/require/SKILL.md` for the upgrade steps and examples.