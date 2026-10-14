import type {
  RequirementSchemaComposeOptions,
  RequirementWithSource,
  ValidationError,
} from "./types.js";
import { loadWasmBindings } from "./wasm.js";

export function parseJsonResult<T>(raw: string): T {
  return JSON.parse(raw) as T;
}

export function parseMaybeError(raw: string): { error: ValidationError } | null {
  const trimmed = raw.trimStart();
  if (!trimmed.startsWith("{")) {
    return null;
  }
  let parsed: { error?: ValidationError };
  try {
    parsed = JSON.parse(raw) as { error?: ValidationError };
  } catch {
    return null;
  }
  if (parsed && typeof parsed === "object" && parsed.error && parsed.error.message) {
    return { error: parsed.error };
  }
  return null;
}

export function parseRequirementContent(
  content: string,
  filePath: string
): { requirement: RequirementWithSource } | { error: ValidationError } {
  const raw = loadWasmBindings().parseRequirementContent(content, filePath);
  const err = parseMaybeError(raw);
  if (err) return err;
  return parseJsonResult<{ requirement: RequirementWithSource }>(raw);
}

export function validateRequirements(requirements: RequirementWithSource[]): ValidationError[] {
  const raw = loadWasmBindings().validateRequirements(JSON.stringify(requirements));
  const err = parseMaybeError(raw);
  if (err) return [err.error];
  return parseJsonResult<ValidationError[]>(raw);
}

export function formatRequirementToYaml(requirement: RequirementWithSource | { id: string }): string {
  const raw = loadWasmBindings().formatRequirementToYaml(JSON.stringify(requirement));
  const err = parseMaybeError(raw);
  if (err) throw new Error(err.error.message);
  return parseJsonResult<{ yaml: string }>(raw).yaml;
}

export function validateCustomTags(
  markerYaml: string,
  markerPath: string,
  requirements: RequirementWithSource[]
): ValidationError[] {
  const raw = loadWasmBindings().validateCustomTags(
    markerYaml,
    markerPath,
    JSON.stringify(requirements)
  );
  const err = parseMaybeError(raw);
  if (err) return [err.error];
  return parseJsonResult<ValidationError[]>(raw);
}

export function customTagComposeOptions(
  markerYaml: string
): RequirementSchemaComposeOptions | undefined {
  const raw = loadWasmBindings().customTagComposeOptions(markerYaml);
  if (!raw || raw === "null") return undefined;
  const err = parseMaybeError(raw);
  if (err) return undefined;
  const parsed = parseJsonResult<RequirementSchemaComposeOptions>(raw);
  if (!parsed.tags || parsed.tags.length === 0) return undefined;
  return parsed;
}

export function exportRequirementFileJsonSchema(
  compose?: RequirementSchemaComposeOptions
): Record<string, unknown> {
  const raw = loadWasmBindings().exportRequirementFileJsonSchema(
    compose ? JSON.stringify(compose) : null
  );
  const err = parseMaybeError(raw);
  if (err) throw new Error(err.error.message);
  return parseJsonResult<Record<string, unknown>>(raw);
}

export function generateSingleRequirementHtmlRaw(
  requirement: RequirementWithSource,
  allRequirements?: RequirementWithSource[],
  artifactLinksJson?: string | null
): string {
  const html = loadWasmBindings().generateSingleRequirementHtml(
    JSON.stringify(requirement),
    allRequirements ? JSON.stringify(allRequirements) : null,
    artifactLinksJson ?? null
  );
  const err = parseMaybeError(html);
  if (err) throw new Error(err.error.message);
  return html;
}

export function parseRootMarker(
  yaml: string,
  markerLabel: string
): { profile: string; requirementDirs: string[] } | { error: ValidationError } {
  const raw = loadWasmBindings().parseRootMarker(yaml, markerLabel);
  const err = parseMaybeError(raw);
  if (err) return err;
  return parseJsonResult<{ profile: string; requirementDirs: string[] }>(raw);
}

export function stampEditableFieldMarkers(html: string): string {
  return html
    .replaceAll('<div class="require">', '<div class="require" data-shallgraph-field="require">')
    .replaceAll('<div class="refinement">', '<div class="refinement" data-shallgraph-field="refinement">')
    .replaceAll('<div class="rationale">', '<div class="rationale" data-shallgraph-field="rationale">');
}
