//! Extra filename-id-match tests (GRD-VALID-002).

use shallgraph_core::{
    expected_requirement_basenames_for_id, filename_id_match, requirement_file_extensions_display,
    Requirement, RequirementWithSource, REQUIREMENT_FILE_EXTENSION,
};
use std::path::PathBuf;

fn req(id: &str, source_path: &str) -> RequirementWithSource {
    RequirementWithSource::from_requirement(
        Requirement {
            id: id.to_string(),
            title: "Test".into(),
            require: "The system shall test.".into(),
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
fn matches_yml_and_yaml() {
    let r = req(
        "GRD-VALID-002",
        &format!("/project/requirements/GRD-VALID-002{REQUIREMENT_FILE_EXTENSION}"),
    );
    assert!(filename_id_match(&r).is_empty());
    let r = req(
        "GRD-VALID-002",
        "/project/requirements/GRD-VALID-002.req.yaml",
    );
    assert!(filename_id_match(&r).is_empty());
}

#[test]
fn mismatch_and_wrong_extension() {
    let bad = format!("GRD-VALID-002{REQUIREMENT_FILE_EXTENSION}");
    let r = req("GRD-VALID-001", &format!("/project/requirements/{bad}"));
    let errors = filename_id_match(&r);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("GRD-VALID-001"));
    assert!(errors[0]
        .message
        .contains(&expected_requirement_basenames_for_id("GRD-VALID-001")));

    let r = req("GRD-VALID-002", "/project/requirements/GRD-VALID-002.json");
    let errors = filename_id_match(&r);
    assert!(errors[0]
        .message
        .contains(&requirement_file_extensions_display()));
}

#[test]
fn multi_document_file_skips_id_match_and_still_checks_extension() {
    let mut r = req("SYS-001", "/project/requirements/bundle.req.yml");
    r.multi_document = true;
    assert!(filename_id_match(&r).is_empty());

    let mut bad = req("SYS-001", "/project/requirements/bundle.json");
    bad.multi_document = true;
    let errors = filename_id_match(&bad);
    assert_eq!(errors.len(), 1);
    assert!(errors[0]
        .message
        .contains(&requirement_file_extensions_display()));
}

#[test]
fn leading_document_marker_still_requires_filename_match() {
    let r = req("SYS-001", "/project/requirements/OTHER.req.yml");
    assert!(!r.multi_document);
    assert!(filename_id_match(&r)
        .iter()
        .any(|e| e.message.contains("does not match filename")));
}
