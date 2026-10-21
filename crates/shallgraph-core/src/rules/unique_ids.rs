//! GRD-VALID-003: Requirement ids must be unique across the project.

use crate::types::{RequirementWithSource, ValidationError};

pub fn unique_ids(requirements: &[RequirementWithSource]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut by_id: Vec<(&str, &RequirementWithSource)> = Vec::new();

    for r in requirements {
        if let Some(existing) = by_id.iter().find(|(id, _)| *id == r.id.as_str()) {
            let message = format!(
                "Duplicate requirement id: {} (also in {})",
                r.id,
                location(existing.1)
            );
            let err = match r.document_line {
                Some(line) => {
                    ValidationError::with_line(r.source_path.display().to_string(), message, line)
                }
                None => ValidationError::new(r.source_path.display().to_string(), message),
            };
            errors.push(err);
        } else {
            by_id.push((r.id.as_str(), r));
        }
    }

    errors
}

fn location(requirement: &RequirementWithSource) -> String {
    match requirement.document_line {
        Some(line) => format!("{}:{line}", requirement.source_path.display()),
        None => requirement.source_path.display().to_string(),
    }
}
