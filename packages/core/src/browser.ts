/**
 * Browser entry for `@shallgraph/core`. Omits the Node filesystem adapter.
 * Selected via the package `"browser"` export condition.
 */
export {
  initShallgraphWasm,
  loadWasmBindings,
  generateSingleRequirementHtml,
  stampEditableFieldMarkers,
  type SingleRequirementHtmlOptions,
} from "./html.js";

export {
  parseRequirementContent,
  parseRequirementDocuments,
  validateRequirements,
  formatRequirementToYaml,
  exportRequirementFileJsonSchema,
  generateSingleRequirementHtmlRaw,
  parseRootMarker,
} from "./engine.js";

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

export type {
  ArtifactLinkRenderOptions,
  ArtifactRef,
  DiscoverResult,
  Link,
  LoadResult,
  ParameterValue,
  Requirement,
  RequirementSchemaComposeOptions,
  RequirementWithSource,
  ValidationError,
} from "./types.js";
