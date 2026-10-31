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

/**
 * GRD-VSC-003: Requirement id from a report link.
 * Hash links (`#GRD-SYS-001`, including a citation suffix) and relative page links
 * (`GRD-SYS-001.html`). Absolute http(s) links stay ordinary links.
 *
 * Nested helpers stay inside this function so the rendered view can install the
 * same function in the webview via `toString()`.
 */
export function requirementIdFromLinkHref(href: string): string | undefined {
  function idFromFragment(fragment: string): string | undefined {
    let raw: string;
    try {
      raw = decodeURIComponent(fragment).trim();
    } catch {
      return undefined;
    }
    const paren = raw.indexOf("(");
    if (paren >= 0) raw = raw.slice(0, paren).trim();
    if (!/^[A-Za-z0-9][A-Za-z0-9-]*$/.test(raw)) return undefined;
    return raw;
  }
  function idFromHtmlFilename(pathPart: string): string | undefined {
    const noQuery = pathPart.split("?")[0] || "";
    let decoded: string;
    try {
      decoded = decodeURIComponent(noQuery);
    } catch {
      return undefined;
    }
    const base = decoded.split("/").pop() || "";
    const match = /^([A-Za-z0-9][A-Za-z0-9]*-[A-Za-z0-9-]+)\.html$/i.exec(base);
    return match ? match[1] : undefined;
  }
  const trimmed = href.trim();
  if (!trimmed || /^[a-z][a-z0-9+.-]*:/i.test(trimmed)) return undefined;
  if (trimmed.charAt(0) === "#") return idFromFragment(trimmed.slice(1));
  const hashAt = trimmed.indexOf("#");
  const pathPart = hashAt >= 0 ? trimmed.slice(0, hashAt) : trimmed;
  const fromFile = idFromHtmlFilename(pathPart);
  if (fromFile) return fromFile;
  if (hashAt >= 0) return idFromFragment(trimmed.slice(hashAt + 1));
  return undefined;
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
  const requirementIdFromLinkHref = ${requirementIdFromLinkHref.toString()};
  document.addEventListener("click", function(e) {
    var node = e.target;
    var el = node && node.nodeType === 1 ? node : node && node.parentElement;
    if (!el || !el.closest) return;
    var a = el.closest("a[href]");
    if (!a) return;
    var href = a.getAttribute("href");
    if (!href) return;
    var id = requirementIdFromLinkHref(href);
    if (!id) return;
    e.preventDefault();
    e.stopPropagation();
    vscode.postMessage({ type: "openRequirement", requirementId: id });
  }, true);
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
