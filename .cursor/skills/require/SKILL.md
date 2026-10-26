---
name: require
description: >-
  Drafts or updates shallgraph requirement YAML under requirements/ without
  implementing code. Use when the user wants to add or refine requirements,
  requirement files, or a requirements-only workflow in a shallgraph project.
disable-model-invocation: true
---

# shallgraph — define requirements only

Do not implement the requirement. Only define the requirement.

## Steps

1. Analyze the given requirements.

   - Check for clarity — If the requirement is unclear or inconsistent, stop to ask the user clarifying questions and update the requirement. If changes are made, redo this step until there are no changes made.
   - Check for redundancy — If the requirement is redundant with another requirement, stop to ask the user if the requirement should be removed. If changes are made, redo this step until there are no changes made.

2. Define the requirement.

   - Create a new requirement or update an existing requirement file in the requirements directory. The requirement file should be named after the requirement ID.
   - The requirement file should be a YAML file and adhere to the requirement schema.

## YAML style

Each requirement file uses:

- `require` — one short normative sentence with exactly one of shall, should, or may (for this requirement ID). State the obligation so a reader can understand it at a glance. Do not list every feature, option, or constraint in `require`; put that detail in `refinement`.
- `refinement` — optional supporting detail. Use either the string form or a claim list, as described below.

## Refinement forms

A string and a claim list are equally valid. Choose the form that matches the detail. `require` stays one sentence with exactly one of shall, should, or may. Claim text does not follow that keyword rule.

### String

A string is Markdown prose. Use it for a paragraph or a note.

```yaml
refinement: |
  * If no project root is found, the CLI shall exit with an error.
```

Dashes, asterisks, and numbers inside the string are text. They are prose, and they have no citations.

### Claim list

A YAML list is an outline of claims, details, or assertions. The requirement still has one `id` and one `title`. A claim has no id and no title.

```yaml
refinement:
  - for the life of the account that created it.
  - text: until an operator deletes it.
    refinement:
      - The deletion shall be recorded in the audit log.
      - A deleted recording shall be removed from search.
  - in the encrypted store named by SYS-001.
```

A list item is a string, or a mapping with `text` and an optional nested `refinement`. The nested value may again be a string (a paragraph under that claim) or a list. A fragment continues the parent `require` or claim and starts in lowercase. A complete sentence stands on its own.

Position assigns the citation. The series is `(a)`, then `(1)`, then `(A)`, then `(i)`, then `(I)`, and then it repeats. For requirement `SYS-003`, the claims above are `SYS-003(a)`, `SYS-003(b)`, `SYS-003(b)(1)`, `SYS-003(b)(2)`, and `SYS-003(c)`.

## Changing between the two forms

Rewrite a refinement into the other form only when the user asks. A Markdown list inside a `|` block is prose. A YAML list is a cited outline. Changing one into the other changes citations and the review diff.

When the user asks for that change:

1. Leave `id`, `title`, and `require` in place. When `require` is a lead-in, keep the lead-in there.
2. For a list, turn each point into one list item. Use a plain string when the claim has no children. Use `text` plus a nested `refinement` list for sub-claims.
3. For a string, write the same points as Markdown prose.
4. Leave claims without ids, titles, or their own `require` field.
5. Leave a fragment without a shall, should, or may. That keyword rule applies to `require` only.

When writing requirements YAML, do not include newlines to limit the line length. Assume instead that word-wrap will be used to maintain readability.
