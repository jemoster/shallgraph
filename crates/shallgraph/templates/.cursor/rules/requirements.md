# Requirements
Requirements for the project are stored in the `requirements` directory.

Code comments shall reference the requirement ID that they fulfill.

## Refinement claims

`refinement` is either a Markdown string or a YAML list of claims.

String refinements are the older standard and remain valid. Leave them as strings unless the user asks to upgrade them. Dashes inside a `|` block are text, not sub-claims.

A list is an outline. Each item is a string, or a mapping with `text` and an optional nested `refinement`. Do not add `id` or `title` on a claim. `require` stays one sentence with one shall, should, or may. Claim text does not need an RFC2119 keyword.

Position assigns the citation: `(a)`, then `(1)`, then `(A)`, then `(i)`, then `(I)`, then the series repeats. The first claim is cited as `{id}(a)`. A nested list uses the next series, as in `{id}(b)(1)`.

When asked to upgrade a string refinement:

1. Leave `id`, `title`, and `require` in place.
2. Turn each point into one list item. Use a plain string when it has no children.
3. Use `text` plus a nested `refinement` list only for sub-claims.
4. Do not invent ids or titles on claims.
5. Do not add shall, should, or may to a fragment. That keyword rule applies to `require` only.
