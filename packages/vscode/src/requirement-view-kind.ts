/**
 * GRD-VSC-003: Pure helpers for detecting YAML vs rendered requirement view.
 * Kept free of the `vscode` module so unit tests can import them under Jest.
 */
import { isRequirementFilename } from "@shallgraph/core";

/** Must match the custom editor viewType in package.json / rendered-view.ts. */
export const RENDERED_VIEW_TYPE = "shallgraph.requirementRendered";

export const SHOW_RENDERED_COMMAND = "shallgraph.requirement.showRendered";
export const SHOW_SOURCE_COMMAND = "shallgraph.requirement.showSource";
/** Status-bar entry point: toggles YAML ↔ rendered without depending on a mode-specific command id. */
export const TOGGLE_VIEW_COMMAND = "shallgraph.requirement.toggleView";

/** Context key: YAML text editor for a requirement file is active. */
export const CONTEXT_REQUIREMENT_SOURCE_ACTIVE = "shallgraph.requirementSourceActive";
/** Context key: ShallGraph rendered custom editor is active. */
export const CONTEXT_REQUIREMENT_RENDERED_ACTIVE = "shallgraph.requirementRenderedActive";

export type RequirementViewKind = "source" | "rendered";

/** Tab shape used to classify the active requirement view without vscode instanceof. */
export type RequirementTabSnapshot = {
  kind: "text" | "custom" | "other";
  viewType?: string;
  basename?: string;
};

/**
 * Classify the active requirement editor from tab + text-editor snapshots.
 * Prefers tab input so custom editors are detected even when `activeTextEditor` is unset.
 */
export function classifyRequirementViewKind(
  tab: RequirementTabSnapshot | undefined,
  activeEditorBasename: string | undefined
): RequirementViewKind | null {
  if (tab?.kind === "custom" && tab.viewType === RENDERED_VIEW_TYPE) {
    return "rendered";
  }
  if (tab?.kind === "text" && tab.basename && isRequirementFilename(tab.basename)) {
    return "source";
  }
  if (activeEditorBasename && isRequirementFilename(activeEditorBasename)) {
    return "source";
  }
  return null;
}
