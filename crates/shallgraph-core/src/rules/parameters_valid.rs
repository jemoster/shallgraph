//! GRD-SYS-005: Validate that parameter names are unique within the requirement
//! and do not overlap with any other fields in the requirement.

use crate::types::{RequirementWithSource, ValidationError};

const TOP_LEVEL_KEYS: &[&str] = &[
    "id",
    "title",
    "require",
    "refinement",
    "attributes",
    "links",
    "satisfied_by",
    "verified_by",
    "parameters",
    "tags",
];

pub fn parameters_valid(requirement: &RequirementWithSource) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let Some(params) = &requirement.parameters else {
        return errors;
    };

    for name in params.keys() {
        if TOP_LEVEL_KEYS.contains(&name.as_str()) {
            errors.push(ValidationError::new(
                requirement.source_path.display().to_string(),
                format!(
                    "GRD-SYS-005: Parameter name must not overlap with requirement field: {name}"
                ),
            ));
        }
    }
    errors
}
