//! GRD-VALID-002: A single-document requirement file is named after its requirement id.
//! GRD-SYS-020: A file with multiple requirement documents does not have to match any id.

use std::path::Path;

use crate::requirement_files::{
    expected_requirement_basenames_for_id, requirement_file_extensions_display,
    requirement_id_from_filename,
};
use crate::types::{RequirementWithSource, ValidationError};

#[shallgraph::implements("GRD-VALID-002")]
pub fn filename_id_match(requirement: &RequirementWithSource) -> Vec<ValidationError> {
    let basename = Path::new(&requirement.source_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let message = match requirement_id_from_filename(&basename) {
        None => Some(format!(
            "Requirement file must use {} (got \"{basename}\")",
            requirement_file_extensions_display()
        )),
        Some(name_without_ext)
            if !requirement.multi_document && name_without_ext != requirement.id =>
        {
            Some(format!(
                "Requirement id \"{}\" does not match filename \"{basename}\" (expected {})",
                requirement.id,
                expected_requirement_basenames_for_id(&requirement.id)
            ))
        }
        Some(_) => None,
    };
    match message {
        None => Vec::new(),
        Some(message) => vec![match requirement.document_line {
            Some(line) => ValidationError::with_line(
                requirement.source_path.display().to_string(),
                message,
                line,
            ),
            None => ValidationError::new(requirement.source_path.display().to_string(), message),
        }],
    }
}
