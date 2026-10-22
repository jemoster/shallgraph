/**
 * VSCode extension for shallgraph: link resolution, same-tab rendered view (GRD-VSC-003) that stays
 * read-only (GRD-VSC-006) with context keys + status-bar toggle for hosts that hide title icons,
 * navigation, YAML schema for requirement files (GRD-VSC-004): registered at runtime from core
 * (GRD-SYS-009), refreshed when project root markers change, new requirement from explorer
 * (GRD-VSC-005), and hover titles for requirement id references (GRD-VSC-007).
 */
import * as vscode from "vscode";
import * as path from "node:path";
import { REQUIREMENT_FILE_EXTENSION, isRequirementFilename } from "@shallgraph/core";
import { isRequirementDocument } from "./requirement-document.js";
import { resolveRequirementPath } from "./link-resolver.js";
import { newRequirementYamlTemplate } from "./new-requirement-template.js";
import {
  DEFAULT_TEXT_EDITOR,
  RENDERED_VIEW_TYPE,
  RequirementRenderedEditorProvider,
  SHOW_RENDERED_COMMAND,
  SHOW_SOURCE_COMMAND,
  switchRequirementEditor,
} from "./rendered-view.js";
import { TOGGLE_VIEW_COMMAND } from "./requirement-view-kind.js";
import { registerRequirementYamlSchema } from "./requirement-yaml-schema.js";
import { registerRequirementHoverProvider } from "./requirement-hover.js";
import { registerRequirementViewToggle } from "./requirement-view-toggle.js";

/**
 * GRD-VSC-003: Resolve the active requirement URI from the tab first.
 * Status-bar clicks can clear `activeTextEditor` before the command runs.
 */
function activeRequirementSourceUri(): vscode.Uri | undefined {
  const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
  const input = tab?.input;
  if (input instanceof vscode.TabInputText) {
    const basename = path.basename(input.uri.fsPath || input.uri.path);
    if (isRequirementFilename(basename)) {
      return input.uri;
    }
  }
  const editor = vscode.window.activeTextEditor;
  if (editor && isRequirementDocument(editor.document)) {
    return editor.document.uri;
  }
  return undefined;
}

function activeRequirementRenderedUri(): vscode.Uri | undefined {
  const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
  const input = tab?.input;
  if (input instanceof vscode.TabInputCustom && input.viewType === RENDERED_VIEW_TYPE) {
    return input.uri;
  }
  return undefined;
}

const SATISFIES_REGEX = /satisfies:\s*['"]?([^\s'"\n]+)['"]?/g;

function* findLinkRanges(document: vscode.TextDocument): Generator<{ range: vscode.Range; id: string }> {
  for (let i = 0; i < document.lineCount; i++) {
    const line = document.lineAt(i);
    const text = line.text;
    let match: RegExpExecArray | null;
    SATISFIES_REGEX.lastIndex = 0;
    while ((match = SATISFIES_REGEX.exec(text)) !== null) {
      const id = match[1]!;
      const startOffset = match.index + match[0].indexOf(id);
      const endOffset = startOffset + id.length;
      yield {
        range: new vscode.Range(i, startOffset, i, endOffset),
        id,
      };
    }
  }
}

const OUTPUT_CHANNEL_NAME = "ShallGraph";

export function activate(context: vscode.ExtensionContext): void {
  const outputChannel = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
  const log = (message: string) => outputChannel.appendLine(message);
  context.subscriptions.push(outputChannel);
  log(
    "[ShallGraph] Extension active. Open a `.req.yml` or `.req.yaml` requirement file to resolve links; logs will appear here."
  );

  context.subscriptions.push(registerRequirementYamlSchema(context));
  // GRD-VSC-003: Context keys + status bar toggle (Cursor hides editor-title icons by default).
  const viewToggle = registerRequirementViewToggle(context);
  context.subscriptions.push(RequirementRenderedEditorProvider.register(log, viewToggle.refresh));

  /** GRD-SYS-007: requirement files use `.req.yml` or `.req.yaml`. */
  const requirementYamlSelector: vscode.DocumentSelector = [
    { language: "yaml", pattern: "**/*.req.yml" },
    { language: "yaml", pattern: "**/*.req.yaml" },
  ];

  const documentLinkProvider = vscode.languages.registerDocumentLinkProvider(
    requirementYamlSelector,
    {
      async provideDocumentLinks(
        document: vscode.TextDocument,
        _token: vscode.CancellationToken
      ): Promise<vscode.DocumentLink[]> {
        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        if (!folder) {
          log("[ShallGraph] No workspace folder for document; links disabled.");
          return [];
        }
        const workspaceRoot = folder.uri.fsPath;
        const links: vscode.DocumentLink[] = [];
        for (const { range, id } of findLinkRanges(document)) {
          const targetPath = await resolveRequirementPath(workspaceRoot, id, log);
          if (targetPath) {
            links.push(new vscode.DocumentLink(range, vscode.Uri.file(targetPath)));
          }
        }
        return links;
      },
    }
  );

  const definitionProvider = vscode.languages.registerDefinitionProvider(
    requirementYamlSelector,
    {
      async provideDefinition(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken
      ): Promise<vscode.Definition | undefined> {
        const folder = vscode.workspace.getWorkspaceFolder(document.uri);
        if (!folder) {
          log("[ShallGraph] No workspace folder for document; Go to Definition disabled.");
          return undefined;
        }
        const workspaceRoot = folder.uri.fsPath;
        for (const { range, id } of findLinkRanges(document)) {
          if (token.isCancellationRequested) {
            return undefined;
          }
          if (range.contains(position)) {
            const targetPath = await resolveRequirementPath(workspaceRoot, id, log);
            if (targetPath) {
              return new vscode.Location(vscode.Uri.file(targetPath), new vscode.Position(0, 0));
            }
            return undefined;
          }
        }
        return undefined;
      },
    }
  );

  // GRD-VSC-007: Hover shows resolved requirement title for ids under the pointer (satisfies lines and other references).
  const hoverProvider = registerRequirementHoverProvider(requirementYamlSelector, log);

  // GRD-VSC-003: Editor title / status bar / keybinding switches this tab between YAML and rendered.
  const showRenderedCommand = vscode.commands.registerCommand(SHOW_RENDERED_COMMAND, async () => {
    const uri = activeRequirementSourceUri();
    if (!uri) {
      log("[ShallGraph] Show Rendered View applies only to `.req.yml` / `.req.yaml` requirement files.");
      return;
    }
    await switchRequirementEditor(uri, RENDERED_VIEW_TYPE);
    viewToggle.refresh();
  });

  const showSourceCommand = vscode.commands.registerCommand(SHOW_SOURCE_COMMAND, async () => {
    const uri = activeRequirementRenderedUri();
    if (!uri) {
      log("[ShallGraph] Show YAML Source applies when the rendered requirement view is active.");
      return;
    }
    await switchRequirementEditor(uri, DEFAULT_TEXT_EDITOR);
    viewToggle.refresh();
  });

  // GRD-VSC-003: Status bar uses a single toggle so clicks stay correct if editor focus blurs first.
  const toggleViewCommand = vscode.commands.registerCommand(TOGGLE_VIEW_COMMAND, async () => {
    const renderedUri = activeRequirementRenderedUri();
    if (renderedUri) {
      await switchRequirementEditor(renderedUri, DEFAULT_TEXT_EDITOR);
      viewToggle.refresh();
      return;
    }
    const sourceUri = activeRequirementSourceUri();
    if (sourceUri) {
      await switchRequirementEditor(sourceUri, RENDERED_VIEW_TYPE);
      viewToggle.refresh();
      return;
    }
    log("[ShallGraph] Toggle Rendered View applies only to `.req.yml` / `.req.yaml` requirement files.");
  });

  // GRD-VSC-005: Explorer context menu — create a new requirement with schema-based template.
  const newRequirementCommand = vscode.commands.registerCommand(
    "shallgraph.requirement.new",
    async (resource?: vscode.Uri) => {
      let targetDir: string;
      if (resource) {
        const stat = await vscode.workspace.fs.stat(resource);
        targetDir =
          stat.type === vscode.FileType.Directory
            ? resource.fsPath
            : path.dirname(resource.fsPath);
      } else {
        const folder = vscode.workspace.workspaceFolders?.[0];
        if (!folder) {
          log("[ShallGraph] No workspace folder; open a folder to add a new requirement.");
          vscode.window.showErrorMessage("ShallGraph: Open a workspace folder first.");
          return;
        }
        targetDir = folder.uri.fsPath;
      }

      const id = await vscode.window.showInputBox({
        title: "ShallGraph: New Requirement",
        prompt: `Requirement ID (e.g. GRD-VSC-006). Filename will be {id}${REQUIREMENT_FILE_EXTENSION}`,
        placeHolder: "GRD-XXX-001",
        validateInput(value) {
          if (!value.trim()) {
            return "ID is required.";
          }
          if (!/^[A-Za-z0-9][A-Za-z0-9-]*$/.test(value.trim())) {
            return "ID must be alphanumeric with optional hyphens.";
          }
          return null;
        },
      });
      if (id === undefined || !id.trim()) {
        return;
      }
      const trimmedId = id.trim();
      const filePath = path.join(targetDir, `${trimmedId}${REQUIREMENT_FILE_EXTENSION}`);
      const uri = vscode.Uri.file(filePath);
      try {
        await vscode.workspace.fs.stat(uri);
        vscode.window.showErrorMessage(`ShallGraph: File already exists: ${trimmedId}${REQUIREMENT_FILE_EXTENSION}`);
        return;
      } catch {
        // file does not exist, ok to create
      }
      const content = newRequirementYamlTemplate(trimmedId);
      await vscode.workspace.fs.writeFile(uri, Buffer.from(content, "utf8"));
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc, { viewColumn: vscode.ViewColumn.Active });
      log(`[ShallGraph] Created requirement: ${filePath}`);
    }
  );

  context.subscriptions.push(
    documentLinkProvider,
    definitionProvider,
    hoverProvider,
    showRenderedCommand,
    showSourceCommand,
    toggleViewCommand,
    newRequirementCommand
  );
}

export function deactivate(): void {}
