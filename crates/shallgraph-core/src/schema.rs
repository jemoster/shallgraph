//! GRD-SYS-009: Requirement YAML schema — single source of truth for structure and validation.
//! GRD-SYS-005: parameters (string | number | boolean).
//! JSON Schema for editors is exported from the same field model.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde_yaml::Value;

use crate::types::{ArtifactRef, Link, ParameterValue, Requirement};

const ALLOWED_KEYS: &[&str] = &[
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

fn mapping_key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn yaml_to_json(v: &Value) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// Coerce a YAML scalar to a trimmed string (GRD-SYS-009: yamlScalar then String).
pub fn yaml_scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        _ => None,
    }
}

fn required_string(obj: &serde_yaml::Mapping, field: &str) -> Result<String, String> {
    match obj.get(mapping_key(field)) {
        None => Err(format!("Missing required field: {field}")),
        Some(v) => match yaml_scalar_to_string(v) {
            Some(s) if !s.is_empty() => Ok(s),
            Some(_) => Err(format!("Missing required field: {field}")),
            None => Err(format!("{field}: expected a string, number, or boolean")),
        },
    }
}

fn optional_refinement(obj: &serde_yaml::Mapping) -> String {
    match obj.get(mapping_key("refinement")) {
        None | Some(Value::Null) => String::new(),
        Some(v) => yaml_scalar_to_string(v).unwrap_or_default(),
    }
}

fn parse_artifact_ref(item: &Value) -> Result<ArtifactRef, String> {
    let mapping = match item {
        Value::Mapping(m) => m,
        _ => return Err("satisfied_by/verified_by entries must be mappings".to_string()),
    };
    let allowed: BTreeSet<&str> = ["artifact", "description"].into_iter().collect();
    for k in mapping.keys() {
        if let Value::String(ks) = k {
            if !allowed.contains(ks.as_str()) {
                return Err(format!("artifact: unknown key: {ks}"));
            }
        }
    }
    let artifact = match mapping.get(mapping_key("artifact")) {
        None => return Err("Missing required field: artifact".to_string()),
        Some(v) => match yaml_scalar_to_string(v) {
            Some(s) if !s.is_empty() => s,
            _ => return Err("Missing required field: artifact".to_string()),
        },
    };
    let description = match mapping.get(mapping_key("description")) {
        None | Some(Value::Null) => None,
        Some(v) => yaml_scalar_to_string(v).and_then(|s| if s.is_empty() { None } else { Some(s) }),
    };
    Ok(ArtifactRef {
        artifact,
        description,
    })
}

fn parse_artifact_list(value: &Value, field: &str) -> Result<Option<Vec<ArtifactRef>>, String> {
    match value {
        Value::Null => Ok(None),
        Value::Sequence(seq) => {
            let mut out = Vec::new();
            for (i, item) in seq.iter().enumerate() {
                match parse_artifact_ref(item) {
                    Ok(r) => out.push(r),
                    Err(msg) => return Err(format!("{field}[{i}]: {msg}")),
                }
            }
            Ok(if out.is_empty() { None } else { Some(out) })
        }
        _ => Err(format!("{field}: expected a sequence")),
    }
}

fn parse_links(value: &Value) -> Result<Option<Vec<Link>>, String> {
    match value {
        Value::Null => Ok(None),
        Value::Sequence(seq) => {
            let mut out = Vec::new();
            for item in seq {
                let mapping = match item {
                    Value::Mapping(m) => m,
                    _ => return Err("links: each entry must be a mapping".to_string()),
                };
                let mut link = Link::default();
                for (k, v) in mapping {
                    let key = match k {
                        Value::String(s) => s.clone(),
                        _ => continue,
                    };
                    if key == "satisfies" {
                        link.satisfies = yaml_scalar_to_string(v);
                    } else {
                        link.extra.insert(key, yaml_to_json(v));
                    }
                }
                out.push(link);
            }
            Ok(Some(out))
        }
        _ => Err("links: expected a sequence".to_string()),
    }
}

fn parse_parameters(value: &Value) -> Option<IndexMap<String, ParameterValue>> {
    let mapping = match value {
        Value::Mapping(m) => m,
        _ => return None,
    };
    let mut out = IndexMap::new();
    for (k, v) in mapping {
        let key = match k {
            Value::String(s) if !s.trim().is_empty() => s.clone(),
            _ => continue,
        };
        let pv = match v {
            Value::String(s) => ParameterValue::String(s.clone()),
            Value::Bool(b) => ParameterValue::Bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ParameterValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    ParameterValue::Float(f)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        out.insert(key, pv);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn parse_attributes(value: &Value) -> Option<IndexMap<String, serde_json::Value>> {
    let mapping = match value {
        Value::Mapping(m) => m,
        _ => return None,
    };
    let mut out = IndexMap::new();
    for (k, v) in mapping {
        let key = match k {
            Value::String(s) => s.clone(),
            _ => continue,
        };
        out.insert(key, yaml_to_json(v));
    }
    Some(out)
}

/// Parse YAML object data into a normalized Requirement (GRD-SYS-009 / GRD-SYS-010).
pub fn parse_requirement_value(data: &Value) -> Result<Requirement, String> {
    let obj = match data {
        Value::Mapping(m) => m,
        _ => return Err("Expected an object".to_string()),
    };

    for k in obj.keys() {
        if let Value::String(ks) = k {
            if !ALLOWED_KEYS.contains(&ks.as_str()) {
                return Err(format!("unknown key: {ks}"));
            }
        } else {
            return Err("unknown key".to_string());
        }
    }

    let id = required_string(obj, "id")?;
    let title = required_string(obj, "title")?;
    let require = required_string(obj, "require")?;
    let refinement = optional_refinement(obj);

    let mut req = Requirement {
        id,
        title,
        require,
        refinement,
        attributes: None,
        links: None,
        satisfied_by: None,
        verified_by: None,
        parameters: None,
        tags: None,
    };

    if let Some(v) = obj.get(mapping_key("attributes")) {
        req.attributes = parse_attributes(v);
    }
    if let Some(v) = obj.get(mapping_key("links")) {
        req.links = parse_links(v)?;
    }
    if let Some(v) = obj.get(mapping_key("satisfied_by")) {
        req.satisfied_by = parse_artifact_list(v, "satisfied_by")?;
    }
    if let Some(v) = obj.get(mapping_key("verified_by")) {
        req.verified_by = parse_artifact_list(v, "verified_by")?;
    }
    if let Some(v) = obj.get(mapping_key("parameters")) {
        req.parameters = parse_parameters(v);
    }
    if let Some(v) = obj.get(mapping_key("tags")) {
        req.tags = Some(parse_tags(v)?);
    }

    Ok(req)
}

fn parse_tags(value: &Value) -> Result<IndexMap<String, serde_json::Value>, String> {
    let mapping = match value {
        Value::Mapping(m) => m,
        _ => return Err("tags: expected a mapping".to_string()),
    };
    let mut out = IndexMap::new();
    for (key, item) in mapping {
        let name = match key {
            Value::String(s) => s.clone(),
            _ => return Err("tags: tag names must be strings".to_string()),
        };
        out.insert(name, yaml_to_json(item));
    }
    Ok(out)
}

/// Export JSON Schema (draft-7 compatible) for requirement YAML files (GRD-SYS-009).
/// GRD-SYS-019: compose options add custom-tag constraints for the project.
pub fn export_requirement_file_json_schema(
    options: Option<&crate::types::RequirementSchemaComposeOptions>,
) -> serde_json::Value {
    let mut schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "ShallGraph requirement",
        "description": "YAML format for a single requirement file (id, title, require, refinement, attributes, links, satisfied_by, verified_by).",
        "type": "object",
        "additionalProperties": false,
        "required": ["id", "title", "require"],
        "properties": {
            "id": {
                "type": ["string", "number", "boolean"],
                "description": "Unique requirement identifier (e.g. GRD-HTML-001). Must match the filename without extension."
            },
            "title": {
                "type": ["string", "number", "boolean"],
                "description": "Short title of the requirement."
            },
            "require": {
                "type": ["string", "number", "boolean"],
                "description": "Single normative statement for this requirement (one Shall, Should, or May)."
            },
            "refinement": {
                "description": "Supporting detail for the requirement. Supports Markdown in HTML report output."
            },
            "attributes": {
                "type": "object",
                "description": "Optional key-value attributes (e.g. status, rationale)."
            },
            "links": {
                "type": "array",
                "items": { "type": "object" },
                "description": "Optional list of link objects (e.g. satisfies)."
            },
            "satisfied_by": {
                "type": "array",
                "description": "Optional artifacts that implement or satisfy this requirement.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["artifact"],
                    "properties": {
                        "artifact": { "type": ["string", "number", "boolean"] },
                        "description": { "type": ["string", "number", "boolean"] }
                    }
                }
            },
            "verified_by": {
                "type": "array",
                "description": "Optional artifacts that verify this requirement was met.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["artifact"],
                    "properties": {
                        "artifact": { "type": ["string", "number", "boolean"] },
                        "description": { "type": ["string", "number", "boolean"] }
                    }
                }
            },
            "parameters": {
                "type": "object",
                "description": "Named parameters for templating in text fields (string, number, or boolean values)."
            }
        }
    });
    if let Some(options) = options {
        crate::tags::apply_custom_tags_to_schema(&mut schema, &options.tags);
    }
    schema
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(yaml: &str) -> Requirement {
        let data: Value = serde_yaml::from_str(yaml).unwrap();
        parse_requirement_value(&data).unwrap()
    }

    #[test]
    fn parses_minimal_object_with_require_and_default_empty_refinement() {
        let r = parse_ok("id: GRD-X-001\ntitle: T\nrequire: The system shall do X.\n");
        assert_eq!(r.id, "GRD-X-001");
        assert_eq!(r.title, "T");
        assert_eq!(r.require, "The system shall do X.");
        assert_eq!(r.refinement, "");
    }

    #[test]
    fn rejects_legacy_description_key() {
        let data: Value = serde_yaml::from_str(
            "id: A\ntitle: B\nrequire: The system shall do X.\ndescription: legacy\n",
        )
        .unwrap();
        assert!(parse_requirement_value(&data).is_err());
    }

    #[test]
    fn rejects_unknown_top_level_keys() {
        let data: Value =
            serde_yaml::from_str("id: A\ntitle: B\nrequire: The system shall do X.\nextra: 1\n")
                .unwrap();
        assert!(parse_requirement_value(&data).is_err());
    }

    #[test]
    fn export_json_schema_is_draft07_object_with_id_and_title() {
        let json = export_requirement_file_json_schema(None);
        assert_eq!(json["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(json["type"], "object");
        let required = json["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "id"));
        assert!(required.iter().any(|v| v == "title"));
        assert!(json["properties"]["parameters"].is_object());
        assert!(json["properties"]["satisfied_by"].is_object());
        assert!(json["properties"]["verified_by"].is_object());
    }

    #[test]
    fn parses_artifacts_and_trims() {
        let r = parse_ok(
            r#"
id: GRD-X-001
title: T
require: The system shall do X.
satisfied_by:
  - artifact: src/foo.ts
    description: Implements X.
  - artifact: https://example.com/spec
verified_by:
  - artifact: test/foo.test.ts
"#,
        );
        assert_eq!(r.satisfied_by.as_ref().unwrap()[0].artifact, "src/foo.ts");
        assert_eq!(
            r.satisfied_by.as_ref().unwrap()[0].description.as_deref(),
            Some("Implements X.")
        );
        assert_eq!(
            r.satisfied_by.as_ref().unwrap()[1].artifact,
            "https://example.com/spec"
        );
        assert_eq!(
            r.verified_by.as_ref().unwrap()[0].artifact,
            "test/foo.test.ts"
        );

        let trimmed = parse_ok(
            r#"
id: GRD-X-002
title: T
require: The system shall do Y.
satisfied_by:
  - artifact: "  path.ts  "
    description: "  note  "
"#,
        );
        assert_eq!(
            trimmed.satisfied_by.as_ref().unwrap()[0].artifact,
            "path.ts"
        );
        assert_eq!(
            trimmed.satisfied_by.as_ref().unwrap()[0]
                .description
                .as_deref(),
            Some("note")
        );
    }

    #[test]
    fn rejects_artifact_missing_path_and_unknown_keys() {
        let missing: Value = serde_yaml::from_str(
            "id: GRD-X-003\ntitle: T\nrequire: The system shall do Z.\nsatisfied_by:\n  - description: no path\n",
        )
        .unwrap();
        assert!(parse_requirement_value(&data_or(&missing)).is_err());

        let extra: Value = serde_yaml::from_str(
            "id: GRD-X-004\ntitle: T\nrequire: The system shall do W.\nverified_by:\n  - artifact: a.ts\n    extra: 1\n",
        )
        .unwrap();
        assert!(parse_requirement_value(&extra).is_err());
    }

    fn data_or(v: &Value) -> Value {
        v.clone()
    }
}
