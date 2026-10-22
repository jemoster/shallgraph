/**
 * GRD-VSC-003: Same-tab rendered view — contribution tests.
 * Opening a requirement file stays on the YAML text editor (`priority: option`).
 * Editor title actions switch that tab to the rendered view and back.
 *
 * The rendered view uses core's generateSingleRequirementHtml (same as the full report),
 * so description and rationale are rendered as Markdown per GRD-HTML-004.
 */
import * as path from "node:path";
import * as fs from "node:fs";
import { exportRequirementFileJsonSchema } from "@shallgraph/core";

const packagePath = path.join(__dirname, "..", "package.json");
const pkg = JSON.parse(fs.readFileSync(packagePath, "utf-8")) as {
  scripts?: { build?: string };
  dependencies?: Record<string, string>;
  contributes?: {
    commands?: Array<{ command: string; title: string; category?: string; icon?: string }>;
    menus?: Record<string, Array<{ command: string; when?: string; group?: string }>>;
    customEditors?: Array<{
      viewType: string;
      displayName?: string;
      priority?: string;
      selector?: Array<{ filenamePattern?: string }>;
    }>;
  };
};

describe("GRD-VSC-003 same-tab rendered view contributions", () => {
  it("keeps the YAML text editor as the default for requirement files", () => {
    const editors = pkg.contributes?.customEditors ?? [];
    const rendered = editors.find((e) => e.viewType === "shallgraph.requirementRendered");
    expect(rendered).toBeDefined();
    expect(rendered!.priority).toBe("option");
    const patterns = (rendered!.selector ?? []).map((s) => s.filenamePattern);
    expect(patterns).toEqual(expect.arrayContaining(["*.req.yml", "*.req.yaml"]));
  });

  it("contributes a show-rendered command for the YAML editor title", () => {
    const commands = pkg.contributes?.commands ?? [];
    const showRendered = commands.find((c) => c.command === "shallgraph.requirement.showRendered");
    expect(showRendered).toBeDefined();
    expect(showRendered!.title).toContain("Rendered");
    expect(showRendered!.category).toBe("ShallGraph");
    expect(showRendered!.icon).toBe("$(open-preview)");
    expect(commands.some((c) => c.command === "shallgraph.requirement.openPreview")).toBe(false);

    const editorTitle = pkg.contributes?.menus?.["editor/title"] ?? [];
    const entry = editorTitle.find((m) => m.command === "shallgraph.requirement.showRendered");
    expect(entry).toBeDefined();
    // Filename-based context key (not editorLangId / regex) so the action still
    // contributes when the host hides title icons or language id is not yaml.
    expect(entry!.when).toBe("shallgraph.requirementSourceActive");
  });

  it("contributes a show-source command that returns the same tab to YAML", () => {
    const commands = pkg.contributes?.commands ?? [];
    const showSource = commands.find((c) => c.command === "shallgraph.requirement.showSource");
    expect(showSource).toBeDefined();
    expect(showSource!.title).toContain("YAML");
    expect(showSource!.category).toBe("ShallGraph");

    const editorTitle = pkg.contributes?.menus?.["editor/title"] ?? [];
    const entry = editorTitle.find((m) => m.command === "shallgraph.requirement.showSource");
    expect(entry).toBeDefined();
    expect(entry!.when).toBe("shallgraph.requirementRenderedActive");
  });

  it("contributes keybindings for the same-tab YAML ↔ rendered switch", () => {
    const keybindings = (
      pkg.contributes as { keybindings?: Array<{ command: string; when?: string; key?: string }> }
    )?.keybindings ?? [];
    const showRendered = keybindings.find((k) => k.command === "shallgraph.requirement.showRendered");
    const showSource = keybindings.find((k) => k.command === "shallgraph.requirement.showSource");
    expect(showRendered?.when).toBe("shallgraph.requirementSourceActive");
    expect(showSource?.when).toBe("shallgraph.requirementRenderedActive");
    expect(showRendered?.key).toBe("ctrl+k v");
    expect(showSource?.key).toBe("ctrl+k v");
  });

  it("does not bundle a rich-text editor for the rendered view", () => {
    expect(pkg.dependencies?.["@toast-ui/editor"]).toBeUndefined();
    expect(pkg.scripts?.build ?? "").not.toContain("webview");
  });
});

describe("GRD-VSC-004 YAML schema for requirement files", () => {
  it("does not ship static yamlValidation; schema is applied at runtime via yaml.schemas", () => {
    expect((pkg.contributes as Record<string, unknown> | undefined)?.yamlValidation).toBeUndefined();
  });

  it("exported JSON Schema describes requirement structure (id, title, description, attributes, links)", () => {
    const schema = exportRequirementFileJsonSchema() as {
      type?: string;
      required?: string[];
      properties?: Record<string, unknown>;
    };
    expect(schema.type).toBe("object");
    expect(schema.required).toContain("id");
    expect(schema.required).toContain("title");
    expect(schema.properties).toBeDefined();
    expect(schema.properties!.id).toBeDefined();
    expect(schema.properties!.title).toBeDefined();
    expect(schema.properties!.require).toBeDefined();
    expect(schema.properties!.refinement).toBeDefined();
    expect(schema.properties!.attributes).toBeDefined();
    expect(schema.properties!.links).toBeDefined();
    expect(schema.properties!.satisfied_by).toBeDefined();
    expect(schema.properties!.verified_by).toBeDefined();
    expect(schema.properties!.parameters).toBeDefined();
  });
});

describe("GRD-VSC-005 Add new requirement from explorer context menu", () => {
  it("contributes shallgraph.requirement.new command with ShallGraph category", () => {
    const commands = pkg.contributes?.commands ?? [];
    const newReq = commands.find((c) => c.command === "shallgraph.requirement.new");
    expect(newReq).toBeDefined();
    expect(newReq!.title).toContain("Add New Requirement");
    expect(newReq!.category).toBe("ShallGraph");
  });

  it("contributes explorer/context menu for the new requirement command", () => {
    const explorerContext = pkg.contributes?.menus?.["explorer/context"] ?? [];
    const newReqEntry = explorerContext.find(
      (m) => m.command === "shallgraph.requirement.new"
    );
    expect(newReqEntry).toBeDefined();
    expect(newReqEntry!.group).toBe("ShallGraph");
  });

  it("newRequirementYamlTemplate produces YAML matching requirement schema (id, title, require, refinement, attributes, links)", async () => {
    const { newRequirementYamlTemplate } = await import(
      "../src/new-requirement-template.js"
    );
    const id = "GRD-TEST-001";
    const yaml = newRequirementYamlTemplate(id);
    expect(yaml).toContain(`id: ${id}`);
    expect(yaml).toContain("title:");
    expect(yaml).toContain("require:");
    expect(yaml).toContain("refinement:");
    expect(yaml).toContain("attributes:");
    expect(yaml).toContain("links:");
    const { parse } = await import("yaml");
    const parsed = parse(yaml) as Record<string, unknown>;
    expect(parsed).toBeDefined();
    expect(parsed.id).toBe(id);
    expect(typeof parsed.title).toBe("string");
    expect("require" in parsed).toBe(true);
    expect(parsed.attributes).toBeDefined();
    expect(Array.isArray(parsed.links)).toBe(true);
  });
});
