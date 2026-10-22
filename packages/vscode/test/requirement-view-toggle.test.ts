/**
 * GRD-VSC-003: Active-view detection for the YAML ↔ rendered toggle.
 */
import {
  CONTEXT_REQUIREMENT_RENDERED_ACTIVE,
  CONTEXT_REQUIREMENT_SOURCE_ACTIVE,
  RENDERED_VIEW_TYPE,
  classifyRequirementViewKind,
} from "../src/requirement-view-kind";

describe("GRD-VSC-003 requirement view toggle context keys", () => {
  it("exports stable context key names used by package.json when clauses", () => {
    expect(CONTEXT_REQUIREMENT_SOURCE_ACTIVE).toBe("shallgraph.requirementSourceActive");
    expect(CONTEXT_REQUIREMENT_RENDERED_ACTIVE).toBe("shallgraph.requirementRenderedActive");
    expect(RENDERED_VIEW_TYPE).toBe("shallgraph.requirementRendered");
  });
});

describe("GRD-VSC-003 classifyRequirementViewKind", () => {
  it("detects the rendered custom editor from the active tab", () => {
    expect(
      classifyRequirementViewKind(
        { kind: "custom", viewType: RENDERED_VIEW_TYPE },
        undefined
      )
    ).toBe("rendered");
  });

  it("detects requirement YAML from a text tab basename", () => {
    expect(
      classifyRequirementViewKind(
        { kind: "text", basename: "GRD-VSC-003.req.yml" },
        undefined
      )
    ).toBe("source");
    expect(
      classifyRequirementViewKind(
        { kind: "text", basename: "GRD-VSC-003.req.yaml" },
        undefined
      )
    ).toBe("source");
  });

  it("falls back to the active text editor basename when the tab is not a text input", () => {
    expect(classifyRequirementViewKind({ kind: "other" }, "GRD-VSC-003.req.yml")).toBe("source");
    expect(classifyRequirementViewKind(undefined, "GRD-VSC-003.req.yml")).toBe("source");
  });

  it("ignores non-requirement files and other custom editors", () => {
    expect(
      classifyRequirementViewKind({ kind: "text", basename: "shallgraph.yaml" }, undefined)
    ).toBeNull();
    expect(
      classifyRequirementViewKind({ kind: "custom", viewType: "other.editor" }, undefined)
    ).toBeNull();
    expect(classifyRequirementViewKind(undefined, "readme.md")).toBeNull();
  });

  it("prefers the rendered tab over a leftover text-editor basename", () => {
    expect(
      classifyRequirementViewKind(
        { kind: "custom", viewType: RENDERED_VIEW_TYPE },
        "GRD-VSC-003.req.yml"
      )
    ).toBe("rendered");
  });
});
