/**
 * GRD-VSC-003 / GRD-VSC-006: Read-only webview document around the shared report HTML.
 * The document does not include an editor for requirement fields.
 */

const RENDERED_THEME_STYLES = `
body { background-color: var(--vscode-editor-background); color: var(--vscode-editor-foreground); }
.requirement-detail .label { color: var(--vscode-foreground); }
.requirement-detail h2 { color: var(--vscode-foreground); }
.meta, .source { color: var(--vscode-descriptionForeground); }
.param-value { background: var(--vscode-textCodeBlock-background); color: var(--vscode-textPreformat-foreground); }
.parameters-table th { background: var(--vscode-editorWidget-background); color: var(--vscode-foreground); }
.parameters-table td, .parameters-table th { border-color: var(--vscode-panel-border); }
a { color: var(--vscode-textLink-foreground); }
`;

/** GRD-VSC-003: Hash links in the report HTML name a requirement id. */
export function requirementIdFromHashHref(href: string): string | undefined {
  if (!href.startsWith("#") || href.length < 2) return undefined;
  try {
    const id = decodeURIComponent(href.slice(1)).trim();
    return id.length > 0 ? id : undefined;
  } catch {
    return undefined;
  }
}

function contentSecurityPolicy(cspSource: string, nonce: string): string {
  return [
    `default-src 'none'`,
    `img-src ${cspSource} https: data:`,
    `style-src ${cspSource} 'unsafe-inline'`,
    `script-src 'nonce-${nonce}'`,
    `font-src ${cspSource}`,
  ].join("; ");
}

/** GRD-VSC-003: Clicks on requirement links ask the extension to open that file as YAML. */
function linkScript(nonce: string): string {
  return `<script nonce="${nonce}">
(function() {
  const vscode = acquireVsCodeApi();
  const previous = vscode.getState();
  if (previous && typeof previous.scrollTop === "number") {
    document.documentElement.scrollTop = previous.scrollTop;
  }
  document.addEventListener("scroll", function() {
    vscode.setState({ scrollTop: document.documentElement.scrollTop || document.body.scrollTop || 0 });
  }, true);
  document.body.addEventListener("click", function(e) {
    var t = e.target;
    if (!t || !t.closest) return;
    var a = t.closest('a[href^="#"]');
    if (!a) return;
    var href = a.getAttribute("href");
    if (!href || href.length < 2) return;
    e.preventDefault();
    e.stopPropagation();
    vscode.postMessage({ type: "openRequirement", requirementId: href.slice(1) });
  });
})();
</script>`;
}

function injectDocument(baseHtml: string, csp: string, themeStyles: string, bodyAppend: string): string {
  const cspMeta = `<meta http-equiv="Content-Security-Policy" content="${csp}">`;
  const theme = `<style>${themeStyles}</style>`;
  let html = baseHtml.includes("<head>")
    ? baseHtml.replace("<head>", `<head>${cspMeta}`)
    : `<!DOCTYPE html><html><head>${cspMeta}</head><body>${baseHtml}</body></html>`;
  // Theme rules follow the report stylesheet so editor colors apply inside the tab.
  if (html.includes("</head>")) {
    html = html.replace("</head>", `${theme}</head>`);
  } else {
    html = html.replace("<head>", `<head>${theme}`);
  }
  if (html.includes("</body>")) {
    html = html.replace("</body>", `${bodyAppend}</body>`);
  } else {
    html += bodyAppend;
  }
  return html;
}

/**
 * GRD-VSC-003 / GRD-VSC-006: Wrap report HTML for the in-tab rendered view.
 * Read-only: no field editors and no path that writes the requirement file.
 */
export function buildRenderedWebviewHtml(
  baseHtml: string,
  options: { cspSource: string; nonce: string }
): string {
  const { cspSource, nonce } = options;
  return injectDocument(
    baseHtml,
    contentSecurityPolicy(cspSource, nonce),
    RENDERED_THEME_STYLES,
    linkScript(nonce)
  );
}

/** Place every requirement document from one file into a single rendered document. */
export function combineRequirementHtml(parts: string[]): string {
  if (parts.length === 0) return "";
  const first = parts[0]!;
  if (parts.length === 1) return first;
  const rest = parts.slice(1).map((html) => {
    const match = html.match(/<body[^>]*>([\s\S]*)<\/body>/i);
    return match?.[1] ?? html;
  });
  const insertion = rest.join("\n");
  if (first.includes("</body>")) {
    return first.replace("</body>", `${insertion}</body>`);
  }
  return `${first}\n${insertion}`;
}

/** GRD-VSC-003: Shown when the current YAML cannot be rendered. */
export function buildRenderedErrorHtml(
  message: string,
  options: { cspSource: string; nonce: string }
): string {
  const escaped = message
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
  const body = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>ShallGraph rendered requirement</title>
</head>
<body>
  <h1>Could not render this requirement</h1>
  <pre>${escaped}</pre>
</body>
</html>`;
  return buildRenderedWebviewHtml(body, options);
}
