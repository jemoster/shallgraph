//! Default validation rules provided by the base ruleset (GRD-VALID-001).

mod filename_id_match;
mod links_acyclic;
mod parameters_valid;
mod require_valid;
mod satisfies_references_exist;
mod types;
mod unique_ids;

pub use filename_id_match::filename_id_match;
pub use links_acyclic::links_acyclic;
pub use parameters_valid::parameters_valid;
pub use require_valid::require_valid;
pub use satisfies_references_exist::satisfies_references_exist;
pub use types::{GlobalValidationRule, NamedGlobalRule, NamedRule, ValidationRule};
pub use unique_ids::unique_ids;

use crate::types::{RequirementWithSource, ValidationError};

pub fn default_rules() -> Vec<NamedRule> {
    vec![
        NamedRule {
            id: "GRD-VALID-002",
            run: filename_id_match,
        },
        NamedRule {
            id: "GRD-SYS-005",
            run: parameters_valid,
        },
        NamedRule {
            id: "GRD-SYS-015",
            run: require_valid,
        },
    ]
}

pub fn default_global_rules() -> Vec<NamedGlobalRule> {
    vec![
        NamedGlobalRule {
            id: "GRD-VALID-003",
            run: unique_ids,
        },
        NamedGlobalRule {
            id: "GRD-VALID-004",
            run: satisfies_references_exist,
        },
        NamedGlobalRule {
            id: "GRD-VALID-005",
            run: links_acyclic,
        },
    ]
}

pub fn run_rules(
    requirements: &[RequirementWithSource],
    rules: &[NamedRule],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for requirement in requirements {
        for rule in rules {
            errors.extend((rule.run)(requirement));
        }
    }
    errors
}

pub fn run_global_rules(
    requirements: &[RequirementWithSource],
    rules: &[NamedGlobalRule],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for rule in rules {
        errors.extend((rule.run)(requirements));
    }
    errors
}

/// Validate a list of requirements (GRD-VALID-003 unique ids, GRD-VALID-004
/// link references exist, GRD-VALID-005 links acyclic, and ruleset rules such
/// as filename-id match). Returns a list of validation errors; empty if valid.
/// GRD-SYS-010: The active profile delegates here for the standard ruleset.
pub fn validate_requirements(requirements: &[RequirementWithSource]) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    errors.extend(run_global_rules(requirements, &default_global_rules()));
    errors.extend(run_rules(requirements, &default_rules()));
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Link, Requirement, RequirementWithSource};
    use indexmap::IndexMap;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn req(id: &str, source_path: &str) -> RequirementWithSource {
        RequirementWithSource::from_requirement(
            Requirement {
                id: id.to_string(),
                title: "Test".to_string(),
                require: "The system shall test.".to_string(),
                refinement: String::new(),
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

    #[test]
    fn default_rules_include_filename_match() {
        let ids: Vec<_> = default_rules().into_iter().map(|r| r.id).collect();
        assert!(ids.contains(&"GRD-VALID-002"));
    }

    #[test]
    fn default_global_rules_include_unique_links_acyclic() {
        let ids: Vec<_> = default_global_rules().into_iter().map(|r| r.id).collect();
        assert!(ids.contains(&"GRD-VALID-003"));
        assert!(ids.contains(&"GRD-VALID-004"));
        assert!(ids.contains(&"GRD-VALID-005"));
    }

    #[test]
    fn duplicate_ids() {
        let requirements = vec![
            req("SAME-ID", "/p/SAME-ID.req.yml"),
            req("SAME-ID", "/p/other/SAME-ID.req.yml"),
        ];
        let errors = run_global_rules(&requirements, &default_global_rules());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Duplicate requirement id"));
        assert!(errors[0].message.contains("SAME-ID"));
    }

    #[test]
    fn unique_ids_ok() {
        let requirements = vec![req("A", "/p/A.req.yml"), req("B", "/p/B.req.yml")];
        let errors = run_global_rules(&requirements, &default_global_rules());
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_satisfies() {
        let mut b = req("B", "/p/B.req.yml");
        b.links = Some(vec![Link {
            satisfies: Some("MISSING-ID".into()),
            extra: BTreeMap::new(),
        }]);
        let requirements = vec![req("A", "/p/A.req.yml"), b];
        let errors = run_global_rules(&requirements, &default_global_rules());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("references unknown id"));
        assert!(errors[0].message.contains("MISSING-ID"));
    }

    #[test]
    fn existing_satisfies_ok() {
        let mut b = req("B", "/p/B.req.yml");
        b.links = Some(vec![Link {
            satisfies: Some("A".into()),
            extra: BTreeMap::new(),
        }]);
        let requirements = vec![req("A", "/p/A.req.yml"), b];
        let errors = run_global_rules(&requirements, &default_global_rules());
        assert!(errors.is_empty());
    }

    #[test]
    fn acyclic_ok() {
        let mut b = req("B", "/p/B.req.yml");
        b.links = Some(vec![Link {
            satisfies: Some("A".into()),
            extra: BTreeMap::new(),
        }]);
        let mut c = req("C", "/p/C.req.yml");
        c.links = Some(vec![Link {
            satisfies: Some("B".into()),
            extra: BTreeMap::new(),
        }]);
        let requirements = vec![req("A", "/p/A.req.yml"), b, c];
        let errors = run_global_rules(&requirements, &default_global_rules());
        assert!(errors.is_empty());
    }

    #[test]
    fn self_cycle() {
        let mut a = req("A", "/p/A.req.yml");
        a.links = Some(vec![Link {
            satisfies: Some("A".into()),
            extra: BTreeMap::new(),
        }]);
        let errors = run_global_rules(&[a], &default_global_rules());
        let cycle: Vec<_> = errors
            .into_iter()
            .filter(|e| e.message.contains("Cycle in requirement links"))
            .collect();
        assert_eq!(cycle.len(), 1);
        assert_eq!(cycle[0].message, "Cycle in requirement links: A -> A");
        assert_eq!(cycle[0].path, "/p/A.req.yml");
    }

    #[test]
    fn two_cycle() {
        let mut a = req("A", "/p/A.req.yml");
        a.links = Some(vec![Link {
            satisfies: Some("B".into()),
            extra: BTreeMap::new(),
        }]);
        let mut b = req("B", "/p/B.req.yml");
        b.links = Some(vec![Link {
            satisfies: Some("A".into()),
            extra: BTreeMap::new(),
        }]);
        let errors = run_global_rules(&[a, b], &default_global_rules());
        let cycle: Vec<_> = errors
            .into_iter()
            .filter(|e| e.message.contains("Cycle in requirement links"))
            .collect();
        assert_eq!(cycle.len(), 1);
        assert!(
            cycle[0].message == "Cycle in requirement links: A -> B -> A"
                || cycle[0].message == "Cycle in requirement links: B -> A -> B"
        );
    }

    #[test]
    fn filename_id_errors() {
        let requirements = vec![req("GRD-VALID-001", "/p/GRD-VALID-002.req.yml")];
        let errors = run_rules(&requirements, &default_rules());
        assert!(errors
            .iter()
            .any(|e| e.message.contains("does not match filename")));
    }

    #[test]
    fn filename_id_ok() {
        let requirements = vec![
            req("GRD-VALID-001", "/p/GRD-VALID-001.req.yml"),
            req("GRD-VALID-002", "/p/GRD-VALID-002.req.yml"),
        ];
        let errors = run_rules(&requirements, &default_rules());
        assert!(errors.is_empty());
    }

    #[test]
    fn require_valid_cases() {
        let mut ok = req("X-001", "/p/X-001.req.yml");
        ok.require = "The system shall do X.".into();
        assert!(require_valid(&ok).is_empty());

        let mut empty = req("X-001", "/p/X-001.req.yml");
        empty.require = String::new();
        assert_eq!(require_valid(&empty).len(), 1);

        let mut none = req("X-001", "/p/X-001.req.yml");
        none.require = "The system must do X.".into();
        assert!(require_valid(&none)[0]
            .message
            .contains("shall, should, or may"));

        let mut many = req("X-001", "/p/X-001.req.yml");
        many.require = "The system shall do X and may do Y.".into();
        assert!(require_valid(&many)[0].message.contains("single statement"));
    }

    #[test]
    fn parameters_valid_overlap() {
        let mut r = req("R", "/r.req.yml");
        r.parameters = Some(IndexMap::from([(
            "id".into(),
            crate::types::ParameterValue::String("value".into()),
        )]));
        let errors = parameters_valid(&r);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("GRD-SYS-005"));
        assert!(errors[0].message.contains("id"));
        assert!(errors[0].message.contains("overlap"));
    }
}
