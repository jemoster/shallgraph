/**
 * GRD-VSC-003 / GRD-VSC-006: The in-tab rendered document is report HTML plus a
 * requirement-link handler. It does not include a field editor.
 */
import { buildRenderedErrorHtml, buildRenderedWebviewHtml, requirementIdFromHashHref } from "../src/rendered-html";

const options = { cspSource: "https://example.vscode-cdn.net", nonce: "test-nonce" };

describe("GRD-VSC-003 rendered webview HTML", () => {
  it("reads a requirement id from a report hash link", () => {
    expect(requirementIdFromHashHref("#GRD-SYS-001")).toBe("GRD-SYS-001");
    expect(requirementIdFromHashHref("#GRD%2DVSC-003")).toBe("GRD-VSC-003");
    expect(requirementIdFromHashHref("https://github.com/acme/repo")).toBeUndefined();
    expect(requirementIdFromHashHref("#")).toBeUndefined();
  });

  it("keeps the report HTML and posts requirement-link clicks", () => {
    const html = buildRenderedWebviewHtml(
      `<!DOCTYPE html><html><head></head><body><a href="#GRD-SYS-001">GRD-SYS-001</a></body></html>`,
      options
    );
    expect(html).toContain('href="#GRD-SYS-001"');
    expect(html).toContain('nonce="test-nonce"');
    expect(html).toContain('type: "openRequirement"');
    expect(html).toContain("requirementId: href.slice(1)");
  });
});

describe("GRD-VSC-006 read-only rendered view", () => {
  it("does not add a field editor to the report HTML", () => {
    const html = buildRenderedWebviewHtml(
      `<!DOCTYPE html><html><head></head><body><section class="requirement-detail"><div class="require">The system shall stay put.</div></section></body></html>`,
      options
    );
    expect(html).not.toContain("toastui");
    expect(html).not.toContain("contenteditable");
    expect(html).not.toContain("data-shallgraph-field");
    expect(html).not.toContain("fieldEdit");
    expect(html).not.toContain("<textarea");
  });

  it("renders parse errors as text without an editor", () => {
    const html = buildRenderedErrorHtml(`bad <yaml> & "quotes"`, options);
    expect(html).toContain("bad &lt;yaml&gt; &amp; &quot;quotes&quot;");
    expect(html).not.toContain("<textarea");
    expect(html).not.toContain("fieldEdit");
  });
});
