export interface WasmBindings {
  requirementFileExtension(): string;
  isRequirementFilename(basename: string): boolean;
  requirementIdFromFilename(basename: string): string | undefined;
  parseRequirementContent(yaml: string, path: string): string;
  parseRequirementDocuments(yaml: string, path: string): string;
  validateRequirements(requirementsJson: string): string;
  formatRequirementToYaml(requirementJson: string): string;
  exportRequirementFileJsonSchema(composeJson?: string | null): string;
  validateCustomTags(markerYaml: string, markerPath: string, requirementsJson: string): string;
  customTagComposeOptions(markerYaml: string): string;
  generateSingleRequirementHtml(
    requirementJson: string,
    allJson?: string | null,
    artifactLinksJson?: string | null
  ): string;
  parseRootMarker(yaml: string, markerLabel: string): string;
  standardProfileId(): string;
  listRegisteredProfileIds(): string;
  hasRequirementProfile(id: string): boolean;
}
