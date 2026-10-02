//! GRD-SYS-010: Default `standard` profile — current requirement schema, ruleset, and HTML report layout.

use std::path::Path;

use crate::artifact_links::ArtifactLinkRenderOptions;
use crate::html::{
    generate_full_html_with_artifact_links, generate_single_requirement_html_with_source_links,
};
use crate::markdown::generate_full_markdown_with_artifact_links;
use crate::parse::parse_requirement_content;
use crate::rules::validate_requirements;
use crate::schema::export_requirement_file_json_schema;
#[cfg(feature = "std-fs")]
use crate::schema_compose::requirement_schema_compose_options_for_project as compose_options;
use crate::types::{
    RequirementSchemaComposeOptions, RequirementWithSource, SourceLink, ValidationError,
};

use super::types::RequirementProfile;

pub struct StandardProfile;

impl RequirementProfile for StandardProfile {
    fn id(&self) -> &str {
        "standard"
    }

    fn parse_requirement_file(
        &self,
        file_path: &Path,
    ) -> Result<RequirementWithSource, ValidationError> {
        #[cfg(feature = "std-fs")]
        {
            crate::parse::parse_requirement_file(file_path)
        }
        #[cfg(not(feature = "std-fs"))]
        {
            let _ = file_path;
            Err(ValidationError::new(
                file_path.display().to_string(),
                "filesystem access is unavailable",
            ))
        }
    }

    fn parse_requirement_content(
        &self,
        content: &str,
        file_path: &Path,
    ) -> Result<RequirementWithSource, ValidationError> {
        parse_requirement_content(content, file_path)
    }

    fn validate_requirements(
        &self,
        requirements: &[RequirementWithSource],
    ) -> Vec<ValidationError> {
        validate_requirements(requirements)
    }

    fn export_requirement_file_json_schema(
        &self,
        options: Option<&RequirementSchemaComposeOptions>,
    ) -> serde_json::Value {
        export_requirement_file_json_schema(options)
    }

    fn requirement_schema_compose_options_for_project(
        &self,
        project_root: &Path,
    ) -> Option<RequirementSchemaComposeOptions> {
        #[cfg(feature = "std-fs")]
        {
            compose_options(project_root)
        }
        #[cfg(not(feature = "std-fs"))]
        {
            let _ = project_root;
            None
        }
    }

    fn generate_full_html(
        &self,
        requirements: &[RequirementWithSource],
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String {
        generate_full_html_with_artifact_links(requirements, source_links, artifact_links)
    }

    fn generate_full_markdown(
        &self,
        requirements: &[RequirementWithSource],
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String {
        generate_full_markdown_with_artifact_links(requirements, source_links, artifact_links)
    }

    fn generate_single_requirement_html(
        &self,
        requirement: &RequirementWithSource,
        all_requirements: Option<&[RequirementWithSource]>,
        source_links: &[SourceLink],
        artifact_links: Option<&ArtifactLinkRenderOptions>,
    ) -> String {
        generate_single_requirement_html_with_source_links(
            requirement,
            all_requirements,
            source_links,
            artifact_links,
        )
    }
}
