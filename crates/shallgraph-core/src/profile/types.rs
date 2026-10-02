//! GRD-SYS-010: Active profile drives requirement document shape, validation, and HTML output.

use crate::artifact_links::ArtifactLinkRenderOptions;
use crate::types::{
    RequirementSchemaComposeOptions, RequirementWithSource, SourceLink, ValidationError,
};
use std::path::Path;

/// GRD-SYS-010: Active profile drives requirement document shape, validation, and HTML output.
pub trait RequirementProfile: Send + Sync {
    fn id(&self) -> &str;
    fn parse_requirement_file(
        &self,
        file_path: &Path,
    ) -> Result<RequirementWithSource, ValidationError>;
    fn parse_requirement_content(
        &self,
        content: &str,
        file_path: &Path,
    ) -> Result<RequirementWithSource, ValidationError>;
    fn validate_requirements(&self, requirements: &[RequirementWithSource])
        -> Vec<ValidationError>;
    fn export_requirement_file_json_schema(
        &self,
        options: Option<&RequirementSchemaComposeOptions>,
    ) -> serde_json::Value;
    fn requirement_schema_compose_options_for_project(
        &self,
        project_root: &Path,
    ) -> Option<RequirementSchemaComposeOptions>;
    /// GRD-SYS-010: Full HTML report. GRD-HTML-008: `artifact_links` may include GitHub blob URLs.
    fn generate_full_html(
        &self,
        requirements: &[RequirementWithSource],
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String;
    /// GRD-MD-001 / GRD-SYS-010: Full Markdown report. GRD-MD-008: `artifact_links` may include GitHub blob URLs.
    fn generate_full_markdown(
        &self,
        requirements: &[RequirementWithSource],
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String;
    /// GRD-VSC-003 / GRD-SYS-010: Single-requirement HTML using the same detail rendering as the full report.
    /// GRD-UI-009: `artifact_links` optionally turns project-relative artifact paths into GitHub blob URLs.
    fn generate_single_requirement_html(
        &self,
        requirement: &RequirementWithSource,
        all_requirements: Option<&[RequirementWithSource]>,
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String;
}
