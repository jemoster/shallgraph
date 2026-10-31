/**
 * VSCode extension for shallgraph: link resolution, same-tab rendered view (GRD-VSC-003) that stays
 * read-only (GRD-VSC-006), navigation, YAML schema for requirement files (GRD-VSC-004): registered at
 * runtime from core (GRD-SYS-009), refreshed when project root markers change, new requirement from
 * explorer (GRD-VSC-005), and hover titles for requirement id references (GRD-VSC-007).
 */
import * as vscode from "vscode";
import fs from "node:fs";
import * as path from "node:path";
import { REQUIREMENT_FILE_EXTENSION } from "@shallgraph/core";
import { isRequirementDocument } from "./requirement-document.js";
import { clearRequirementPathCache, resolveLinkTarget, type ResolvedLink } from "./link-resolver.js";
import { findRequirementLinkTargets, requirementLineInFile } from "./requirement-links.js";
import { newRequirementYamlTemplate } from "./new-requirement-template.js";
import {
  DEFAULT_TEXT_EDITOR,
  RENDERED_VIEW_TYPE,
  RequirementRenderedEditorProvider,
  SHOW_RENDERED_COMMAND,
  SHOW_SOURCE_COMMAND,
  switchRequirementEditor,
} from "./rendered-view.js";
import { registerRequirementYamlSchema } from "./requirement-yaml-schema.js";
import { registerRequirementHoverProvider } from "./requirement-hover.js";

const OUTPUT_CHANNEL_NAME = "ShallGraph";

function linkUri(resolved: ResolvedLink): vscode.Uri {
  return resolved.kind === "url" ? vscode.Uri.parse(resolved.url) : vscode.Uri.file(resolved.path);
}

function linkLocation(resolved: ResolvedLink): vscode.Location {
  if (resolved.kind === "requirement") {
    let line = 0;
    let character = 0;
    try {
      const raw = fs.readFileSync(resolved.path, "utf8");
      line = requirementLineInFile(raw, resolved.requirementId);
      const lineText = raw.split(/\r?\n/)[line] ?? "";
      const at = lineText.indexOf(resolved.requirementId);
      character = at >= 0 ? at : 0;
    } catch {
      line = 0;
      character = 0;
    }
    return new vscode.Location(vscode.Uri.file(resolved.path), new vscode.Position(line, character));
  }
  const uri = linkUri(resolved);
  return new vscode.Location(uri, new vscode.Position(0, 0));
}

export function activate(context: vscode.ExtensionContext): void {
  const outputChannel = vscode.window.createOutputChannel(OUTPUT_CHANNEL_NAME);
  const log = (message: string) => outputChannel.appendLine(message);
  context.subscriptions.push(outputChannel);
  log(
    "[ShallGraph] Extension active. Open a `.req.yml` or `.req.yaml` requirement file to resolve links; logs will appear here."
  );

  context.subscriptions.push(registerRequirementYamlSchema(context));
  context.subscriptions.push(RequirementRenderedEditorProvider.register(log));

  /** GRD-SYS-007: requirement files use `.req.yml` or `.req.yaml`. */
  const requirementYamlSelector: vscode.DocumentSelector = [
    { language: "yaml", pattern: "**/*.req.yml" },
    { language: "yaml", pattern: "**/*.req.yaml" },
  ];

  // GRD-VSC-002: Follow satisfies and every other link target. A requirement id opens that requirement's file.
  const invalidateLinks = () => clearRequirementPathCache();
  for (const pattern of ["**/*.req.yml", "**/*.req.yaml", "**/shallgraph.yaml", "**/shallgraph.yml"]) {
    const watcher = vscode.workspace.createFileSystemWatcher(pattern);
    context.subscriptions.push(
      watcher,
      watcher.onDidCreate(invalidateLinks),
      watcher.onDidChange(invalidateLinks),
      watcher.onDidDelete(invalidateLinks)
    );
  }

  const documentLinkProvider = vscode.languages.registerDocumentLinkProvider(
    requirementYamlSelector,
    {
      async provideDocumentLinks(
        document: vscode.TextDocument,
        token: vscode.CancellationToken
      ): Promise<vscode.DocumentLink[]> {
        const startDir = path.dirname(document.uri.fsPath);
        const links: vscode.DocumentLink[] = [];
        for (const target of findRequirementLinkTargets(document.getText())) {
          if (token.isCancellationRequested) return links;
          const resolved = await resolveLinkTarget(startDir, target.value, startDir, log);
          if (!resolved) continue;
          const range = new vscode.Range(
            document.positionAt(target.start),
            document.positionAt(target.end)
          );
          const link = new vscode.DocumentLink(range, linkUri(resolved));
          link.tooltip =
            resolved.kind === "requirement" ? `Open ${resolved.requirementId}` : `Open ${target.value}`;
          links.push(link);
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
        const startDir = path.dirname(document.uri.fsPath);
        const text = document.getText();
        for (const target of findRequirementLinkTargets(text)) {
          if (token.isCancellationRequested) return undefined;
          const range = new vscode.Range(
            document.positionAt(target.start),
            document.positionAt(target.end)
          );
          if (!range.contains(position)) continue;
          const resolved = await resolveLinkTarget(startDir, target.value, startDir, log);
          if (!resolved || resolved.kind === "url") return undefined;
          return linkLocation(resolved);
        }
        return undefined;
      },
    }
  );

  // GRD-VSC-007: Hover shows resolved requirement title for ids under the pointer (satisfies lines and other references).
  const hoverProvider = registerRequirementHoverProvider(requirementYamlSelector, log);

  // GRD-VSC-003: Editor title action switches this tab between YAML and the rendered view.
  const showRenderedCommand = vscode.commands.registerCommand(SHOW_RENDERED_COMMAND, async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !isRequirementDocument(editor.document)) {
      log("[ShallGraph] Show Rendered View applies only to `.req.yml` / `.req.yaml` requirement files.");
      return;
    }
    await switchRequirementEditor(editor.document.uri, RENDERED_VIEW_TYPE);
  });

  const showSourceCommand = vscode.commands.registerCommand(SHOW_SOURCE_COMMAND, async () => {
    const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
    const input = tab?.input;
    if (!(input instanceof vscode.TabInputCustom) || input.viewType !== RENDERED_VIEW_TYPE) {
      log("[ShallGraph] Show YAML Source applies when the rendered requirement view is active.");
      return;
    }
    await switchRequirementEditor(input.uri, DEFAULT_TEXT_EDITOR);
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
    newRequirementCommand
  );
}

export function deactivate(): void {}
