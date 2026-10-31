/**
 * GRD-VSC-003: Switch a requirement file between the YAML text editor and a rendered view in the same tab.
 * GRD-VSC-006: The rendered view is read-only. Requirement text is edited in the YAML source.
 * GRD-HTML-008: file paths in the rendered HTML are GitHub blob links when origin is GitHub.
 */
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  discoverProjectRoot,
  getRequirementProfile,
  loadActiveProfile,
  loadRequirements,
  parseRequirementDocuments,
  STANDARD_PROFILE_ID,
} from "@shallgraph/core";
import type { RequirementProfile } from "@shallgraph/core";
import type { ArtifactLinkRenderOptions, RequirementWithSource } from "@shallgraph/core";
import { resolveRequirementPath } from "./link-resolver.js";
import { requirementLineInFile } from "./requirement-links.js";
import {
  buildRenderedErrorHtml,
  buildRenderedWebviewHtml,
  combineRequirementHtml,
  requirementIdFromLinkHref,
} from "./rendered-html.js";

export const RENDERED_VIEW_TYPE = "shallgraph.requirementRendered";
export const SHOW_RENDERED_COMMAND = "shallgraph.requirement.showRendered";
export const SHOW_SOURCE_COMMAND = "shallgraph.requirement.showSource";

/** GRD-VSC-003: Reopen this file with the text editor (`default`) or the rendered custom editor. */
export const DEFAULT_TEXT_EDITOR = "default";

function gitStdout(cwd: string, args: string[]): string | undefined {
  try {
    const out = execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
    return out || undefined;
  } catch {
    return undefined;
  }
}

function parseGithubOrigin(url: string): { host: string; owner: string; repo: string } | undefined {
  const trimmed = url.trim();
  let host = "";
  let repoPath = "";
  const scp = /^git@([^:]+):(.+)$/.exec(trimmed);
  if (scp) {
    host = scp[1] ?? "";
    repoPath = scp[2] ?? "";
  } else {
    const m = /^(?:ssh:\/\/|https?:\/\/|git:\/\/)(?:[^@]+@)?([^/:]+)(?::\d+)?\/(.+)$/.exec(trimmed);
    if (!m) return undefined;
    host = m[1] ?? "";
    repoPath = m[2] ?? "";
  }
  const hostLower = host.toLowerCase();
  if (hostLower !== "github.com" && !hostLower.startsWith("github.")) {
    return undefined;
  }
  const cleaned = repoPath.replace(/\/+$/, "").replace(/\.git$/, "");
  const [owner, repo] = cleaned.split("/");
  if (!owner || !repo) return undefined;
  return { host, owner, repo };
}

/** GRD-HTML-008: GitHub blob context from `origin` and `HEAD` for the workspace folder. */
function discoverGithubLinkContext(projectRoot: string): ArtifactLinkRenderOptions["github"] | undefined {
  const toplevel = gitStdout(projectRoot, ["rev-parse", "--show-toplevel"]);
  const origin = gitStdout(projectRoot, ["remote", "get-url", "origin"]);
  const head = gitStdout(projectRoot, ["rev-parse", "HEAD"]);
  if (!toplevel || !origin || !head) return undefined;
  const parsed = parseGithubOrigin(origin);
  if (!parsed) return undefined;
  const projectAbs = path.resolve(projectRoot).replace(/\\/g, "/");
  const topAbs = path.resolve(toplevel).replace(/\\/g, "/");
  let projectRootRel = "";
  if (projectAbs !== topAbs && projectAbs.startsWith(`${topAbs}/`)) {
    projectRootRel = projectAbs.slice(topAbs.length + 1);
  }
  return {
    host: parsed.host,
    owner: parsed.owner,
    repo: parsed.repo,
    commitSha: head,
    projectRootRel,
    projectRoot: projectAbs,
  };
}

async function readRequirementText(uri: vscode.Uri): Promise<string> {
  const open = vscode.workspace.textDocuments.find((d) => d.uri.toString() === uri.toString());
  if (open) return open.getText();
  const bytes = await vscode.workspace.fs.readFile(uri);
  return new TextDecoder("utf8").decode(bytes);
}

function tabMatchesUri(tab: vscode.Tab, uri: vscode.Uri): boolean {
  const input = tab.input;
  if (input instanceof vscode.TabInputText || input instanceof vscode.TabInputCustom) {
    return input.uri.toString() === uri.toString();
  }
  return false;
}

function tabIsView(tab: vscode.Tab, viewId: string): boolean {
  const input = tab.input;
  if (viewId === DEFAULT_TEXT_EDITOR) {
    return input instanceof vscode.TabInputText;
  }
  return input instanceof vscode.TabInputCustom && input.viewType === viewId;
}

/**
 * GRD-VSC-003: Replace the active editor for this file so YAML and the rendered view share one tab.
 * `vscode.openWith` reopens the resource with the requested editor. Any leftover tab for the same
 * file in the active group is closed after the requested editor is open.
 */
export async function switchRequirementEditor(uri: vscode.Uri, viewId: string): Promise<void> {
  await vscode.commands.executeCommand("vscode.openWith", uri, viewId, {
    viewColumn: vscode.ViewColumn.Active,
    preview: false,
  });
  const group = vscode.window.tabGroups.activeTabGroup;
  const matches = group.tabs.filter((tab) => tabMatchesUri(tab, uri));
  const desired = matches.filter((tab) => tabIsView(tab, viewId));
  if (desired.length === 0) return;
  const extras = matches.filter((tab) => !tabIsView(tab, viewId));
  if (extras.length > 0) {
    await vscode.window.tabGroups.close(extras, true);
  }
}

/**
 * GRD-VSC-003: A requirement link in the rendered view opens the target file with the YAML editor.
 */
async function openRequirementAsYaml(
  requirementId: string,
  fromUri: vscode.Uri,
  log: (message: string) => void
): Promise<void> {
  const id =
    requirementIdFromLinkHref(requirementId.startsWith("#") ? requirementId : `#${requirementId}`) ??
    requirementIdFromLinkHref(requirementId);
  if (!id) {
    log("[ShallGraph] Not a requirement link: " + requirementId);
    return;
  }
  const startDir = path.dirname(fromUri.fsPath);
  const targetPath = await resolveRequirementPath(startDir, id, log);
  if (!targetPath) {
    log("[ShallGraph] Requirement not found: " + id);
    vscode.window.showErrorMessage(`ShallGraph: Requirement not found: ${id}`);
    return;
  }
  const uri = vscode.Uri.file(targetPath);
  await switchRequirementEditor(uri, DEFAULT_TEXT_EDITOR);
  let line = 0;
  try {
    line = requirementLineInFile(fs.readFileSync(targetPath, "utf8"), id);
  } catch {
    line = 0;
  }
  const doc = await vscode.workspace.openTextDocument(uri);
  const safeLine = Math.min(line, Math.max(0, doc.lineCount - 1));
  const at = doc.lineAt(safeLine).text.indexOf(id);
  const pos = new vscode.Position(safeLine, at >= 0 ? at : 0);
  await vscode.window.showTextDocument(doc, {
    viewColumn: vscode.ViewColumn.Active,
    preview: false,
    selection: new vscode.Range(pos, pos),
  });
}

/** GRD-VSC-006: Custom editor shows report HTML and never writes the requirement file. */
export class RequirementRenderedEditorProvider implements vscode.CustomReadonlyEditorProvider {
  constructor(
    private readonly log: (message: string) => void
  ) {}

  static register(log: (message: string) => void): vscode.Disposable {
    return vscode.window.registerCustomEditorProvider(
      RENDERED_VIEW_TYPE,
      new RequirementRenderedEditorProvider(log),
      { webviewOptions: { retainContextWhenHidden: true } }
    );
  }

  async openCustomDocument(
    uri: vscode.Uri,
    _openContext: vscode.CustomDocumentOpenContext,
    _token: vscode.CancellationToken
  ): Promise<vscode.CustomDocument> {
    return { uri, dispose() {} };
  }

  async resolveCustomEditor(
    document: vscode.CustomDocument,
    webviewPanel: vscode.WebviewPanel,
    _token: vscode.CancellationToken
  ): Promise<void> {
    webviewPanel.webview.options = { enableScripts: true };
    const subscriptions: vscode.Disposable[] = [];
    let generation = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const renderNow = async () => {
      const gen = ++generation;
      const html = await this.renderHtml(document.uri, webviewPanel.webview);
      if (gen !== generation) return;
      webviewPanel.webview.html = html;
    };
    const schedule = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = undefined;
        void renderNow();
      }, 50);
    };

    subscriptions.push(
      vscode.workspace.onDidChangeTextDocument((e) => {
        if (e.document.uri.toString() === document.uri.toString()) schedule();
      })
    );
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(
        vscode.Uri.file(path.dirname(document.uri.fsPath)),
        path.basename(document.uri.fsPath)
      )
    );
    subscriptions.push(watcher, watcher.onDidChange(schedule), watcher.onDidCreate(schedule));
    subscriptions.push(
      webviewPanel.webview.onDidReceiveMessage((msg: { type?: string; requirementId?: string }) => {
        if (msg?.type === "openRequirement" && typeof msg.requirementId === "string") {
          void openRequirementAsYaml(msg.requirementId, document.uri, this.log);
        }
      })
    );
    webviewPanel.onDidDispose(() => {
      if (timer) clearTimeout(timer);
      for (const sub of subscriptions) sub.dispose();
    });

    await renderNow();
  }

  private async renderHtml(uri: vscode.Uri, webview: vscode.Webview): Promise<string> {
    const options = { cspSource: webview.cspSource, nonce: randomBytes(16).toString("base64url") };
    try {
      const text = await readRequirementText(uri);
      const parsed = parseRequirementDocuments(text, uri.fsPath);
      if (parsed.errors.length > 0 || parsed.requirements.length === 0) {
        const message =
          parsed.errors
            .map((err) => (err.line != null ? `${err.message} (line ${err.line})` : err.message))
            .join("\n") || "Requirement file contains no requirement document";
        return buildRenderedErrorHtml(message, options);
      }
      const fileRequirements = parsed.requirements.map((requirement) => ({
        ...requirement,
        sourcePath: uri.fsPath,
      }));
      let allRequirements: RequirementWithSource[] | undefined;
      const folder = vscode.workspace.getWorkspaceFolder(uri);
      if (folder) {
        try {
          const result = await loadRequirements(folder.uri.fsPath);
          const byId = new Map(result.requirements.map((requirement) => [requirement.id, requirement]));
          for (const requirement of fileRequirements) {
            byId.set(requirement.id, requirement);
          }
          allRequirements = [...byId.values()];
        } catch {
          allRequirements = undefined;
        }
      }
      let profile: RequirementProfile = getRequirementProfile(STANDARD_PROFILE_ID);
      if (folder) {
        const projectRoot = await discoverProjectRoot(folder.uri.fsPath);
        if (projectRoot) profile = loadActiveProfile(projectRoot);
      }
      const projectRoot = folder?.uri.fsPath ?? path.dirname(uri.fsPath);
      const parts = fileRequirements.map((requirement) =>
        profile.generateSingleRequirementHtml(requirement, allRequirements, {
          artifactLinks: { github: discoverGithubLinkContext(projectRoot) },
        })
      );
      return buildRenderedWebviewHtml(combineRequirementHtml(parts), options);
    } catch (err) {
      const message = `[ShallGraph] Failed to render ${uri.fsPath}: ${String(err)}`;
      this.log(message);
      return buildRenderedErrorHtml(String(err), options);
    }
  }
}
