import fs from "node:fs";
import path from "node:path";
import {
  discoverRequirementPaths,
  parseRequirementDocuments,
  requirementIdFromFilename,
} from "@shallgraph/core";

const cache = new Map<string, Map<string, string>>();

export type LogFn = (message: string) => void;

/**
 * Resolve a requirement id to its absolute file path. Uses the workspace root
 * as project root. Results are cached per workspace root.
 */
export async function resolveRequirementPath(
  workspaceRoot: string,
  requirementId: string,
  log?: LogFn
): Promise<string | null> {
  const idToPath = await getIdToPathMap(workspaceRoot, log);
  return idToPath.get(requirementId) ?? null;
}

/**
 * Build a map from requirement id to absolute file path for the given
 * workspace root. Cached by workspace root.
 */
async function getIdToPathMap(
  workspaceRoot: string,
  log?: LogFn
): Promise<Map<string, string>> {
  let idToPath = cache.get(workspaceRoot);
  if (idToPath !== undefined) {
    return idToPath;
  }

  log?.(`[ShallGraph] Using workspace root: ${workspaceRoot}`);
  let paths: string[];
  try {
    paths = await discoverRequirementPaths(workspaceRoot);
    log?.(`[ShallGraph] Discovered ${paths.length} requirement file(s)`);
  } catch (err) {
    paths = [];
    log?.(`[ShallGraph] Discovery failed: ${err instanceof Error ? err.message : String(err)}`);
  }

  idToPath = new Map<string, string>();
  for (const filePath of paths) {
    let raw: string;
    try {
      raw = fs.readFileSync(filePath, "utf8");
    } catch {
      continue;
    }
    const parsed = parseRequirementDocuments(raw, filePath);
    if (parsed.requirements.length > 0) {
      for (const requirement of parsed.requirements) {
        if (!idToPath.has(requirement.id)) {
          idToPath.set(requirement.id, filePath);
        }
      }
    } else {
      const id = requirementIdFromFilename(path.basename(filePath));
      if (id !== null && !idToPath.has(id)) {
        idToPath.set(id, filePath);
      }
    }
  }
  cache.set(workspaceRoot, idToPath);
  return idToPath;
}
