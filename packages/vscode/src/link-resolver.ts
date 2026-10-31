import fs from "node:fs";
import path from "node:path";
import {
  discoverProjectRoot,
  discoverRequirementPaths,
  parseRequirementDocuments,
  requirementIdFromFilename,
} from "@shallgraph/core";

const cache = new Map<string, Map<string, string>>();
const loggedOnce = new Set<string>();

export type LogFn = (message: string) => void;

function logOnce(key: string, log: LogFn | undefined, message: string): void {
  if (!log || loggedOnce.has(key)) return;
  loggedOnce.add(key);
  log(message);
}

export type ResolvedLink =
  | { kind: "requirement"; path: string; requirementId: string }
  | { kind: "file"; path: string }
  | { kind: "url"; url: string };

/** Drop cached id-to-path maps so the next lookup sees added or removed requirement files. */
export function clearRequirementPathCache(): void {
  cache.clear();
  loggedOnce.clear();
}

/**
 * Resolve a requirement id to its absolute file path. `startDir` is a directory
 * inside the project (usually the open requirement file's directory). The project
 * root is the nearest ancestor with `shallgraph.yaml` or `shallgraph.yml`.
 */
export async function resolveRequirementPath(
  startDir: string,
  requirementId: string,
  log?: LogFn
): Promise<string | null> {
  const projectRoot = await discoverProjectRoot(startDir);
  if (!projectRoot) {
    logOnce(
      `root:${startDir}`,
      log,
      `[ShallGraph] No project root from ${startDir}; links disabled.`
    );
    return null;
  }
  const idToPath = await getIdToPathMap(projectRoot, log);
  return idToPath.get(requirementId) ?? null;
}

/**
 * GRD-VSC-002: A link target opens the requirement file, a file path, or a URL.
 * Requirement ids win over a same-named file.
 */
export async function resolveLinkTarget(
  startDir: string,
  linkValue: string,
  fromDir: string,
  log?: LogFn
): Promise<ResolvedLink | null> {
  const value = linkValue.trim();
  if (!value) return null;
  if (/^https?:\/\//i.test(value)) return { kind: "url", url: value };

  const requirementPath = await resolveRequirementPath(startDir, value, log);
  if (requirementPath) return { kind: "requirement", path: requirementPath, requirementId: value };
  if (/^[a-z][a-z0-9+.-]*:/i.test(value)) return null;

  const projectRoot = await discoverProjectRoot(startDir);
  const candidates = [path.resolve(fromDir, value)];
  if (projectRoot) candidates.push(path.resolve(projectRoot, value));
  const seen = new Set<string>();
  for (const candidate of candidates) {
    if (seen.has(candidate)) continue;
    seen.add(candidate);
    try {
      if (fs.statSync(candidate).isFile()) return { kind: "file", path: candidate };
    } catch {
      // not a file
    }
  }
  return null;
}

/**
 * Build a map from requirement id to absolute file path for the given
 * project root. Cached by project root. A failed discovery is not cached.
 */
async function getIdToPathMap(
  projectRoot: string,
  log?: LogFn
): Promise<Map<string, string>> {
  const cached = cache.get(projectRoot);
  if (cached !== undefined) return cached;

  log?.(`[ShallGraph] Using project root: ${projectRoot}`);
  let paths: string[];
  try {
    paths = await discoverRequirementPaths(projectRoot);
    log?.(`[ShallGraph] Discovered ${paths.length} requirement file(s)`);
  } catch (err) {
    logOnce(
      `discover:${projectRoot}`,
      log,
      `[ShallGraph] Discovery failed: ${err instanceof Error ? err.message : String(err)}`
    );
    return new Map();
  }

  const idToPath = new Map<string, string>();
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
  cache.set(projectRoot, idToPath);
  return idToPath;
}
