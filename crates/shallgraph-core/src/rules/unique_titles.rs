//! GRD-VALID-006: Requirement titles must be unique across the project.

use crate::types::{RequirementWithSource, ValidationError};

#[shallgraph::implements("GRD-VALID-006")]
pub fn unique_titles(requirements: &[RequirementWithSource]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut by_title: Vec<(&str, &RequirementWithSource)> = Vec::new();

    for r in requirements {
        if let Some(existing) = by_title
            .iter()
            .find(|(title, _)| *title == r.title.as_str())
        {
            let message = format!(
                "Duplicate requirement title: {} (also in {})",
                r.title,
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
            by_title.push((r.title.as_str(), r));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Requirement, RequirementWithSource};
    use std::path::PathBuf;

    fn req(id: &str, title: &str, source_path: &str) -> RequirementWithSource {
        RequirementWithSource::from_requirement(
            Requirement {
                id: id.to_string(),
                title: title.to_string(),
                require: "The system shall test.".to_string(),
                refinement: "".into(),
                attributes: None,
                links: None,
                satisfied_by: None,
                verified_by: None,
                parameters: None,
                tags: None,
            },
            PathBuf::from(source_path),
        )
    }

    #[shallgraph::verifies("GRD-VALID-006")]
    #[test]
    fn duplicate_titles_across_files_report_both_paths() {
        let errors = unique_titles(&[
            req("A", "Shared", "/p/A.req.yml"),
            req("B", "Shared", "/p/B.req.yml"),
        ]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/p/B.req.yml");
        assert_eq!(errors[0].line, None);
        assert_eq!(
            errors[0].message,
            "Duplicate requirement title: Shared (also in /p/A.req.yml)"
        );
    }

    #[shallgraph::verifies("GRD-VALID-006")]
    #[test]
    fn duplicate_titles_in_one_file_report_the_file_and_line() {
        let mut first = req("SYS-001", "Shared", "/p/bundle.req.yml");
        first.document_line = Some(1);
        let mut second = req("SYS-002", "Shared", "/p/bundle.req.yml");
        second.document_line = Some(5);
        let errors = unique_titles(&[first, second]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "/p/bundle.req.yml");
        assert_eq!(errors[0].line, Some(5));
        assert_eq!(
            errors[0].message,
            "Duplicate requirement title: Shared (also in /p/bundle.req.yml:1)"
        );
    }

    #[shallgraph::verifies("GRD-VALID-006")]
    #[test]
    fn unique_titles_ok() {
        let errors = unique_titles(&[
            req("A", "Alpha", "/p/A.req.yml"),
            req("B", "Beta", "/p/B.req.yml"),
        ]);
        assert!(errors.is_empty());
    }
}
