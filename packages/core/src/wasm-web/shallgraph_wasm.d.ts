/* Ambient types for the generated wasm-bindgen web glue copied into dist/wasm-web/. */
export function requirementFileExtension(): string;
export function isRequirementFilename(basename: string): boolean;
export function requirementIdFromFilename(basename: string): string | undefined;
export function parseRequirementContent(yaml: string, path: string): string;
export function parseRequirementDocuments(yaml: string, path: string): string;
export function validateRequirements(requirementsJson: string): string;
export function formatRequirementToYaml(requirementJson: string): string;
export function exportRequirementFileJsonSchema(composeJson?: string | null): string;
export function validateCustomTags(markerYaml: string, markerPath: string, requirementsJson: string): string;
export function customTagComposeOptions(markerYaml: string): string;
export function generateSingleRequirementHtml(
  requirementJson: string,
  allJson?: string | null,
  artifactLinksJson?: string | null
): string;
export function parseRootMarker(yaml: string, markerLabel: string): string;
export function standardProfileId(): string;
export function listRegisteredProfileIds(): string;
export function hasRequirementProfile(id: string): boolean;

export default function init(moduleOrPath?: unknown): Promise<unknown>;
