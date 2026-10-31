/**
 * GRD-VSC-002: Link targets in requirement YAML, and where a requirement sits in its file.
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { clearRequirementPathCache, resolveLinkTarget } from "../src/link-resolver";
import { findRequirementLinkTargets, requirementLineInFile } from "../src/requirement-links";

describe("GRD-VSC-002 requirement link targets", () => {
  it("finds satisfies and other link keys, excluding quotes", () => {
    const text = [
      "links:",
      "  - satisfies: GRD-SYS-001",
      '  - refines: "GRD-SYS-002"',
      "  - derived_from: 'GRD-SYS-003'",
      "",
    ].join("\n");
    const targets = findRequirementLinkTargets(text);
    expect(targets.map((target) => target.value)).toEqual([
      "GRD-SYS-001",
      "GRD-SYS-002",
      "GRD-SYS-003",
    ]);
    for (const target of targets) {
      expect(text.slice(target.start, target.end)).toBe(target.value);
    }
  });

  it("finds link targets in each document and in flow style", () => {
    const text = [
      "id: GRD-A-001",
      "links: [{satisfies: GRD-SYS-001, refines: GRD-SYS-002}]",
      "---",
      "id: GRD-B-001",
      "links:",
      "  - see: notes/evidence.md",
      "",
    ].join("\n");
    expect(findRequirementLinkTargets(text).map((target) => target.value)).toEqual([
      "GRD-SYS-001",
      "GRD-SYS-002",
      "notes/evidence.md",
    ]);
  });

  it("ignores requirement ids that are not link targets", () => {
    const text = [
      "id: GRD-A-001",
      "require: See GRD-SYS-009.",
      "refinement: |",
      "  satisfies: GRD-SYS-008",
      "links:",
      "  - satisfies: GRD-SYS-001",
      "",
    ].join("\n");
    expect(findRequirementLinkTargets(text).map((target) => target.value)).toEqual(["GRD-SYS-001"]);
  });

  it("still finds satisfies when the document does not parse", () => {
    const text = ["links:", "  - satisfies: GRD-SYS-001", "  - refines: [", ""].join("\n");
    expect(findRequirementLinkTargets(text).map((target) => target.value)).toContain("GRD-SYS-001");
  });

  it("locates a requirement id inside a multi-document file", () => {
    const text = "id: CAM-001\n---\nid: CAM-002\n";
    expect(requirementLineInFile(text, "CAM-002")).toBe(2);
    expect(requirementLineInFile("id: 'GRD-SYS-001'\n", "GRD-SYS-001")).toBe(0);
    expect(requirementLineInFile("title: none\n", "GRD-SYS-001")).toBe(0);
  });
});

describe("GRD-VSC-002 resolve a link to a file", () => {
  afterEach(() => {
    clearRequirementPathCache();
  });

  it("opens an http link target as a url", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-links-"));
    await expect(resolveLinkTarget(dir, "https://example.com/evidence", dir)).resolves.toEqual({
      kind: "url",
      url: "https://example.com/evidence",
    });
  });

  it("opens a relative file path", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-links-"));
    const notes = path.join(dir, "notes.txt");
    fs.writeFileSync(notes, "hi");
    await expect(resolveLinkTarget(dir, "notes.txt", dir)).resolves.toEqual({
      kind: "file",
      path: notes,
    });
  });

  it("opens the requirement file for an id, including when the filename is not the id", async () => {
    const project = path.resolve(__dirname, "../../../sample_projects/basic");
    const streaming = path.join(project, "system", "streaming.req.yml");
    await expect(resolveLinkTarget(path.join(project, "system"), "CAM-001", project)).resolves.toEqual({
      kind: "requirement",
      path: streaming,
      requirementId: "CAM-001",
    });
    await expect(resolveLinkTarget(project, "CAM-002", project)).resolves.toEqual({
      kind: "requirement",
      path: streaming,
      requirementId: "CAM-002",
    });
  });

  it("returns null when the target is neither a requirement nor a file", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "shallgraph-links-"));
    await expect(resolveLinkTarget(dir, "GRD-NOPE-001", dir)).resolves.toBeNull();
  });
});
