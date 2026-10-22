# ShallGraph

Follow links in requirement files named `*.req.yml` or `*.req.yaml` (for example `satisfies: GRD-SYS-001`) via click or Go to Definition.

The extension activates when you open YAML in a workspace whose root contains `shallgraph.yaml` or `shallgraph.yml`. Link navigation, Go to Definition, schema validation under `requirements/`, and the rendered view apply to `*.req.yml` and `*.req.yaml` files. Requirement YAML validation uses the YAML extension: on activation, ShallGraph registers the JSON Schema from core and refreshes it when `shallgraph.yaml` / `shallgraph.yml` or workspace folders change.

## Rendered requirement view

Opening a `*.req.yml` or `*.req.yaml` file shows the YAML editor. A file may contain one requirement or several requirements separated by a line that contains only `---`. With that file active, switch to the rendered HTML view with the status bar **ShallGraph** control, the editor title **Show Rendered View** action (preview icon), Command Palette **ShallGraph: Show Rendered View**, or `Ctrl+K V` (`Cmd+K V` on macOS). **Show YAML Source** (same controls) switches that tab back to the YAML editor. In Cursor, editor title icons may sit under the editor `…` menu; the status bar control stays visible. Links and Go to Definition resolve a requirement id to the file that contains it, including when that file's name is not the id.

The rendered view uses the same HTML as the exported requirements report. Edits belong in the YAML file. When that YAML changes while the rendered view is showing, the view refreshes. A requirement link in the rendered view opens the target requirement file as YAML. When the git origin remote is GitHub, file paths in the view are the same GitHub links as in the exported report.
