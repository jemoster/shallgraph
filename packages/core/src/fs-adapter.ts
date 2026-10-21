import { glob } from "glob";
import fs from "node:fs";
import path from "node:path";
import {
  customTagComposeOptions,
  parseRootMarker,
  parseRequirementContent,
  parseRequirementDocuments,
  validateCustomTags,
  validateRequirements,
} from "./engine.js";
import {
  REQUIREMENT_FILE_EXTENSIONS,
  ROOT_MARKER,
  ROOT_MARKER_FILENAMES,
  ROOT_MARKER_HINT,
} from "./constants.js";
import type {
  DiscoverResult,
  LoadResult,
  RequirementSchemaComposeOptions,
  RequirementWithSource,
  ValidationError,
} from "./types.js";

export { ROOT_MARKER, ROOT_MARKER_FILENAMES, ROOT_MARKER_HINT };

export function findRootMarkerPath(projectRoot: string): string | null {
  for (const name of ROOT_MARKER_FILENAMES) {
    const p = path.join(projectRoot, name);
    if (fs.existsSync(p)) {
      return p;
    }
  }
  return null;
}

export async function discoverProjectRootCandidates(startDir: string): Promise<string[]> {
  const resolved = path.resolve(startDir);
  let current = fs.statSync(resolved).isDirectory() ? resolved : path.dirname(resolved);
  while (true) {
    if (findRootMarkerPath(current) !== null) {
      return [current];
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return [];
    }
    current = parent;
  }
}

export async function discoverProjectRoot(startDir: string): Promise<string | null> {
  const candidates = await discoverProjectRootCandidates(startDir);
  return candidates.length === 1 ? candidates[0]! : null;
}

export function getRequirementDirs(projectRoot: string): string[] {
  const rootPath = findRootMarkerPath(projectRoot);
  if (rootPath === null) {
    throw new Error(`Failed to find ${ROOT_MARKER_HINT} under ${path.resolve(projectRoot)}`);
  }
  const markerLabel = path.basename(rootPath);
  const raw = fs.readFileSync(rootPath, "utf-8");
  const parsed = parseRootMarker(raw, markerLabel);
  if ("error" in parsed) {
    throw new Error(parsed.error.message);
  }
  const resolvedDirs: string[] = [];
  const seen = new Set<string>();
  for (const trimmed of parsed.requirementDirs) {
    const abs = path.resolve(projectRoot, trimmed);
    if (seen.has(abs)) {
      throw new Error(
        `Invalid ${markerLabel} at ${rootPath}: duplicate "requirement_dirs" entry after resolving paths: ${abs}`
      );
    }
    seen.add(abs);
    resolvedDirs.push(abs);
  }
  return resolvedDirs;
}

export function getActiveProfileId(projectRoot: string): string {
  const rootPath = findRootMarkerPath(path.resolve(projectRoot));
  if (rootPath === null) {
    return "standard";
  }
  const markerLabel = path.basename(rootPath);
  let raw: string;
  try {
    raw = fs.readFileSync(rootPath, "utf-8");
  } catch {
    return "standard";
  }
  const parsed = parseRootMarker(raw, markerLabel);
  if ("error" in parsed) {
    throw new Error(parsed.error.message);
  }
  return parsed.profile;
}

export async function discoverRequirementPaths(projectRoot: string): Promise<string[]> {
  const cwd = path.resolve(projectRoot);
  const requirementDirs = getRequirementDirs(cwd);
  if (requirementDirs.length === 0) {
    return [];
  }
  const relDirs = requirementDirs.map((abs) => path.relative(cwd, abs) || ".");
  const patterns: string[] = [];
  for (const rel of relDirs) {
    const base = rel === "." ? "" : `${rel.replace(/\/+$/, "")}/`;
    for (const ext of REQUIREMENT_FILE_EXTENSIONS) {
      patterns.push(`${base}**/*${ext}`);
    }
  }
  const matches = await glob(patterns, { cwd, ignore: ["**/node_modules/**"], absolute: true });
  return [...new Set(matches)].sort();
}

export async function discoverProject(startDir: string): Promise<DiscoverResult> {
  const candidates = await discoverProjectRootCandidates(startDir);
  if (candidates.length === 0) {
    throw new Error(
      `No project root found (missing ${ROOT_MARKER_HINT}) from: ${path.resolve(startDir)}`
    );
  }
  const rootDir = candidates[0]!;
  const requirementPaths = await discoverRequirementPaths(rootDir);
  return { rootDir, requirementPaths };
}

function categoryPathFor(sourcePath: string, requirementDirs: string[]): string[] {
  const fileDir = path.dirname(sourcePath);
  const normalized = path.normalize(fileDir);
  for (const dir of requirementDirs) {
    const reqDirNorm = path.normalize(path.resolve(dir));
    if (normalized === reqDirNorm || normalized.startsWith(reqDirNorm + path.sep)) {
      const rel = path.relative(reqDirNorm, normalized);
      if (rel === "" || rel === ".") return [];
      return rel.split(path.sep).filter(Boolean);
    }
  }
  return [];
}

export function parseRequirementFile(
  filePath: string
): { requirement: RequirementWithSource } | { error: ValidationError } {
  let raw: string;
  try {
    raw = fs.readFileSync(filePath, "utf-8");
  } catch (err) {
    return { error: { path: filePath, message: String(err) } };
  }
  return parseRequirementContent(raw, filePath);
}

export async function loadRequirements(startDir: string, projectRoot?: string): Promise<LoadResult> {
  let root: string;
  let requirementPaths: string[];
  if (projectRoot != null) {
    root = projectRoot;
    requirementPaths = await discoverRequirementPaths(projectRoot);
  } else {
    const discovered = await discoverProject(startDir);
    root = discovered.rootDir;
    requirementPaths = discovered.requirementPaths;
  }
  const requirementDirs = getRequirementDirs(root);
  const requirements: RequirementWithSource[] = [];
  const errors: ValidationError[] = [];
  for (const filePath of requirementPaths) {
    let raw: string;
    try {
      raw = fs.readFileSync(filePath, "utf-8");
    } catch (err) {
      errors.push({ path: filePath, message: String(err) });
      continue;
    }
    const parsed = parseRequirementDocuments(raw, filePath);
    errors.push(...parsed.errors);
    for (const req of parsed.requirements) {
      req.categoryPath = categoryPathFor(filePath, requirementDirs);
      requirements.push(req);
    }
  }
  errors.push(...validateRequirements(requirements));
  const markerPath = findRootMarkerPath(root);
  if (markerPath !== null) {
    try {
      const raw = fs.readFileSync(markerPath, "utf-8");
      errors.push(...validateCustomTags(raw, markerPath, requirements));
    } catch {
      // An unreadable marker is already reported by discovery.
    }
  }
  return { requirements, errors };
}

export function readRequirementSchemaComposeOptions(
  projectRoot: string
): RequirementSchemaComposeOptions | undefined {
  const rootPath = findRootMarkerPath(path.resolve(projectRoot));
  if (rootPath === null) return undefined;
  let raw: string;
  try {
    raw = fs.readFileSync(rootPath, "utf-8");
  } catch {
    return undefined;
  }
  return customTagComposeOptions(raw);
}

export function loadActiveProfileId(projectRoot: string): string {
  return getActiveProfileId(projectRoot);
}
