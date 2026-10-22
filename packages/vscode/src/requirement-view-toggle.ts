/**
 * GRD-VSC-003: Keep the YAML ↔ rendered switch discoverable when the host hides
 * editor-title action icons (Cursor 2.1+). Tracks the active requirement editor via
 * context keys for menus/keybindings and a status-bar toggle that stays visible.
 */
import * as path from "node:path";
import * as vscode from "vscode";
import { isRequirementDocument } from "./requirement-document.js";
import {
  CONTEXT_REQUIREMENT_RENDERED_ACTIVE,
  CONTEXT_REQUIREMENT_SOURCE_ACTIVE,
  SHOW_RENDERED_COMMAND,
  SHOW_SOURCE_COMMAND,
  classifyRequirementViewKind,
  type RequirementTabSnapshot,
  type RequirementViewKind,
} from "./requirement-view-kind.js";

export {
  CONTEXT_REQUIREMENT_RENDERED_ACTIVE,
  CONTEXT_REQUIREMENT_SOURCE_ACTIVE,
  classifyRequirementViewKind,
  type RequirementTabSnapshot,
  type RequirementViewKind,
} from "./requirement-view-kind.js";

function snapshotActiveTab(activeTab: vscode.Tab | undefined): RequirementTabSnapshot | undefined {
  const input = activeTab?.input;
  if (input instanceof vscode.TabInputCustom) {
    return { kind: "custom", viewType: input.viewType };
  }
  if (input instanceof vscode.TabInputText) {
    return {
      kind: "text",
      basename: path.basename(input.uri.fsPath || input.uri.path),
    };
  }
  if (input) {
    return { kind: "other" };
  }
  return undefined;
}

/**
 * Resolve whether the active tab is requirement YAML or the rendered view.
 */
export function activeRequirementViewKind(
  activeTab: vscode.Tab | undefined,
  activeTextEditor: vscode.TextEditor | undefined
): RequirementViewKind | null {
  const activeEditorBasename =
    activeTextEditor && isRequirementDocument(activeTextEditor.document)
      ? path.basename(activeTextEditor.document.uri.fsPath)
      : undefined;
  return classifyRequirementViewKind(snapshotActiveTab(activeTab), activeEditorBasename);
}

async function setRequirementViewContexts(kind: RequirementViewKind | null): Promise<void> {
  await Promise.all([
    vscode.commands.executeCommand("setContext", CONTEXT_REQUIREMENT_SOURCE_ACTIVE, kind === "source"),
    vscode.commands.executeCommand(
      "setContext",
      CONTEXT_REQUIREMENT_RENDERED_ACTIVE,
      kind === "rendered"
    ),
  ]);
}

function applyStatusBar(
  item: vscode.StatusBarItem,
  kind: RequirementViewKind | null
): void {
  if (kind === "source") {
    item.text = "$(open-preview) ShallGraph";
    item.tooltip = "ShallGraph: Show Rendered View";
    item.command = SHOW_RENDERED_COMMAND;
    item.show();
    return;
  }
  if (kind === "rendered") {
    item.text = "$(file-code) ShallGraph";
    item.tooltip = "ShallGraph: Show YAML Source";
    item.command = SHOW_SOURCE_COMMAND;
    item.show();
    return;
  }
  item.hide();
}

export type RequirementViewToggle = {
  /** Re-read the active tab and refresh context keys + status bar. */
  refresh: () => void;
};

/**
 * GRD-VSC-003: Register context keys + status bar so the same-tab switch remains
 * available when editor-title icons are compressed into the overflow menu.
 */
export function registerRequirementViewToggle(
  context: vscode.ExtensionContext
): RequirementViewToggle {
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  context.subscriptions.push(statusBar);

  const refresh = (): void => {
    const kind = activeRequirementViewKind(
      vscode.window.tabGroups.activeTabGroup.activeTab,
      vscode.window.activeTextEditor
    );
    void setRequirementViewContexts(kind);
    applyStatusBar(statusBar, kind);
  };

  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(() => refresh()),
    vscode.window.tabGroups.onDidChangeTabs(() => refresh()),
    vscode.window.tabGroups.onDidChangeTabGroups(() => refresh())
  );

  refresh();
  return { refresh };
}
