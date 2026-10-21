export type ParameterValue = string | number | boolean;

export interface Link {
  satisfies?: string;
  [key: string]: unknown;
}

export interface ArtifactRef {
  artifact: string;
  description?: string;
}

export interface Requirement {
  id: string;
  title: string;
  require: string;
  refinement: string;
  attributes?: Record<string, unknown>;
  links?: Link[];
  satisfied_by?: ArtifactRef[];
  verified_by?: ArtifactRef[];
  parameters?: Record<string, ParameterValue>;
  tags?: Record<string, unknown>;
}

export interface RequirementWithSource extends Requirement {
  sourcePath: string;
  categoryPath?: string[];
  /** True when this requirement came from a file with more than one requirement document. */
  multiDocument?: boolean;
  /** 1-based line where this requirement document starts. */
  documentLine?: number;
}

export interface ValidationError {
  path: string;
  message: string;
  line?: number;
}

export interface LoadResult {
  requirements: RequirementWithSource[];
  errors: ValidationError[];
}

export interface DiscoverResult {
  rootDir: string;
  requirementPaths: string[];
}

export type ArtifactLinkRenderOptions = {
  github?: {
    host?: string;
    owner: string;
    repo: string;
    commitSha: string;
    projectRootRel: string;
    projectRoot?: string;
  };
};

export interface CustomTagDefinition {
  name: string;
  values: string[];
  required: boolean;
  multiple: boolean;
  allowNull: boolean;
  allowEmpty: boolean;
  allowNullEntry: boolean;
  maxEntries?: number | null;
}

export interface RequirementSchemaComposeOptions {
  tags?: CustomTagDefinition[];
}
