/**
 * GRD-VSC-002: Link targets under `links` in a requirement file.
 * The ranges cover the target text the user clicks (quotes excluded).
 */
import { isMap, isScalar, isSeq, parseAllDocuments, type Scalar } from "yaml";

export interface RequirementLinkTarget {
  /** Start offset of the target text in the file. */
  start: number;
  /** End offset of the target text in the file. */
  end: number;
  /** Unquoted target value. */
  value: string;
}

const SATISFIES_LINE = /satisfies:\s*['"]?([^\s'"\n]+)['"]?/g;

/**
 * GRD-VSC-002: Every string link target in requirement YAML, including `satisfies`
 * and any other key under `links`.
 */
export function findRequirementLinkTargets(text: string): RequirementLinkTarget[] {
  const targets: RequirementLinkTarget[] = [];
  let hadError = false;
  let sawDocument = false;
  for (const doc of parseAllDocuments(text)) {
    sawDocument = true;
    if (doc.errors.length > 0) hadError = true;
    collectFromContents(text, doc.contents, targets);
  }
  if (targets.length === 0 && (hadError || !sawDocument)) {
    collectSatisfiesFallback(text, targets);
  }
  return targets;
}

/**
 * Line of `id: <requirementId>` in a requirement file. Zero when the id is not a document id.
 */
export function requirementLineInFile(fileText: string, requirementId: string): number {
  const escaped = requirementId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`^\\s*id:\\s*['"]?${escaped}['"]?\\s*(?:#.*)?$`);
  const lines = fileText.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    if (re.test(lines[i] ?? "")) return i;
  }
  return 0;
}

function collectFromContents(text: string, contents: unknown, out: RequirementLinkTarget[]): void {
  if (!isMap(contents)) return;
  const links = contents.get("links", true);
  if (links == null) return;
  if (isSeq(links)) {
    for (const item of links.items) {
      if (isMap(item)) collectLinkMap(text, item, out);
    }
    return;
  }
  if (isMap(links)) collectLinkMap(text, links, out);
}

function collectLinkMap(
  text: string,
  map: { items: Array<{ value: unknown }> },
  out: RequirementLinkTarget[]
): void {
  for (const pair of map.items) {
    if (isScalar(pair.value)) addScalar(text, pair.value, out);
  }
}

function addScalar(text: string, scalar: Scalar, out: RequirementLinkTarget[]): void {
  if (!scalar.range) return;
  const value = scalarText(scalar.value);
  if (!value) return;
  const rawStart = scalar.range[0];
  const rawEnd = scalar.range[1];
  const raw = text.slice(rawStart, rawEnd);
  const at = raw.indexOf(value);
  if (at >= 0) {
    out.push({ start: rawStart + at, end: rawStart + at + value.length, value });
    return;
  }
  out.push({ start: rawStart, end: rawEnd, value });
}

function scalarText(value: unknown): string | undefined {
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : undefined;
  }
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return undefined;
}

/** When the document does not parse, still follow `satisfies:` values. */
function collectSatisfiesFallback(text: string, out: RequirementLinkTarget[]): void {
  for (const match of text.matchAll(SATISFIES_LINE)) {
    const value = match[1];
    if (!value || match.index === undefined) continue;
    const start = match.index + match[0].indexOf(value);
    out.push({ start, end: start + value.length, value });
  }
}
