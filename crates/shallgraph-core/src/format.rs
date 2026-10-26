//! GRD-SYS-011: Canonical YAML serialization for requirement files.
//! GRD-CLI-006: Project-wide format uses the same serialization and skip-write rules.

#[cfg(feature = "std-fs")]
use std::path::PathBuf;

use indexmap::IndexMap;
use regex::Regex;
use serde_yaml::{Mapping, Value};

use crate::types::{ArtifactRef, Claim, Link, ParameterValue, Refinement, Requirement};

#[cfg(feature = "std-fs")]
use crate::types::{RequirementWithSource, ValidationError};

#[cfg(feature = "std-fs")]
use crate::discovery::{
    discover_project_root_candidates, discover_requirement_paths, ROOT_MARKER_HINT,
};
#[cfg(feature = "std-fs")]
use crate::profile::load_active_profile;
#[cfg(feature = "std-fs")]
use std::fs;
#[cfg(feature = "std-fs")]
use std::path::Path;

/// Result of formatting every discovered requirement file in a project (GRD-CLI-006).
#[cfg(feature = "std-fs")]
#[derive(Debug, Clone, Default)]
pub struct FormatProjectResult {
    pub success: bool,
    pub errors: Vec<ValidationError>,
    pub written_paths: Vec<PathBuf>,
    pub skipped_paths: Vec<PathBuf>,
}

fn mapping_insert(map: &mut Mapping, key: &str, value: Value) {
    map.insert(Value::String(key.to_string()), value);
}

fn json_to_yaml(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                Value::Number(serde_yaml::Number::from(f))
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => {
            Value::Sequence(items.iter().map(json_to_yaml).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut map = Mapping::new();
            for k in keys {
                mapping_insert(&mut map, k, json_to_yaml(&obj[k]));
            }
            Value::Mapping(map)
        }
    }
}

fn sort_json_keys_deep(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(sort_json_keys_deep).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                out.insert(k.clone(), sort_json_keys_deep(&obj[k]));
            }
            serde_json::Value::Object(out)
        }
        other => other.clone(),
    }
}

fn attributes_to_yaml(attrs: &IndexMap<String, serde_json::Value>) -> Option<Value> {
    if attrs.is_empty() {
        return None;
    }
    let mut keys: Vec<&String> = attrs.keys().collect();
    keys.sort();
    let mut map = Mapping::new();
    for k in keys {
        mapping_insert(&mut map, k, json_to_yaml(&sort_json_keys_deep(&attrs[k])));
    }
    Some(Value::Mapping(map))
}

fn parameter_to_yaml(value: &ParameterValue) -> Value {
    match value {
        ParameterValue::String(s) => Value::String(s.clone()),
        ParameterValue::Integer(n) => Value::Number((*n).into()),
        ParameterValue::Float(n) => Value::Number(serde_yaml::Number::from(*n)),
        ParameterValue::Bool(b) => Value::Bool(*b),
    }
}

fn tags_to_yaml(tags: &IndexMap<String, serde_json::Value>) -> Value {
    let mut map = Mapping::new();
    for (key, value) in tags {
        mapping_insert(&mut map, key, json_to_yaml(value));
    }
    Value::Mapping(map)
}

fn parameters_to_yaml(params: &IndexMap<String, ParameterValue>) -> Option<Value> {
    if params.is_empty() {
        return None;
    }
    let mut keys: Vec<&String> = params.keys().collect();
    keys.sort();
    let mut map = Mapping::new();
    for k in keys {
        mapping_insert(&mut map, k, parameter_to_yaml(&params[k]));
    }
    Some(Value::Mapping(map))
}

/// GRD-SYS-016: artifact first, then description.
fn artifact_ref_to_yaml(r: &ArtifactRef) -> Value {
    let mut map = Mapping::new();
    mapping_insert(&mut map, "artifact", Value::String(r.artifact.clone()));
    if let Some(desc) = &r.description {
        if !desc.is_empty() {
            mapping_insert(&mut map, "description", Value::String(desc.clone()));
        }
    }
    Value::Mapping(map)
}

fn artifact_refs_to_yaml(refs: &[ArtifactRef]) -> Option<Value> {
    if refs.is_empty() {
        return None;
    }
    Some(Value::Sequence(
        refs.iter().map(artifact_ref_to_yaml).collect(),
    ))
}

/// Satisfies first, then other keys alphabetically.
fn link_to_yaml(link: &Link) -> Value {
    let mut map = Mapping::new();
    if let Some(satisfies) = &link.satisfies {
        mapping_insert(&mut map, "satisfies", Value::String(satisfies.clone()));
    }
    let mut extra_keys: Vec<&String> = link
        .extra
        .keys()
        .filter(|k| k.as_str() != "satisfies")
        .collect();
    extra_keys.sort();
    for k in extra_keys {
        if let Some(v) = link.extra.get(k) {
            mapping_insert(&mut map, k, json_to_yaml(v));
        }
    }
    Value::Mapping(map)
}

fn links_to_yaml(links: &[Link]) -> Option<Value> {
    if links.is_empty() {
        return None;
    }
    Some(Value::Sequence(links.iter().map(link_to_yaml).collect()))
}

/// Use block clip chomping (`|`) instead of strip (`|-`) for refinement, rationale, and claim text.
fn prefer_clip_block_chomp_for_markdown_keys(yaml: &str) -> String {
    let re = Regex::new(r"(?m)^([ \t]*(?:refinement|rationale|text):[ \t]*)\|-(\r?\n)").unwrap();
    re.replace_all(yaml, "$1|$2").into_owned()
}

/// GRD-SYS-021: Serialize prose as a string and a claim list as a sequence.
/// Empty prose is omitted. An empty list is kept so a list is not rewritten into prose.
fn refinement_to_yaml(refinement: &Refinement) -> Option<Value> {
    match refinement {
        Refinement::Prose(text) if text.is_empty() => None,
        Refinement::Prose(text) => Some(Value::String(text.clone())),
        Refinement::Claims(claims) => {
            Some(Value::Sequence(claims.iter().map(claim_to_yaml).collect()))
        }
    }
}

fn claim_to_yaml(claim: &Claim) -> Value {
    if claim.refinement.is_empty_prose() {
        return Value::String(claim.text.clone());
    }
    let mut map = Mapping::new();
    mapping_insert(&mut map, "text", Value::String(claim.text.clone()));
    if let Some(nested) = refinement_to_yaml(&claim.refinement) {
        mapping_insert(&mut map, "refinement", nested);
    }
    Value::Mapping(map)
}

fn stringify_yaml_value(value: &Value) -> String {
    let mut s = serde_yaml::to_string(value).unwrap_or_else(|_| String::new());
    if let Some(rest) = s.strip_prefix("---\n") {
        s = rest.to_string();
    }
    if let Some(rest) = s.strip_suffix("...\n") {
        s = rest.to_string();
    }
    if !s.ends_with('\n') {
        s.push('\n');
    }
    prefer_clip_block_chomp_for_markdown_keys(&s)
}

/// Normalize text so two requirement files that differ only by line endings or trailing
/// whitespace compare equal (GRD-SYS-011 / GRD-CLI-006 skip-write check).
pub fn normalize_requirement_file_text_for_compare(text: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = unified
        .split('\n')
        .map(|line| line.trim_end_matches([' ', '\t']).to_string())
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// GRD-SYS-020: Canonical YAML for every requirement document, kept in one file.
/// One requirement is serialized the same way as [`format_requirement_to_yaml`].
/// Several requirements are separated by a `---` line.
#[shallgraph::implements("GRD-SYS-020")]
pub fn format_requirement_documents_to_yaml(requirements: &[Requirement]) -> String {
    let mut out = String::new();
    for (index, requirement) in requirements.iter().enumerate() {
        if index > 0 {
            out.push_str("---\n");
        }
        out.push_str(&format_requirement_to_yaml(requirement));
    }
    out
}

/// Serialize a validated requirement to canonical YAML. Omits empty optional sections.
#[shallgraph::implements("GRD-SYS-011", "GRD-SYS-021")]
pub fn format_requirement_to_yaml(requirement: &Requirement) -> String {
    let mut map = Mapping::new();
    mapping_insert(&mut map, "id", Value::String(requirement.id.clone()));
    mapping_insert(&mut map, "title", Value::String(requirement.title.clone()));
    mapping_insert(
        &mut map,
        "require",
        Value::String(requirement.require.clone()),
    );
    if let Some(refinement) = refinement_to_yaml(&requirement.refinement) {
        mapping_insert(&mut map, "refinement", refinement);
    }
    if let Some(attrs) = &requirement.attributes {
        if let Some(yaml) = attributes_to_yaml(attrs) {
            mapping_insert(&mut map, "attributes", yaml);
        }
    }
    // GRD-SYS-019: preserve custom tag entries, including an empty mapping.
    if let Some(tags) = &requirement.tags {
        mapping_insert(&mut map, "tags", tags_to_yaml(tags));
    }
    if let Some(refs) = &requirement.satisfied_by {
        if let Some(yaml) = artifact_refs_to_yaml(refs) {
            mapping_insert(&mut map, "satisfied_by", yaml);
        }
    }
    if let Some(refs) = &requirement.verified_by {
        if let Some(yaml) = artifact_refs_to_yaml(refs) {
            mapping_insert(&mut map, "verified_by", yaml);
        }
    }
    if let Some(links) = &requirement.links {
        if let Some(yaml) = links_to_yaml(links) {
            mapping_insert(&mut map, "links", yaml);
        }
    }
    if let Some(params) = &requirement.parameters {
        if let Some(yaml) = parameters_to_yaml(params) {
            mapping_insert(&mut map, "parameters", yaml);
        }
    }
    stringify_yaml_value(&Value::Mapping(map))
}

/// GRD-CLI-006: Format every discovered requirement YAML file in a project using the active
/// profile parser and GRD-SYS-011 canonical serialization.
#[cfg(feature = "std-fs")]
#[shallgraph::implements("GRD-CLI-006")]
pub fn format_project_requirement_files(project_dir: &Path) -> FormatProjectResult {
    let candidates = match discover_project_root_candidates(project_dir) {
        Ok(c) => c,
        Err(err) => {
            return FormatProjectResult {
                success: false,
                errors: vec![ValidationError::new(
                    project_dir.display().to_string(),
                    err.to_string(),
                )],
                written_paths: Vec::new(),
                skipped_paths: Vec::new(),
            };
        }
    };
    if candidates.is_empty() {
        return FormatProjectResult {
            success: false,
            errors: vec![ValidationError::new(
                project_dir.display().to_string(),
                format!("No project root found (missing {ROOT_MARKER_HINT})"),
            )],
            written_paths: Vec::new(),
            skipped_paths: Vec::new(),
        };
    }

    let root = &candidates[0];
    let paths = match discover_requirement_paths(root) {
        Ok(p) => p,
        Err(err) => {
            return FormatProjectResult {
                success: false,
                errors: vec![ValidationError::new(
                    root.display().to_string(),
                    err.to_string(),
                )],
                written_paths: Vec::new(),
                skipped_paths: Vec::new(),
            };
        }
    };
    let profile = match load_active_profile(root) {
        Ok(p) => p,
        Err(err) => {
            return FormatProjectResult {
                success: false,
                errors: vec![ValidationError::new(
                    root.display().to_string(),
                    err.to_string(),
                )],
                written_paths: Vec::new(),
                skipped_paths: Vec::new(),
            };
        }
    };

    let mut errors: Vec<ValidationError> = Vec::new();
    let mut parsed: Vec<(PathBuf, String, Vec<RequirementWithSource>)> = Vec::new();
    for file_path in paths {
        let raw = match fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(err) => {
                errors.push(ValidationError::new(
                    file_path.display().to_string(),
                    err.to_string(),
                ));
                continue;
            }
        };
        let documents = profile.parse_requirement_documents(&raw, &file_path);
        if !documents.errors.is_empty() {
            errors.extend(documents.errors);
            continue;
        }
        if documents.requirements.is_empty() {
            errors.push(ValidationError::new(
                file_path.display().to_string(),
                "Requirement file contains no requirement document",
            ));
            continue;
        }
        parsed.push((file_path, raw, documents.requirements));
    }

    if !errors.is_empty() {
        return FormatProjectResult {
            success: false,
            errors,
            written_paths: Vec::new(),
            skipped_paths: Vec::new(),
        };
    }

    let mut written_paths = Vec::new();
    let mut skipped_paths = Vec::new();
    for (file_path, raw, requirements) in parsed {
        let docs: Vec<Requirement> = requirements.iter().map(|r| r.to_requirement()).collect();
        let yaml = format_requirement_documents_to_yaml(&docs);
        let norm_raw = normalize_requirement_file_text_for_compare(&raw);
        let norm_yaml = normalize_requirement_file_text_for_compare(&yaml);
        if norm_raw == norm_yaml {
            skipped_paths.push(file_path);
        } else {
            if let Err(err) = fs::write(&file_path, yaml.as_bytes()) {
                return FormatProjectResult {
                    success: false,
                    errors: vec![ValidationError::new(
                        file_path.display().to_string(),
                        err.to_string(),
                    )],
                    written_paths: Vec::new(),
                    skipped_paths: Vec::new(),
                };
            }
            written_paths.push(file_path);
        }
    }

    FormatProjectResult {
        success: true,
        errors: Vec::new(),
        written_paths,
        skipped_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::ROOT_MARKER;
    use crate::parse::parse_requirement_content;
    use crate::requirement_files::REQUIREMENT_FILE_EXTENSION;
    use indexmap::IndexMap;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn req(id: &str, title: &str, require: &str) -> Requirement {
        Requirement {
            id: id.to_string(),
            title: title.to_string(),
            require: require.to_string(),
            refinement: "".into(),
            attributes: None,
            links: None,
            satisfied_by: None,
            verified_by: None,
            parameters: None,
            tags: None,
        }
    }

    fn temp_project(req_body: &str) -> (PathBuf, PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let project_root =
            std::env::temp_dir().join(format!("shallgraph-fmt-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&project_root);
        let reqs = project_root.join("requirements");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(
            project_root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        let file_path = reqs.join(format!("GRD-FMT-001{REQUIREMENT_FILE_EXTENSION}"));
        fs::write(&file_path, req_body).unwrap();
        (project_root, file_path)
    }

    #[shallgraph::verifies("GRD-SYS-011")]
    #[test]
    fn format_is_idempotent_for_parsed_requirements() {
        let yaml = concat!(
            "id: GRD-T-001\n",
            "title: T\n",
            "require: The system shall do one and two.\n",
            "refinement: |-\n",
            "  One\n",
            "  Two\n",
            "attributes:\n",
            "  z: 1\n",
            "  a: 2\n",
            "links:\n",
            "  - satisfies: GRD-X\n",
            "parameters:\n",
            "  beta: true\n",
            "  alpha: 1\n",
        );
        let parsed =
            parse_requirement_content(yaml, Path::new(&format!("/x{REQUIREMENT_FILE_EXTENSION}")))
                .unwrap();
        let once = format_requirement_to_yaml(&parsed.to_requirement());
        let again =
            parse_requirement_content(&once, Path::new(&format!("/x{REQUIREMENT_FILE_EXTENSION}")))
                .unwrap();
        let twice = format_requirement_to_yaml(&again.to_requirement());
        assert_eq!(twice, once);
    }

    #[shallgraph::verifies("GRD-SYS-011")]
    #[test]
    fn uses_block_clip_chomping_for_multiline_markdown() {
        let mut r = req("GRD-T-Pipe", "T", "The system shall do one and two.");
        r.refinement = "One\nTwo".into();
        r.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("A\nB"),
        )]));
        let once = format_requirement_to_yaml(&r);
        let refinement_re = Regex::new(r"(?m)^refinement: \|(\r?\n)").unwrap();
        let rationale_re = Regex::new(r"(?m)^([ \t]*)rationale: \|(\r?\n)").unwrap();
        assert!(refinement_re.is_match(&once), "{once}");
        assert!(!once.contains("refinement: |-"));
        assert!(rationale_re.is_match(&once), "{once}");
        assert!(!once.contains("rationale: |-"));
    }

    #[shallgraph::verifies("GRD-SYS-021")]
    #[test]
    fn preserves_prose_and_claim_list_shapes() {
        let prose = req("GRD-T-Prose", "T", "The system shall keep the string.");
        let mut prose = prose;
        prose.refinement = "* still text".into();
        let prose_yaml = format_requirement_to_yaml(&prose);
        assert!(prose_yaml.contains("refinement:"), "{prose_yaml}");
        assert!(!prose_yaml.contains("\n  - "), "{prose_yaml}");
        let prose_again = parse_requirement_content(
            &prose_yaml,
            Path::new(&format!("/x{REQUIREMENT_FILE_EXTENSION}")),
        )
        .unwrap();
        assert_eq!(prose_again.refinement, "* still text");

        let mut listed = req("GRD-T-List", "T", "The record shall meet every clause,");
        listed.refinement = Refinement::Claims(vec![
            Claim {
                text: "must be complete".into(),
                refinement: Refinement::default(),
            },
            Claim {
                text: "until an operator deletes it.".into(),
                refinement: Refinement::Claims(vec![Claim {
                    text: "The deletion shall be recorded.".into(),
                    refinement: Refinement::default(),
                }]),
            },
        ]);
        let once = format_requirement_to_yaml(&listed);
        assert!(once.contains("- must be complete"), "{once}");
        assert!(once.contains("text:"), "{once}");
        let parsed =
            parse_requirement_content(&once, Path::new(&format!("/x{REQUIREMENT_FILE_EXTENSION}")))
                .unwrap();
        let twice = format_requirement_to_yaml(&parsed.to_requirement());
        assert_eq!(twice, once);

        let mut empty_list = req("GRD-T-Empty", "T", "The system shall keep an empty list.");
        empty_list.refinement = Refinement::Claims(Vec::new());
        let empty_yaml = format_requirement_to_yaml(&empty_list);
        assert!(
            empty_yaml.contains("refinement: []") || empty_yaml.contains("refinement: []\n"),
            "{empty_yaml}"
        );
    }

    #[shallgraph::verifies("GRD-SYS-011")]
    #[test]
    fn orders_top_level_keys() {
        let yaml = format_requirement_to_yaml(&Requirement {
            id: "GRD-T-002".into(),
            title: "Title".into(),
            require: "The system shall do d.".into(),
            refinement: "d".into(),
            attributes: Some(IndexMap::from([(
                "status".into(),
                serde_json::json!("active"),
            )])),
            satisfied_by: Some(vec![ArtifactRef {
                artifact: "src/a.ts".into(),
                description: None,
            }]),
            verified_by: Some(vec![ArtifactRef {
                artifact: "test/a.test.ts".into(),
                description: None,
            }]),
            links: Some(vec![Link {
                satisfies: Some("GRD-A".into()),
                extra: BTreeMap::new(),
            }]),
            parameters: Some(IndexMap::from([("p".into(), ParameterValue::Integer(1))])),
            tags: None,
        });
        let lines: Vec<&str> = yaml.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines[0].starts_with("id:"));
        assert!(lines[1].starts_with("title:"));
        assert!(lines[2].starts_with("require:"));
        assert!(lines[3].starts_with("refinement:"));
        let idx = |prefix: &str| lines.iter().position(|l| l.starts_with(prefix)).unwrap();
        assert!(idx("attributes:") < idx("satisfied_by:"));
        assert!(idx("satisfied_by:") < idx("verified_by:"));
        assert!(idx("verified_by:") < idx("links:"));
        assert!(idx("links:") < idx("parameters:"));
    }

    #[shallgraph::verifies("GRD-SYS-011")]
    #[test]
    fn sorts_attribute_and_parameter_keys() {
        let yaml = format_requirement_to_yaml(&Requirement {
            id: "GRD-T-003".into(),
            title: "T".into(),
            require: "The system shall do x.".into(),
            refinement: "".into(),
            attributes: Some(IndexMap::from([
                ("zebra".into(), serde_json::json!(1)),
                ("apple".into(), serde_json::json!(2)),
            ])),
            links: None,
            satisfied_by: None,
            verified_by: None,
            parameters: Some(IndexMap::from([
                ("z".into(), ParameterValue::Bool(false)),
                ("a".into(), ParameterValue::Bool(true)),
            ])),
            tags: None,
        });
        let a_idx = yaml.find("apple:").unwrap();
        let z_attr = yaml.find("zebra:").unwrap();
        assert!(z_attr > a_idx);
        let a_param = yaml.find("\n  a:").unwrap();
        let z_param = yaml.find("\n  z:").unwrap();
        assert!(z_param > a_param);
    }

    #[shallgraph::verifies("GRD-SYS-011")]
    #[test]
    fn normalize_treats_crlf_and_trailing_whitespace_as_equal() {
        let canonical =
            format_requirement_to_yaml(&req("GRD-T-004", "T", "The system shall do x."));
        let messy = format!("{}  \r\n\r\n", canonical.trim_end().replace('\n', "\r\n"));
        assert_eq!(
            normalize_requirement_file_text_for_compare(&messy),
            normalize_requirement_file_text_for_compare(&canonical)
        );
    }

    #[shallgraph::verifies("GRD-CLI-006")]
    #[test]
    fn project_format_rewrites_then_skips_canonical() {
        let (project_root, file_path) =
            temp_project("title: One\nid: GRD-FMT-001\nrequire: The system shall do x.\n");
        let first = format_project_requirement_files(&project_root);
        assert!(first.success, "{:?}", first.errors);
        assert_eq!(first.written_paths.len(), 1);
        assert!(first.skipped_paths.is_empty());

        let second = format_project_requirement_files(&project_root);
        assert!(second.success);
        assert!(second.written_paths.is_empty());
        assert_eq!(second.skipped_paths, vec![file_path.clone()]);

        let after = fs::read_to_string(&file_path).unwrap();
        assert!(parse_requirement_content(&after, &file_path).is_ok());
        let _ = fs::remove_dir_all(&project_root);
    }

    #[shallgraph::verifies("GRD-CLI-006")]
    #[test]
    fn project_format_writes_nothing_when_parse_fails() {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let project_root =
            std::env::temp_dir().join(format!("shallgraph-fmt-bad-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&project_root);
        let reqs = project_root.join("requirements");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(
            project_root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        let good_path = reqs.join(format!("GRD-GOOD{REQUIREMENT_FILE_EXTENSION}"));
        fs::write(
            &good_path,
            "id: GRD-GOOD\ntitle: Ok\nrequire: The system shall do x.\n",
        )
        .unwrap();
        fs::write(
            reqs.join(format!("GRD-BAD{REQUIREMENT_FILE_EXTENSION}")),
            "not: valid requirement\n",
        )
        .unwrap();
        let before = fs::read_to_string(&good_path).unwrap();
        let result = format_project_requirement_files(&project_root);
        assert!(!result.success);
        assert!(!result.errors.is_empty());
        assert!(result.written_paths.is_empty());
        assert_eq!(fs::read_to_string(&good_path).unwrap(), before);
        let _ = fs::remove_dir_all(&project_root);
    }

    #[shallgraph::verifies("GRD-SYS-020", "GRD-SYS-011")]
    #[test]
    fn project_format_keeps_multiple_documents_in_one_file() {
        let raw = "title: Two\nid: SYS-002\nrequire: The backend shall support concurrent streaming.\n---\ntitle: One\nid: SYS-001\nrequire: The system shall encrypt all video streams.\n";
        let (project_root, file_path) = temp_project(raw);
        let first = format_project_requirement_files(&project_root);
        assert!(first.success, "{:?}", first.errors);
        assert_eq!(first.written_paths.len(), 1);
        let after = fs::read_to_string(&file_path).unwrap();
        assert!(after.contains("\n---\n"));
        let parsed = crate::parse::parse_requirement_documents(&after, &file_path);
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(
            parsed
                .requirements
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["SYS-002", "SYS-001"]
        );
        let second = format_project_requirement_files(&project_root);
        assert!(second.success, "{:?}", second.errors);
        assert!(second.written_paths.is_empty());
        assert_eq!(second.skipped_paths, vec![file_path]);
        let _ = fs::remove_dir_all(&project_root);
    }
}
