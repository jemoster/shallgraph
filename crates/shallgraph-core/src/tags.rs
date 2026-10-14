//! GRD-SYS-019: Custom tags in the project root marker and on requirement files.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde_json::Value as Json;
use serde_yaml::Value;

use crate::types::{
    CustomTagDef, RequirementSchemaComposeOptions, RequirementWithSource, ValidationError,
};

const SINGLE_KEYS: &[&str] = &["values", "required", "multiple", "allow_null"];
const MULTI_KEYS: &[&str] = &[
    "values",
    "required",
    "multiple",
    "allow_empty",
    "allow_null_entry",
    "max_entries",
];

/// Parsed `tags` section from a project root marker.
#[derive(Debug, Clone, Default)]
pub struct CustomTagSet {
    /// Valid tag definitions, in marker order.
    pub defs: IndexMap<String, CustomTagDef>,
    /// Tag names whose definitions failed validation.
    pub invalid_names: BTreeSet<String>,
    pub errors: Vec<ValidationError>,
    /// True when `tags` is a non-empty mapping. An omitted or empty `tags` key defines no custom tags.
    pub configured: bool,
}

#[derive(Clone, Copy)]
enum BoolField {
    Absent,
    Value(bool),
    Invalid,
}

fn tag_error(path: &str, name: &str, detail: impl Into<String>) -> ValidationError {
    ValidationError::new(path, format!("tag \"{name}\": {}", detail.into()))
}

fn yaml_key(key: &str) -> Value {
    Value::String(key.to_string())
}

fn optional_bool(
    mapping: &serde_yaml::Mapping,
    field: &str,
    path: &str,
    name: &str,
    errors: &mut Vec<ValidationError>,
) -> BoolField {
    match mapping.get(yaml_key(field)) {
        None => BoolField::Absent,
        Some(Value::Bool(value)) => BoolField::Value(*value),
        Some(_) => {
            errors.push(tag_error(
                path,
                name,
                format!("\"{field}\" must be a boolean"),
            ));
            BoolField::Invalid
        }
    }
}

fn bool_or_default(field: BoolField) -> bool {
    match field {
        BoolField::Value(value) => value,
        BoolField::Absent | BoolField::Invalid => false,
    }
}

fn parse_values(
    mapping: &serde_yaml::Mapping,
    path: &str,
    name: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<Vec<String>> {
    let Some(raw) = mapping.get(yaml_key("values")) else {
        errors.push(tag_error(path, name, "\"values\" is required"));
        return None;
    };
    let Value::Sequence(seq) = raw else {
        errors.push(tag_error(
            path,
            name,
            "\"values\" must be a non-empty sequence of strings",
        ));
        return None;
    };
    if seq.is_empty() {
        errors.push(tag_error(
            path,
            name,
            "\"values\" must be a non-empty sequence of strings",
        ));
        return None;
    }
    let mut values = Vec::new();
    let mut ok = true;
    for (index, item) in seq.iter().enumerate() {
        let entry = index + 1;
        match item {
            Value::String(text) => {
                if values.iter().any(|existing: &String| existing == text) {
                    errors.push(tag_error(path, name, format!("duplicate value \"{text}\"")));
                    ok = false;
                } else {
                    values.push(text.clone());
                }
            }
            Value::Null => {
                errors.push(tag_error(
                    path,
                    name,
                    format!("\"values\" entry {entry} is null"),
                ));
                ok = false;
            }
            _ => {
                errors.push(tag_error(
                    path,
                    name,
                    format!("\"values\" entry {entry} is not a string"),
                ));
                ok = false;
            }
        }
    }
    if ok {
        Some(values)
    } else {
        None
    }
}

fn parse_max_entries(
    mapping: &serde_yaml::Mapping,
    path: &str,
    name: &str,
    errors: &mut Vec<ValidationError>,
) -> Option<Option<u64>> {
    let Some(raw) = mapping.get(yaml_key("max_entries")) else {
        return Some(None);
    };
    let Value::Number(number) = raw else {
        errors.push(tag_error(
            path,
            name,
            "\"max_entries\" must be an integer greater than or equal to 0",
        ));
        return None;
    };
    if let Some(value) = number.as_u64() {
        return Some(Some(value));
    }
    if let Some(value) = number.as_i64() {
        if value >= 0 {
            return Some(Some(value as u64));
        }
    }
    errors.push(tag_error(
        path,
        name,
        "\"max_entries\" must be an integer greater than or equal to 0",
    ));
    None
}

fn parse_tag_def(
    name: &str,
    value: &Value,
    path: &str,
) -> Result<CustomTagDef, Vec<ValidationError>> {
    let mut errors = Vec::new();
    let Value::Mapping(mapping) = value else {
        return Err(vec![tag_error(path, name, "expected a mapping")]);
    };

    let multiple_field = optional_bool(mapping, "multiple", path, name, &mut errors);
    let cardinality_known = !matches!(multiple_field, BoolField::Invalid);
    let multiple = bool_or_default(multiple_field);
    let allowed: &[&str] = if !cardinality_known {
        &[
            "values",
            "required",
            "multiple",
            "allow_null",
            "allow_empty",
            "allow_null_entry",
            "max_entries",
        ]
    } else if multiple {
        MULTI_KEYS
    } else {
        SINGLE_KEYS
    };

    for key in mapping.keys() {
        let Value::String(key) = key else {
            continue;
        };
        if !allowed.contains(&key.as_str()) {
            errors.push(tag_error(path, name, format!("unknown key \"{key}\"")));
        }
    }

    let values = parse_values(mapping, path, name, &mut errors);
    let required_field = optional_bool(mapping, "required", path, name, &mut errors);
    let required = bool_or_default(required_field);

    let allow_null = if !multiple && cardinality_known {
        bool_or_default(optional_bool(
            mapping,
            "allow_null",
            path,
            name,
            &mut errors,
        ))
    } else {
        false
    };
    let allow_empty_field = if multiple && cardinality_known {
        optional_bool(mapping, "allow_empty", path, name, &mut errors)
    } else {
        BoolField::Absent
    };
    let allow_empty = bool_or_default(allow_empty_field);
    let allow_null_entry = if multiple && cardinality_known {
        bool_or_default(optional_bool(
            mapping,
            "allow_null_entry",
            path,
            name,
            &mut errors,
        ))
    } else {
        false
    };
    let max_entries = if multiple && cardinality_known {
        parse_max_entries(mapping, path, name, &mut errors).flatten()
    } else {
        None
    };

    if multiple
        && cardinality_known
        && matches!(allow_empty_field, BoolField::Absent | BoolField::Value(_))
    {
        if max_entries == Some(0) && !allow_empty {
            errors.push(tag_error(
                path,
                name,
                "\"max_entries\" of 0 requires \"allow_empty\" to be true",
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(CustomTagDef {
        name: name.to_string(),
        values: values.unwrap_or_default(),
        required,
        multiple,
        allow_null,
        allow_empty,
        allow_null_entry,
        max_entries,
    })
}

/// Parse custom tags from project root marker YAML.
/// Invalid marker YAML is ignored here; discovery already reports that failure.
#[shallgraph::implements("GRD-SYS-019")]
pub fn custom_tags_from_marker_yaml(raw: &str, marker_path: &str) -> CustomTagSet {
    let mut set = CustomTagSet::default();
    let Ok(data) = serde_yaml::from_str::<Value>(raw) else {
        return set;
    };
    let Value::Mapping(obj) = data else {
        return set;
    };
    let Some(tags_value) = obj.get(yaml_key("tags")) else {
        return set;
    };
    let Value::Mapping(mapping) = tags_value else {
        set.errors.push(ValidationError::new(
            marker_path,
            "tags: expected a mapping",
        ));
        return set;
    };
    if mapping.is_empty() {
        return set;
    }
    set.configured = true;
    for (key, value) in mapping {
        let Value::String(name) = key else {
            set.errors.push(ValidationError::new(
                marker_path,
                "tags: tag names must be strings",
            ));
            continue;
        };
        if name.is_empty() {
            set.errors.push(tag_error(
                marker_path,
                name,
                "name must be a non-empty string",
            ));
            set.invalid_names.insert(name.clone());
            continue;
        }
        match parse_tag_def(name, value, marker_path) {
            Ok(def) => {
                set.defs.insert(name.clone(), def);
            }
            Err(def_errors) => {
                set.errors.extend(def_errors);
                set.invalid_names.insert(name.clone());
            }
        }
    }
    set
}

fn value_error(path: &str, name: &str, detail: impl Into<String>) -> ValidationError {
    tag_error(path, name, detail)
}

fn validate_tag_value(def: &CustomTagDef, value: &Json, path: &str) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if !def.multiple {
        match value {
            Json::Null => {
                if !def.allow_null {
                    errors.push(value_error(path, &def.name, "null is not allowed"));
                }
            }
            Json::String(text) => {
                if !def.values.iter().any(|allowed| allowed == text) {
                    errors.push(value_error(
                        path,
                        &def.name,
                        format!("value \"{text}\" is not allowed"),
                    ));
                }
            }
            _ => errors.push(value_error(path, &def.name, "expected a string")),
        }
        return errors;
    }

    let Json::Array(items) = value else {
        errors.push(value_error(path, &def.name, "expected a sequence"));
        return errors;
    };
    if items.is_empty() && !def.allow_empty {
        errors.push(value_error(path, &def.name, "empty list is not allowed"));
    }
    if let Some(limit) = def.max_entries {
        if items.len() as u64 > limit {
            errors.push(value_error(
                path,
                &def.name,
                format!("has {} entries; limit is {limit}", items.len()),
            ));
        }
    }
    for (index, item) in items.iter().enumerate() {
        let entry = index + 1;
        match item {
            Json::Null => {
                if !def.allow_null_entry {
                    errors.push(value_error(
                        path,
                        &def.name,
                        format!("item {entry}: null is not allowed"),
                    ));
                }
            }
            Json::String(text) => {
                if !def.values.iter().any(|allowed| allowed == text) {
                    errors.push(value_error(
                        path,
                        &def.name,
                        format!("item {entry}: value \"{text}\" is not allowed"),
                    ));
                }
            }
            _ => errors.push(value_error(
                path,
                &def.name,
                format!("item {entry}: expected a string"),
            )),
        }
    }
    errors
}

/// Validate requirement-file tag entries against a parsed marker.
#[shallgraph::implements("GRD-SYS-019")]
pub fn validate_requirement_tags(
    set: &CustomTagSet,
    requirements: &[RequirementWithSource],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    for requirement in requirements {
        let path = requirement.source_path.display().to_string();
        let Some(tags) = &requirement.tags else {
            if set.configured {
                for def in set.defs.values() {
                    if def.required {
                        errors.push(value_error(&path, &def.name, "is required"));
                    }
                }
            }
            continue;
        };

        if !set.configured {
            if set.errors.is_empty() {
                if tags.is_empty() {
                    errors.push(ValidationError::new(
                        &path,
                        "tags: no custom tags are defined",
                    ));
                } else {
                    for name in tags.keys() {
                        errors.push(value_error(&path, name, "is not defined"));
                    }
                }
            }
            continue;
        }

        for (name, value) in tags {
            if set.invalid_names.contains(name) {
                errors.push(value_error(&path, name, "configuration is invalid"));
                continue;
            }
            let Some(def) = set.defs.get(name) else {
                errors.push(value_error(&path, name, "is not defined"));
                continue;
            };
            errors.extend(validate_tag_value(def, value, &path));
        }
        for def in set.defs.values() {
            if def.required && !tags.contains_key(&def.name) {
                errors.push(value_error(&path, &def.name, "is required"));
            }
        }
    }
    errors
}

fn tag_value_schema(def: &CustomTagDef) -> Json {
    if !def.multiple {
        let mut values: Vec<Json> = def.values.iter().cloned().map(Json::String).collect();
        if def.allow_null {
            values.push(Json::Null);
            return Json::Object(serde_json::Map::from_iter([(
                "enum".to_string(),
                Json::Array(values),
            )]));
        }
        return serde_json::json!({
            "type": "string",
            "enum": values,
        });
    }
    let mut item_values: Vec<Json> = def.values.iter().cloned().map(Json::String).collect();
    if def.allow_null_entry {
        item_values.push(Json::Null);
    }
    let mut schema = serde_json::json!({
        "type": "array",
        "items": { "enum": item_values },
    });
    if !def.allow_empty {
        schema["minItems"] = Json::from(1_u64);
    }
    if let Some(limit) = def.max_entries {
        schema["maxItems"] = Json::from(limit);
    }
    schema
}

/// Add custom-tag constraints to the effective requirement JSON Schema.
#[shallgraph::implements("GRD-SYS-019")]
pub fn apply_custom_tags_to_schema(schema: &mut Json, tags: &[CustomTagDef]) {
    if tags.is_empty() {
        return;
    }
    let Some(properties) = schema.get_mut("properties").and_then(Json::as_object_mut) else {
        return;
    };
    let mut tag_properties = serde_json::Map::new();
    let mut required_names = Vec::new();
    for def in tags {
        if def.required {
            required_names.push(Json::String(def.name.clone()));
        }
        tag_properties.insert(def.name.clone(), tag_value_schema(def));
    }
    let mut tags_schema = serde_json::Map::new();
    tags_schema.insert("type".into(), Json::String("object".into()));
    tags_schema.insert("additionalProperties".into(), Json::Bool(false));
    tags_schema.insert(
        "description".into(),
        Json::String("Custom tags defined for this project.".into()),
    );
    tags_schema.insert("properties".into(), Json::Object(tag_properties));
    if !required_names.is_empty() {
        tags_schema.insert("required".into(), Json::Array(required_names));
    }
    properties.insert("tags".into(), Json::Object(tags_schema));

    if tags.iter().any(|def| def.required) {
        if let Some(required) = schema.get_mut("required").and_then(Json::as_array_mut) {
            if !required.iter().any(|item| item.as_str() == Some("tags")) {
                required.push(Json::String("tags".into()));
            }
        }
    }
}

pub fn compose_options_from_tag_set(set: &CustomTagSet) -> Option<RequirementSchemaComposeOptions> {
    if set.defs.is_empty() {
        return None;
    }
    Some(RequirementSchemaComposeOptions {
        tags: set.defs.values().cloned().collect(),
    })
}

pub fn compose_options_to_json(options: &RequirementSchemaComposeOptions) -> Json {
    let tags = options
        .tags
        .iter()
        .map(|def| {
            serde_json::json!({
                "name": def.name,
                "values": def.values,
                "required": def.required,
                "multiple": def.multiple,
                "allowNull": def.allow_null,
                "allowEmpty": def.allow_empty,
                "allowNullEntry": def.allow_null_entry,
                "maxEntries": def.max_entries,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({ "tags": tags })
}

pub fn compose_options_from_json(value: &Json) -> Result<RequirementSchemaComposeOptions, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "compose options must be an object".to_string())?;
    let Some(raw_tags) = obj.get("tags") else {
        return Ok(RequirementSchemaComposeOptions::default());
    };
    if raw_tags.is_null() {
        return Ok(RequirementSchemaComposeOptions::default());
    }
    let items = raw_tags
        .as_array()
        .ok_or_else(|| "compose options tags must be an array".to_string())?;
    let mut tags = Vec::new();
    for item in items {
        let tag = item
            .as_object()
            .ok_or_else(|| "custom tag definition must be an object".to_string())?;
        let name = tag
            .get("name")
            .and_then(Json::as_str)
            .ok_or_else(|| "custom tag definition requires name".to_string())?
            .to_string();
        let values = tag
            .get("values")
            .and_then(Json::as_array)
            .ok_or_else(|| format!("custom tag \"{name}\" requires values"))?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("custom tag \"{name}\" values must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let required = tag.get("required").and_then(Json::as_bool).unwrap_or(false);
        let multiple = tag.get("multiple").and_then(Json::as_bool).unwrap_or(false);
        let allow_null = tag
            .get("allowNull")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let allow_empty = tag
            .get("allowEmpty")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let allow_null_entry = tag
            .get("allowNullEntry")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        let max_entries =
            match tag.get("maxEntries") {
                None | Some(Json::Null) => None,
                Some(value) => Some(value.as_u64().ok_or_else(|| {
                    format!("custom tag \"{name}\" maxEntries must be an integer")
                })?),
            };
        tags.push(CustomTagDef {
            name,
            values,
            required,
            multiple,
            allow_null,
            allow_empty,
            allow_null_entry,
            max_entries,
        });
    }
    Ok(RequirementSchemaComposeOptions { tags })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_requirement_content;
    use crate::schema::export_requirement_file_json_schema;
    use std::path::Path;

    fn marker(body: &str) -> String {
        format!("requirement_dirs:\n  - requirements\n{body}")
    }

    fn set_of(body: &str) -> CustomTagSet {
        custom_tags_from_marker_yaml(&marker(body), "shallgraph.yaml")
    }

    fn messages(set: &CustomTagSet) -> Vec<String> {
        set.errors.iter().map(|err| err.message.clone()).collect()
    }

    fn requirement(yaml: &str) -> RequirementWithSource {
        parse_requirement_content(yaml, Path::new("requirements/GRD-TAG-001.req.yml")).unwrap()
    }

    fn usage_messages(body: &str, yaml: &str) -> Vec<String> {
        let set = set_of(body);
        let req = requirement(yaml);
        let mut errors = set.errors.clone();
        errors.extend(validate_requirement_tags(&set, &[req]));
        errors.into_iter().map(|err| err.message).collect()
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn omitted_or_empty_tags_define_nothing() {
        let omitted = set_of("");
        assert!(!omitted.configured);
        assert!(omitted.errors.is_empty());
        let empty = set_of("tags: {}\n");
        assert!(!empty.configured);
        assert!(empty.errors.is_empty());
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn tags_must_be_a_mapping_of_named_definitions() {
        let set = set_of("tags:\n  - priority\n");
        assert_eq!(set.errors[0].path, "shallgraph.yaml");
        assert_eq!(set.errors[0].message, "tags: expected a mapping");

        let set = set_of("tags:\n  \"\":\n    values: [a]\n");
        assert!(messages(&set).iter().any(|m| m.contains("tag \"\":")));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn values_are_a_non_empty_exact_set() {
        let duplicate = set_of("tags:\n  priority:\n    values: [low, low]\n");
        assert!(messages(&duplicate)
            .iter()
            .any(|m| m.contains("tag \"priority\":") && m.contains("duplicate value \"low\"")));

        let null_entry = set_of("tags:\n  priority:\n    values: [low, null]\n");
        assert!(messages(&null_entry)
            .iter()
            .any(|m| m.contains("\"values\" entry 2 is null")));

        let number = set_of("tags:\n  priority:\n    values: [1]\n");
        assert!(messages(&number)
            .iter()
            .any(|m| m.contains("\"values\" entry 1 is not a string")));

        let empty = set_of("tags:\n  priority:\n    values: []\n");
        assert!(messages(&empty)
            .iter()
            .any(|m| m.contains("non-empty sequence of strings")));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn cardinality_limits_which_keys_are_accepted() {
        let single = set_of(
            "tags:\n  priority:\n    values: [low]\n    allow_empty: true\n    max_entries: 1\n",
        );
        let text = messages(&single).join("\n");
        assert!(text.contains("unknown key \"allow_empty\""));
        assert!(text.contains("unknown key \"max_entries\""));

        let multi =
            set_of("tags:\n  labels:\n    values: [a]\n    multiple: true\n    allow_null: true\n");
        assert!(messages(&multi)
            .iter()
            .any(|m| m.contains("unknown key \"allow_null\"")));

        let bad_max =
            set_of("tags:\n  labels:\n    values: [a]\n    multiple: true\n    max_entries: -1\n");
        assert!(messages(&bad_max)
            .iter()
            .any(|m| m.contains("\"max_entries\" must be an integer")));

        let zero =
            set_of("tags:\n  labels:\n    values: [a]\n    multiple: true\n    max_entries: 0\n");
        assert!(messages(&zero)
            .iter()
            .any(|m| m.contains("requires \"allow_empty\" to be true")));

        let zero_ok = set_of(
            "tags:\n  labels:\n    values: [a]\n    multiple: true\n    allow_empty: true\n    max_entries: 0\n",
        );
        assert!(zero_ok.errors.is_empty(), "{:?}", messages(&zero_ok));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn example_configuration_accepts_the_example_requirement() {
        let body = r#"
tags:
  priority:
    values: [low, medium, high]
    required: true
  owner:
    values: [unassigned, alice, bob]
    allow_null: true
  labels:
    values: [safety, security, performance]
    multiple: true
    allow_empty: false
    allow_null_entry: true
    max_entries: 3
"#;
        let yaml = r#"
id: GRD-TAG-001
title: Tagged
require: The system shall honor tags.
tags:
  priority: high
  owner: null
  labels: [safety, null, performance]
"#;
        let errors = usage_messages(body, yaml);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn requirement_entries_follow_cardinality_and_limits() {
        let body = r#"
tags:
  priority:
    values: [low, medium, high]
    required: true
  owner:
    values: [alice]
    allow_null: true
  labels:
    values: [safety, security]
    multiple: true
    allow_null_entry: true
    max_entries: 2
"#;
        let base = "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\n";

        let missing = usage_messages(body, base);
        assert!(missing
            .iter()
            .any(|m| m.contains("tag \"priority\": is required")));

        let bad_value = usage_messages(
            body,
            &format!("{base}tags:\n  priority: High\n  labels: [safety]\n"),
        );
        assert!(bad_value
            .iter()
            .any(|m| m.contains("tag \"priority\": value \"High\" is not allowed")));

        let null_rejected = usage_messages(
            body,
            &format!("{base}tags:\n  priority: null\n  labels: [safety]\n"),
        );
        assert!(null_rejected
            .iter()
            .any(|m| m.contains("tag \"priority\": null is not allowed")));

        let null_ok = usage_messages(
            body,
            &format!("{base}tags:\n  priority: low\n  owner: null\n  labels: [safety, safety]\n"),
        );
        assert!(null_ok.is_empty(), "{null_ok:?}");

        let shape = usage_messages(
            body,
            &format!("{base}tags:\n  priority: [low]\n  labels: safety\n"),
        );
        assert!(shape
            .iter()
            .any(|m| m.contains("tag \"priority\": expected a string")));
        assert!(shape
            .iter()
            .any(|m| m.contains("tag \"labels\": expected a sequence")));

        let empty = usage_messages(
            body,
            &format!("{base}tags:\n  priority: low\n  labels: []\n"),
        );
        assert!(empty
            .iter()
            .any(|m| m.contains("tag \"labels\": empty list is not allowed")));

        let too_many = usage_messages(
            body,
            &format!("{base}tags:\n  priority: low\n  labels: [safety, null, security]\n"),
        );
        assert!(too_many
            .iter()
            .any(|m| m.contains("tag \"labels\": has 3 entries; limit is 2")));

        let unknown = usage_messages(
            body,
            &format!("{base}tags:\n  priority: low\n  labels: [safety]\n  extra: safety\n"),
        );
        assert!(unknown
            .iter()
            .any(|m| m.contains("tag \"extra\": is not defined")));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn absent_optional_tag_is_valid_and_present_entry_is_checked() {
        let body = "tags:\n  owner:\n    values: [alice]\n";
        let omitted = usage_messages(
            body,
            "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\n",
        );
        assert!(omitted.is_empty(), "{omitted:?}");

        let present = usage_messages(
            body,
            "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\ntags:\n  owner: bob\n",
        );
        assert!(present
            .iter()
            .any(|m| m.contains("tag \"owner\": value \"bob\" is not allowed")));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn tags_field_fails_when_no_custom_tags_are_defined() {
        let yaml = "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\ntags:\n  priority: low\n";
        let errors = usage_messages("", yaml);
        assert_eq!(errors, vec!["tag \"priority\": is not defined".to_string()]);

        let empty = usage_messages(
            "tags: {}\n",
            "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\ntags: {}\n",
        );
        assert_eq!(empty, vec!["tags: no custom tags are defined".to_string()]);
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn null_entries_are_rejected_unless_configured() {
        let body = "tags:\n  labels:\n    values: [safety]\n    multiple: true\n";
        let errors = usage_messages(
            body,
            "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\ntags:\n  labels: [null]\n",
        );
        assert!(errors
            .iter()
            .any(|m| m.contains("tag \"labels\": item 1: null is not allowed")));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn schema_includes_tag_constraints() {
        let set = set_of(
            r#"
tags:
  priority:
    values: [low, medium, high]
    required: true
  labels:
    values: [safety, security]
    multiple: true
    allow_empty: true
    allow_null_entry: true
    max_entries: 3
"#,
        );
        let options = compose_options_from_tag_set(&set).unwrap();
        let schema = export_requirement_file_json_schema(Some(&options));
        let tags = &schema["properties"]["tags"];
        assert_eq!(tags["additionalProperties"], false);
        assert_eq!(
            tags["properties"]["priority"]["enum"],
            serde_json::json!(["low", "medium", "high"])
        );
        assert_eq!(tags["required"], serde_json::json!(["priority"]));
        assert_eq!(tags["properties"]["labels"]["maxItems"], 3);
        assert_eq!(tags["properties"]["labels"]["type"], "array");
        assert!(tags["properties"]["labels"].get("minItems").is_none());
        assert!(tags["properties"]["labels"]["items"]["enum"]
            .as_array()
            .unwrap()
            .contains(&Json::Null));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "tags"));

        let plain = export_requirement_file_json_schema(None);
        assert!(plain["properties"].get("tags").is_none());
        assert!(!plain["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "tags"));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn config_errors_use_the_marker_path_and_tag_name() {
        let set = custom_tags_from_marker_yaml(
            "requirement_dirs:\n  - requirements\ntags:\n  priority:\n    values: []\n",
            "/repo/shallgraph.yaml",
        );
        assert_eq!(set.errors[0].path, "/repo/shallgraph.yaml");
        assert!(set.errors[0].message.contains("tag \"priority\":"));
    }

    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn format_preserves_tag_entries() {
        let yaml = r#"
id: GRD-TAG-001
title: Tagged
require: The system shall honor tags.
tags:
  priority: high
  owner: null
  labels:
    - safety
    - null
    - performance
"#;
        let parsed = requirement(yaml);
        let formatted = crate::format::format_requirement_to_yaml(&parsed.to_requirement());
        let again =
            parse_requirement_content(&formatted, Path::new("requirements/GRD-TAG-001.req.yml"))
                .unwrap();
        assert_eq!(again.tags, parsed.tags);
        let twice = crate::format::format_requirement_to_yaml(&again.to_requirement());
        assert_eq!(twice, formatted);
    }

    #[cfg(feature = "std-fs")]
    #[shallgraph::verifies("GRD-SYS-019")]
    #[test]
    fn load_reports_marker_and_requirement_paths() {
        use crate::load::load_requirements;
        use crate::ROOT_MARKER;
        use std::fs;
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("shallgraph-tags-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("requirements")).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\ntags:\n  priority:\n    values: [low, high]\n    required: true\n",
        )
        .unwrap();
        fs::write(
            root.join("requirements/GRD-TAG-001.req.yml"),
            "id: GRD-TAG-001\ntitle: T\nrequire: The system shall honor tags.\ntags:\n  priority: medium\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.iter().any(|err| {
            err.path == root.join(ROOT_MARKER).display().to_string()
                || (err.path.ends_with("GRD-TAG-001.req.yml")
                    && err.message.contains("tag \"priority\":"))
        }));
        assert!(result.errors.iter().any(|err| {
            err.path.ends_with("GRD-TAG-001.req.yml")
                && err
                    .message
                    .contains("tag \"priority\": value \"medium\" is not allowed")
        }));
        let _ = fs::remove_dir_all(&root);
    }
}
