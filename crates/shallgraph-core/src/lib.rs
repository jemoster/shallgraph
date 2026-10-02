//! Core API for shallgraph. Used by the CLI (GRD-CLI-008).

extern crate shallgraph_macros as shallgraph;

pub mod artifact_links;
pub mod discovery;
pub mod error;
pub mod format;
pub mod html;
pub mod load;
pub mod markdown;
pub mod parameters;
pub mod parse;
pub mod profile;
pub mod requirement_files;
pub mod rules;
pub mod schema;
pub mod schema_compose;
#[cfg(feature = "source-links")]
pub mod source_links;
pub mod types;

pub use artifact_links::{
    github_blob_url_for_artifact, github_blob_url_for_path, github_project_root_rel,
    is_github_host, parse_github_origin_url, posix_join_repo_path, project_relative_file_path,
    ArtifactLinkRenderOptions, GithubArtifactLinkContext, GithubOrigin,
};
#[cfg(feature = "std-fs")]
pub use discovery::{
    discover_project, discover_project_root, discover_project_root_candidates,
    discover_requirement_paths, find_root_marker_path, get_requirement_dirs,
    read_root_marker_mapping,
};
pub use discovery::{
    normalize_path, parse_root_marker_yaml, RootMarkerConfig, ROOT_MARKER, ROOT_MARKER_FILENAMES,
    ROOT_MARKER_HINT,
};
pub use error::{DiscoverResult, Error};
#[cfg(feature = "std-fs")]
pub use format::{format_project_requirement_files, FormatProjectResult};
pub use format::{format_requirement_to_yaml, normalize_requirement_file_text_for_compare};
pub use html::{
    generate_full_html, generate_full_html_with_artifact_links,
    generate_full_html_with_source_links, generate_single_requirement_html,
    generate_single_requirement_html_with_source_links,
};
pub use load::get_requirements_with_links;
#[cfg(feature = "std-fs")]
pub use load::load_requirements;
pub use markdown::{
    generate_full_markdown, generate_full_markdown_with_artifact_links,
    generate_full_markdown_with_source_links,
};
pub use parameters::{resolve_text, resolve_to_segments, ResolvedSegment, SegmentKind};
#[cfg(feature = "std-fs")]
pub use parse::parse_requirement_file;
pub use parse::{parse_requirement_content, parse_requirement_data};
#[cfg(feature = "std-fs")]
pub use profile::{get_active_profile_id, load_active_profile};
pub use profile::{
    get_requirement_profile, list_registered_profile_ids, RequirementProfile, STANDARD_PROFILE_ID,
};
pub use requirement_files::{
    expected_requirement_basenames_for_id, is_requirement_filename,
    requirement_file_extensions_display, requirement_id_from_filename, REQUIREMENT_FILE_EXTENSION,
    REQUIREMENT_FILE_EXTENSIONS,
};
pub use rules::{
    default_global_rules, default_rules, filename_id_match, links_acyclic, parameters_valid,
    require_valid, run_global_rules, run_rules, satisfies_references_exist, unique_ids,
    validate_requirements, NamedGlobalRule, NamedRule, ValidationRule,
};
pub use schema::{export_requirement_file_json_schema, parse_requirement_value};
#[cfg(feature = "std-fs")]
pub use schema_compose::requirement_schema_compose_options_for_project;
#[cfg(feature = "source-links")]
pub use source_links::collect_rust_source_links;
pub use types::{
    ArtifactRef, Link, LoadResult, ParameterValue, ProjectInfo, Requirement,
    RequirementSchemaComposeOptions, RequirementWithSource, SourceLink, SourceLinkKind,
    ValidationError,
};
