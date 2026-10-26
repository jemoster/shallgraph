# Requirements
Requirements for the project are stored in the `requirements` directory.

Code comments shall reference the requirement ID that they fulfill.

## Refinement claims

`refinement` is either a Markdown string or a YAML list of claims. Both forms are equally valid.

A string is Markdown prose. Dashes inside a `|` block are text, and they have no citations.

A list is an outline. Each item is a string, or a mapping with `text` and an optional nested `refinement`. A claim has no `id` or `title`. `require` stays one sentence with one shall, should, or may. Claim text does not need an RFC2119 keyword.

Position assigns the citation: `(a)`, then `(1)`, then `(A)`, then `(i)`, then `(I)`, then the series repeats. The first claim is cited as `{id}(a)`. A nested list uses the next series, as in `{id}(b)(1)`.

Rewrite a refinement into the other form only when the user asks:

1. Leave `id`, `title`, and `require` in place.
2. For a list, turn each point into one list item. Use a plain string when it has no children, and `text` plus a nested `refinement` list for sub-claims.
3. For a string, write the same points as Markdown prose.
4. Leave claims without ids or titles.
5. Leave a fragment without shall, should, or may. That keyword rule applies to `require` only.
