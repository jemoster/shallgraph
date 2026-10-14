import type { ArtifactLinkRenderOptions, RequirementSchemaComposeOptions, RequirementWithSource, ValidationError } from "./types.js";
import {
  exportRequirementFileJsonSchema,
  parseRequirementContent,
  validateRequirements,
} from "./engine.js";
import { generateSingleRequirementHtml as renderSingleRequirementHtml } from "./html.js";
import {
  parseRequirementFile,
  getActiveProfileId,
  readRequirementSchemaComposeOptions,
} from "./fs-adapter.js";
import { STANDARD_PROFILE_ID } from "./constants.js";

export interface RequirementProfile {
  readonly id: string;
  parseRequirementFile(filePath: string): { requirement: RequirementWithSource } | { error: ValidationError };
  parseRequirementContent(content: string, filePath: string): { requirement: RequirementWithSource } | { error: ValidationError };
  validateRequirements(requirements: RequirementWithSource[]): ValidationError[];
  exportRequirementFileJsonSchema(options?: RequirementSchemaComposeOptions): Record<string, unknown>;
  requirementSchemaComposeOptionsForProject(projectRoot: string): RequirementSchemaComposeOptions | undefined;
  generateFullHtml(requirements: RequirementWithSource[]): string;
  generateSingleRequirementHtml(
    requirement: RequirementWithSource,
    allRequirements?: RequirementWithSource[],
    options?: { editableFieldMarkers?: boolean; artifactLinks?: ArtifactLinkRenderOptions }
  ): string;
}

const standardProfile: RequirementProfile = {
  id: STANDARD_PROFILE_ID,
  parseRequirementFile,
  parseRequirementContent,
  validateRequirements,
  exportRequirementFileJsonSchema(compose) {
    return exportRequirementFileJsonSchema(compose);
  },
  requirementSchemaComposeOptionsForProject(projectRoot: string) {
    return readRequirementSchemaComposeOptions(projectRoot);
  },
  generateFullHtml() {
    throw new Error("Full-report HTML is provided by the native shallgraph CLI");
  },
  generateSingleRequirementHtml(requirement, allRequirements, options) {
    return renderSingleRequirementHtml(requirement, allRequirements, options);
  },
};

export function getRequirementProfile(id: string): RequirementProfile {
  if (id !== STANDARD_PROFILE_ID) {
    throw new Error(`Unknown requirement profile: ${id}`);
  }
  return standardProfile;
}

export function listRegisteredProfileIds(): string[] {
  return [STANDARD_PROFILE_ID];
}

export function loadActiveProfile(projectRoot: string): RequirementProfile {
  return getRequirementProfile(getActiveProfileId(projectRoot));
}

export { STANDARD_PROFILE_ID };
