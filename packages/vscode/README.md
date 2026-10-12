# ShallGraph

Follow links in requirement files named `*.req.yml` or `*.req.yaml` (for example `satisfies: GRD-SYS-001`) via click or Go to Definition.

The extension activates when you open YAML in a workspace whose root contains `shallgraph.yaml` or `shallgraph.yml`. Link navigation, Go to Definition, schema validation under `requirements/`, and the rendered view apply to `*.req.yml` and `*.req.yaml` files. Requirement YAML validation uses the YAML extension: on activation, ShallGraph registers the JSON Schema from core and refreshes it when `shallgraph.yaml` / `shallgraph.yml` or workspace folders change.

## Rendered requirement view

Opening a `*.req.yml` or `*.req.yaml` file shows the YAML editor. With that file active, use the editor title **Show Rendered View** action (preview icon) to replace that tab with an HTML rendering of the requirement. **Show YAML Source** switches the same tab back to the YAML editor.

The rendered view uses the same HTML as the exported requirements report. Edits belong in the YAML file. When that YAML changes while the rendered view is showing, the view refreshes. A requirement link in the rendered view opens the target requirement file as YAML. When the git origin remote is GitHub, file paths in the view are the same GitHub links as in the exported report.
