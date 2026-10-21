/**
 * WASM facade smoke tests: init bindings, parse, validate, schema, and single-requirement HTML.
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import {
  exportRequirementFileJsonSchema,
  formatRequirementToYaml,
  generateSingleRequirementHtml,
  loadRequirements,
  readRequirementSchemaComposeOptions,
  loadWasmBindings,
  initShallgraphWasm,
  parseRequirementContent,
  parseRequirementDocuments,
  validateRequirements,
} from "@shallgraph/core";

const REPO_ROOT = path.resolve(__dirname, "../../..");
const SAMPLE = path.join(REPO_ROOT, "sample_projects", "basic");
const FIXTURE_PATH = path.join(SAMPLE, "system", "SYS-001.req.yml");

describe("shallgraph WASM core facade", () => {
  beforeAll(async () => {
    await initShallgraphWasm();
    loadWasmBindings();
  });

  it("parses requirement YAML and returns the id", () => {
    const yaml = `id: GRD-WASM-001
title: WASM parse fixture
require: The facade shall parse this requirement.
refinement: Supporting detail.
`;
    const result = parseRequirementContent(yaml, "GRD-WASM-001.req.yml");
    expect("error" in result).toBe(false);
    if ("error" in result) return;
    expect(result.requirement.id).toBe("GRD-WASM-001");
    expect(result.requirement.title).toBe("WASM parse fixture");
    expect(result.requirement.require).toContain("shall parse");
  });

  it("validates a well-formed requirement with no errors", () => {
    const parsed = parseRequirementContent(
      `id: GRD-WASM-002
title: Valid
require: The system shall be valid.
`,
      "/tmp/GRD-WASM-002.req.yml"
    );
    expect("error" in parsed).toBe(false);
    if ("error" in parsed) return;
    expect(validateRequirements([parsed.requirement])).toEqual([]);
  });

  it("exports JSON Schema keys used by the VS Code YAML language service", () => {
    const schema = exportRequirementFileJsonSchema() as {
      type?: string;
      required?: string[];
      properties?: Record<string, unknown>;
    };
    expect(schema.type).toBe("object");
    expect(schema.required).toEqual(expect.arrayContaining(["id", "title", "require"]));
    expect(schema.properties).toBeDefined();
    expect(schema.properties!.id).toBeDefined();
    expect(schema.properties!.require).toBeDefined();
    expect(schema.properties!.refinement).toBeDefined();
    expect(schema.properties!.satisfied_by).toBeDefined();
  });

  it("formats a requirement to YAML that includes the id", () => {
    const parsed = parseRequirementContent(
      `id: GRD-WASM-003
title: Format me
require: The formatter shall emit YAML.
`,
      "/tmp/GRD-WASM-003.req.yml"
    );
    expect("error" in parsed).toBe(false);
    if ("error" in parsed) return;
    const yaml = formatRequirementToYaml(parsed.requirement);
    expect(yaml).toContain("id: GRD-WASM-003");
    expect(yaml).toContain("require:");
  });

  it("generates single-requirement HTML containing the id and optional field markers", () => {
    const parsed = parseRequirementContent(
      `id: GRD-WASM-004
title: HTML fixture
require: The HTML renderer shall include this id.
refinement: Extra prose.
attributes:
  rationale: Why this exists.
`,
      "/tmp/GRD-WASM-004.req.yml"
    );
    expect("error" in parsed).toBe(false);
    if ("error" in parsed) return;
    const html = generateSingleRequirementHtml(parsed.requirement, [parsed.requirement], {
      editableFieldMarkers: true,
    });
    expect(html).toContain("GRD-WASM-004");
    expect(html).toContain('class="require"');
    expect(html).toContain('data-shallgraph-field="require"');
    expect(html).toContain('data-shallgraph-field="refinement"');
    expect(html).toContain('data-shallgraph-field="rationale"');
  });

  it("turns file paths into GitHub blob links when artifactLinks.github is set", () => {
    const parsed = parseRequirementContent(
      `id: GRD-WASM-005
title: GitHub links
require: The HTML renderer shall emit GitHub blob file links.
satisfied_by:
  - artifact: src/feature.ts
  - artifact: https://example.com/evidence
`,
      "/workspace/requirements/GRD-WASM-005.req.yml"
    );
    expect("error" in parsed).toBe(false);
    if ("error" in parsed) return;
    const html = generateSingleRequirementHtml(parsed.requirement, [parsed.requirement], {
      artifactLinks: {
        github: {
          owner: "acme",
          repo: "widgets",
          commitSha: "deadbeef",
          projectRootRel: "",
          projectRoot: "/workspace",
        },
      },
    });
    expect(html).toContain("GRD-WASM-005");
    expect(html).toContain(
      'href="https://github.com/acme/widgets/blob/deadbeef/src/feature.ts"'
    );
    expect(html).toContain('href="https://example.com/evidence"');
    expect(html).toContain(
      'href="https://github.com/acme/widgets/blob/deadbeef/requirements/GRD-WASM-005.req.yml"'
    );
  });

  it("loads requirements from the basic sample project via the Node fs adapter", async () => {
    const result = await loadRequirements(SAMPLE);
    expect(result.errors).toEqual([]);
    expect(result.requirements.length).toBeGreaterThan(0);
    expect(result.requirements.some((r) => r.id === "SYS-001")).toBe(true);
    expect(result.requirements.some((r) => r.sourcePath.includes("SYS-001"))).toBe(true);
  });

  it("loads every document from a multi-document file and skips the filename match", async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-multidoc-"));
    fs.mkdirSync(path.join(root, "requirements"));
    fs.writeFileSync(path.join(root, "shallgraph.yaml"), "requirement_dirs:\n  - requirements\n");
    const body = `id: SYS-001
title: End-to-End Encryption
require: The system shall encrypt all video streams.
---
id: SYS-002
title: Multi-Stream Support
require: The backend shall support concurrent streaming.
`;
    fs.writeFileSync(path.join(root, "requirements", "bundle.req.yml"), body);
    const parsed = parseRequirementDocuments(body, path.join(root, "requirements", "bundle.req.yml"));
    expect(parsed.errors).toEqual([]);
    expect(parsed.requirements.map((r) => r.id)).toEqual(["SYS-001", "SYS-002"]);
    expect(parsed.requirements.every((r) => r.multiDocument)).toBe(true);

    const loaded = await loadRequirements(root, root);
    expect(loaded.errors).toEqual([]);
    expect(loaded.requirements.map((r) => r.id)).toEqual(["SYS-001", "SYS-002"]);

    const mismatch = validateRequirements([
      {
        ...parsed.requirements[0]!,
        multiDocument: false,
      },
    ]);
    expect(mismatch.some((err) => err.message.includes("does not match filename"))).toBe(true);
  });

  it("parses a sample requirement file from disk", () => {
    const yaml = fs.readFileSync(FIXTURE_PATH, "utf-8");
    const result = parseRequirementContent(yaml, FIXTURE_PATH);
    expect("error" in result).toBe(false);
    if ("error" in result) return;
    expect(result.requirement.id).toBe("SYS-001");
  });

  it("enforces custom tags from the project marker and schema", async () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-tags-"));
    fs.mkdirSync(path.join(root, "requirements"));
    fs.writeFileSync(
      path.join(root, "shallgraph.yaml"),
      "requirement_dirs:\n  - requirements\ntags:\n  priority:\n    values: [low, high]\n    required: true\n"
    );
    fs.writeFileSync(
      path.join(root, "requirements", "GRD-TAG-001.req.yml"),
      "id: GRD-TAG-001\ntitle: Tagged\nrequire: The system shall honor tags.\ntags:\n  priority: medium\n"
    );
    const loaded = await loadRequirements(root, root);
    expect(loaded.errors.some((err) => err.message.includes('tag "priority"'))).toBe(true);
    expect(loaded.errors.some((err) => err.path.endsWith("GRD-TAG-001.req.yml"))).toBe(true);

    const compose = readRequirementSchemaComposeOptions(root);
    const schema = exportRequirementFileJsonSchema(compose) as {
      required?: string[];
      properties?: { tags?: { properties?: { priority?: { enum?: string[] } } } };
    };
    expect(schema.properties?.tags?.properties?.priority?.enum).toEqual(["low", "high"]);
    expect(schema.required).toEqual(expect.arrayContaining(["tags"]));
  });

  it("loads wasm from the package dist when process.cwd is not the repo root", () => {
    const distIndex = path.join(REPO_ROOT, "packages", "core", "dist", "index.js");
    expect(fs.existsSync(distIndex)).toBe(true);
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-wasm-cwd-"));
    const spec = pathToFileURL(distIndex).href;
    const script = `
      import { parseRequirementContent } from ${JSON.stringify(spec)};
      const r = parseRequirementContent(
        "id: GRD-WASM-CWD\\ntitle: Cwd\\nrequire: The loader shall not use process.cwd().\\n",
        "GRD-WASM-CWD.req.yml"
      );
      if ("error" in r) throw new Error(r.error.message);
      if (r.requirement.id !== "GRD-WASM-CWD") throw new Error(r.requirement.id);
    `;
    execFileSync(process.execPath, ["--input-type=module", "-e", script], {
      cwd: tmp,
      encoding: "utf-8",
    });
  });

  it("loads the CommonJS package export and its WASM bindings", () => {
    const cjsIndex = path.join(REPO_ROOT, "packages", "core", "dist-cjs", "index.js");
    expect(fs.existsSync(cjsIndex)).toBe(true);
    const script = `
      const core = require("@shallgraph/core");
      const result = core.parseRequirementContent(
        "id: GRD-CJS-001\\ntitle: CommonJS\\nrequire: The CommonJS export shall load WASM bindings.\\n",
        "GRD-CJS-001.req.yml"
      );
      if ("error" in result) throw new Error(result.error.message);
      if (result.requirement.id !== "GRD-CJS-001") throw new Error(result.requirement.id);
      process.stdout.write(require.resolve("@shallgraph/core"));
    `;
    const resolved = execFileSync(
      process.execPath,
      ["--no-experimental-require-module", "-e", script],
      { cwd: REPO_ROOT, encoding: "utf-8" }
    ).trim();
    expect(resolved).toBe(cjsIndex);
  });
});
