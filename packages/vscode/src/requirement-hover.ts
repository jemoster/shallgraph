/**
 * GRD-VSC-007: Hover shows requirement title for requirement ids in YAML text (links and plain references).
 */
import fs from "node:fs";
import * as vscode from "vscode";
import { parseRequirementDocuments } from "@shallgraph/core";
import { resolveRequirementPath, type LogFn } from "./link-resolver.js";
import { findRequirementIdAtLinePosition } from "./requirement-id-at-position.js";

export { findRequirementIdAtLinePosition, REQUIREMENT_ID_REFERENCE_REGEX } from "./requirement-id-at-position.js";

/** GRD-VSC-007: Register hover provider that resolves ids to requirement titles when the id exists in the workspace. */
export function registerRequirementHoverProvider(
  selector: vscode.DocumentSelector,
  log: LogFn
): vscode.Disposable {
  return vscode.languages.registerHoverProvider(selector, {
    async provideHover(
      document: vscode.TextDocument,
      position: vscode.Position,
      token: vscode.CancellationToken
    ): Promise<vscode.Hover | undefined> {
      const folder = vscode.workspace.getWorkspaceFolder(document.uri);
      if (!folder) {
        return undefined;
      }
      const lineText = document.lineAt(position.line).text;
      const id = findRequirementIdAtLinePosition(lineText, position.character);
      if (!id) {
        return undefined;
      }
      if (token.isCancellationRequested) {
        return undefined;
      }
      const targetPath = await resolveRequirementPath(folder.uri.fsPath, id, log);
      if (!targetPath) {
        return undefined;
      }
      let raw: string;
      try {
        raw = fs.readFileSync(targetPath, "utf8");
      } catch (err) {
        const md = new vscode.MarkdownString();
        md.isTrusted = false;
        md.appendMarkdown(`**${id}**`);
        md.appendText("\n\n");
        md.appendText(`Could not read title: ${String(err)}`);
        return new vscode.Hover(md);
      }
      const parsed = parseRequirementDocuments(raw, targetPath);
      const requirement = parsed.requirements.find((item) => item.id === id);
      if (!requirement) {
        const md = new vscode.MarkdownString();
        md.isTrusted = false;
        md.appendMarkdown(`**${id}**`);
        md.appendText("\n\n");
        md.appendText(
          `Could not read title: ${parsed.errors[0]?.message ?? "requirement was not found in the file"}`
        );
        return new vscode.Hover(md);
      }
      const title = requirement.title;
      const md = new vscode.MarkdownString();
      md.isTrusted = false;
      md.appendMarkdown(`**${title}**`);
      md.appendText(`  \n`);
      md.appendText(`\`${id}\``);
      return new vscode.Hover(md);
    },
  });
}
