/**
 * JS facade over shallgraph-core WASM. Node filesystem helpers stay in this package
 * so desktop VS Code can load projects without a web-extension rewrite.
 */
export { initShallgraphWasm, loadWasmBindings } from "./wasm.js";

export {
  ROOT_MARKER,
  ROOT_MARKER_FILENAMES,
  ROOT_MARKER_HINT,
  REQUIREMENT_FILE_EXTENSION,
  REQUIREMENT_FILE_EXTENSIONS,
  STANDARD_PROFILE_ID,
  isRequirementFilename,
  requirementIdFromFilename,
  requirementFileExtensionsDisplay,
  expectedRequirementBasenamesForId,
} from "./constants.js";

export {
  discoverProject,
  discoverProjectRoot,
  discoverProjectRootCandidates,
  discoverRequirementPaths,
  findRootMarkerPath,
  getRequirementDirs,
  getActiveProfileId,
  loadRequirements,
  parseRequirementFile,
  readRequirementSchemaComposeOptions,
} from "./fs-adapter.js";

export {
  parseRequirementContent,
  parseRequirementDocuments,
  validateRequirements,
  validateCustomTags,
  customTagComposeOptions,
  formatRequirementToYaml,
  exportRequirementFileJsonSchema,
} from "./engine.js";

export {
  generateSingleRequirementHtml,
  type SingleRequirementHtmlOptions,
} from "./html.js";

export {
  getRequirementProfile,
  listRegisteredProfileIds,
  loadActiveProfile,
  type RequirementProfile,
} from "./profile.js";

export type {
  ArtifactLinkRenderOptions,
  ArtifactRef,
  DiscoverResult,
  Link,
  LoadResult,
  ParameterValue,
  Requirement,
  CustomTagDefinition,
  RequirementSchemaComposeOptions,
  RequirementWithSource,
  ValidationError,
} from "./types.js";
