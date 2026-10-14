//! JSON ABI over `shallgraph-core` for JavaScript hosts (VS Code, later web).

use std::path::Path;

use indexmap::IndexMap;
use serde_json::{json, Map, Value};
use shallgraph_core::{
    export_requirement_file_json_schema, format_requirement_to_yaml,
    generate_single_requirement_html_with_source_links, get_requirement_profile,
    is_requirement_filename, list_registered_profile_ids, parse_requirement_content,
    parse_root_marker_yaml, requirement_id_from_filename, validate_requirements,
    ArtifactLinkRenderOptions, ArtifactRef, GithubArtifactLinkContext, Link, ParameterValue,
    RequirementWithSource, REQUIREMENT_FILE_EXTENSION, STANDARD_PROFILE_ID,
};
use wasm_bindgen::prelude::*;

fn parameter_to_json(value: &ParameterValue) -> Value {
    match value {
        ParameterValue::String(s) => Value::String(s.clone()),
        ParameterValue::Integer(n) => json!(n),
        ParameterValue::Float(n) => json!(n),
        ParameterValue::Bool(b) => Value::Bool(*b),
    }
}

fn parameter_from_json(value: &Value) -> Result<ParameterValue, String> {
    match value {
        Value::String(s) => Ok(ParameterValue::String(s.clone())),
        Value::Bool(b) => Ok(ParameterValue::Bool(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ParameterValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(ParameterValue::Float(f))
            } else {
                Err("unsupported number".into())
            }
        }
        _ => Err("parameter values must be string, number, or boolean".into()),
    }
}

fn link_to_json(link: &Link) -> Value {
    let mut obj = Map::new();
    if let Some(id) = &link.satisfies {
        obj.insert("satisfies".into(), Value::String(id.clone()));
    }
    for (k, v) in &link.extra {
        obj.insert(k.clone(), v.clone());
    }
    Value::Object(obj)
}

fn link_from_json(value: &Value) -> Result<Link, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "link must be an object".to_string())?;
    let mut extra = std::collections::BTreeMap::new();
    let mut satisfies = None;
    for (k, v) in obj {
        if k == "satisfies" {
            satisfies = v.as_str().map(str::to_string);
        } else {
            extra.insert(k.clone(), v.clone());
        }
    }
    Ok(Link { satisfies, extra })
}

fn artifact_to_json(r: &ArtifactRef) -> Value {
    let mut obj = Map::new();
    obj.insert("artifact".into(), Value::String(r.artifact.clone()));
    if let Some(d) = &r.description {
        obj.insert("description".into(), Value::String(d.clone()));
    }
    Value::Object(obj)
}

fn artifact_from_json(value: &Value) -> Result<ArtifactRef, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "artifact ref must be an object".to_string())?;
    let artifact = obj
        .get("artifact")
        .and_then(Value::as_str)
        .ok_or_else(|| "artifact is required".to_string())?
        .to_string();
    let description = obj
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(ArtifactRef {
        artifact,
        description,
    })
}

fn indexmap_from_object<T, F>(
    value: Option<&Value>,
    mut convert: F,
) -> Result<Option<IndexMap<String, T>>, String>
where
    F: FnMut(&Value) -> Result<T, String>,
{
    let Some(v) = value else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let obj = v.as_object().ok_or_else(|| "expected object".to_string())?;
    let mut map = IndexMap::new();
    for (k, val) in obj {
        map.insert(k.clone(), convert(val)?);
    }
    Ok(Some(map))
}

fn requirement_to_json(r: &RequirementWithSource) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), Value::String(r.id.clone()));
    obj.insert("title".into(), Value::String(r.title.clone()));
    obj.insert("require".into(), Value::String(r.require.clone()));
    obj.insert("refinement".into(), Value::String(r.refinement.clone()));
    if let Some(attrs) = &r.attributes {
        obj.insert(
            "attributes".into(),
            Value::Object(attrs.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        );
    }
    if let Some(links) = &r.links {
        obj.insert(
            "links".into(),
            Value::Array(links.iter().map(link_to_json).collect()),
        );
    }
    if let Some(refs) = &r.satisfied_by {
        obj.insert(
            "satisfied_by".into(),
            Value::Array(refs.iter().map(artifact_to_json).collect()),
        );
    }
    if let Some(refs) = &r.verified_by {
        obj.insert(
            "verified_by".into(),
            Value::Array(refs.iter().map(artifact_to_json).collect()),
        );
    }
    if let Some(params) = &r.parameters {
        obj.insert(
            "parameters".into(),
            Value::Object(
                params
                    .iter()
                    .map(|(k, v)| (k.clone(), parameter_to_json(v)))
                    .collect(),
            ),
        );
    }
    if let Some(tags) = &r.tags {
        obj.insert(
            "tags".into(),
            Value::Object(tags.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        );
    }
    obj.insert(
        "sourcePath".into(),
        Value::String(r.source_path.to_string_lossy().into_owned()),
    );
    if let Some(cat) = &r.category_path {
        obj.insert(
            "categoryPath".into(),
            Value::Array(cat.iter().cloned().map(Value::String).collect()),
        );
    }
    Value::Object(obj)
}

fn tags_from_json(value: Option<&Value>) -> Result<Option<IndexMap<String, Value>>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(obj)) => {
            let mut map = IndexMap::new();
            for (key, item) in obj {
                map.insert(key.clone(), item.clone());
            }
            Ok(Some(map))
        }
        Some(_) => Err("tags must be an object".into()),
    }
}

fn artifacts_from_json(value: Option<&Value>) -> Result<Option<Vec<ArtifactRef>>, String> {
    let Some(v) = value else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let arr = v.as_array().ok_or_else(|| "expected array".to_string())?;
    let mut out = Vec::new();
    for item in arr {
        out.push(artifact_from_json(item)?);
    }
    Ok(Some(out))
}

fn requirement_from_json(value: &Value) -> Result<RequirementWithSource, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "requirement must be an object".to_string())?;
    let get_str = |key: &str| -> Result<String, String> {
        obj.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("{key} is required"))
    };
    let links = match obj.get("links") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                out.push(link_from_json(item)?);
            }
            Some(out)
        }
        Some(_) => return Err("links must be an array".into()),
    };
    let category_path = match obj.get("categoryPath") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ),
        Some(_) => return Err("categoryPath must be an array".into()),
    };
    Ok(RequirementWithSource {
        id: get_str("id")?,
        title: get_str("title")?,
        require: get_str("require")?,
        refinement: obj
            .get("refinement")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        attributes: indexmap_from_object(obj.get("attributes"), |v| Ok(v.clone()))?,
        links,
        satisfied_by: artifacts_from_json(obj.get("satisfied_by"))?,
        verified_by: artifacts_from_json(obj.get("verified_by"))?,
        parameters: indexmap_from_object(obj.get("parameters"), parameter_from_json)?,
        tags: tags_from_json(obj.get("tags"))?,
        source_path: Path::new(obj.get("sourcePath").and_then(Value::as_str).unwrap_or(""))
            .to_path_buf(),
        category_path,
    })
}

fn error_json(path: &str, message: &str, line: Option<usize>) -> String {
    json!({
        "error": {
            "path": path,
            "message": message,
            "line": line,
        }
    })
    .to_string()
}

fn parse_json(raw: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = requirementFileExtension)]
pub fn requirement_file_extension() -> String {
    REQUIREMENT_FILE_EXTENSION.to_string()
}

#[wasm_bindgen(js_name = isRequirementFilename)]
pub fn wasm_is_requirement_filename(basename: &str) -> bool {
    is_requirement_filename(basename)
}

#[wasm_bindgen(js_name = requirementIdFromFilename)]
pub fn wasm_requirement_id_from_filename(basename: &str) -> Option<String> {
    requirement_id_from_filename(basename).map(str::to_string)
}

#[wasm_bindgen(js_name = parseRequirementContent)]
pub fn wasm_parse_requirement_content(yaml: &str, path: &str) -> String {
    match parse_requirement_content(yaml, Path::new(path)) {
        Ok(req) => json!({ "requirement": requirement_to_json(&req) }).to_string(),
        Err(err) => error_json(&err.path, &err.message, err.line),
    }
}

#[wasm_bindgen(js_name = validateRequirements)]
pub fn wasm_validate_requirements(requirements_json: &str) -> String {
    let parsed = match parse_json(requirements_json) {
        Ok(v) => v,
        Err(err) => return error_json("", &err, None),
    };
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return error_json("", "validateRequirements expects a JSON array", None),
    };
    let mut reqs = Vec::new();
    for item in arr {
        match requirement_from_json(item) {
            Ok(r) => reqs.push(r),
            Err(err) => return error_json("", &err, None),
        }
    }
    let errors: Vec<Value> = validate_requirements(&reqs)
        .into_iter()
        .map(|e| {
            json!({
                "path": e.path,
                "message": e.message,
                "line": e.line,
            })
        })
        .collect();
    json!(errors).to_string()
}

#[wasm_bindgen(js_name = formatRequirementToYaml)]
pub fn wasm_format_requirement_to_yaml(requirement_json: &str) -> String {
    let parsed = match parse_json(requirement_json) {
        Ok(v) => v,
        Err(err) => return error_json("", &err, None),
    };
    let req = match requirement_from_json(&parsed) {
        Ok(r) => r,
        Err(err) => return error_json("", &err, None),
    };
    json!({ "yaml": format_requirement_to_yaml(&req.to_requirement()) }).to_string()
}

#[wasm_bindgen(js_name = exportRequirementFileJsonSchema)]
pub fn wasm_export_requirement_file_json_schema(compose_json: Option<String>) -> String {
    let options = match compose_json.as_deref() {
        None | Some("") | Some("null") => None,
        Some(raw) => match parse_json(raw) {
            Ok(value) => match shallgraph_core::compose_options_from_json(&value) {
                Ok(options) => Some(options),
                Err(err) => return error_json("", &err, None),
            },
            Err(err) => return error_json("", &err, None),
        },
    };
    export_requirement_file_json_schema(options.as_ref()).to_string()
}

#[wasm_bindgen(js_name = validateCustomTags)]
pub fn wasm_validate_custom_tags(
    marker_yaml: &str,
    marker_path: &str,
    requirements_json: &str,
) -> String {
    let parsed = match parse_json(requirements_json) {
        Ok(v) => v,
        Err(err) => return error_json("", &err, None),
    };
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return error_json("", "validateCustomTags expects a JSON array", None),
    };
    let mut reqs = Vec::new();
    for item in arr {
        match requirement_from_json(item) {
            Ok(r) => reqs.push(r),
            Err(err) => return error_json("", &err, None),
        }
    }
    let set = shallgraph_core::custom_tags_from_marker_yaml(marker_yaml, marker_path);
    let mut errors = set.errors.clone();
    errors.extend(shallgraph_core::validate_requirement_tags(&set, &reqs));
    let body: Vec<Value> = errors
        .into_iter()
        .map(|err| {
            json!({
                "path": err.path,
                "message": err.message,
                "line": err.line,
            })
        })
        .collect();
    json!(body).to_string()
}

#[wasm_bindgen(js_name = customTagComposeOptions)]
pub fn wasm_custom_tag_compose_options(marker_yaml: &str) -> String {
    let set = shallgraph_core::custom_tags_from_marker_yaml(marker_yaml, "");
    match shallgraph_core::compose_options_from_tag_set(&set) {
        None => "null".to_string(),
        Some(options) => shallgraph_core::compose_options_to_json(&options).to_string(),
    }
}

fn artifact_links_from_json(raw: Option<&str>) -> Option<ArtifactLinkRenderOptions> {
    let raw = raw?;
    if raw.is_empty() {
        return None;
    }
    let v: Value = serde_json::from_str(raw).ok()?;
    let github = v.get("github").and_then(|github| {
        Some(GithubArtifactLinkContext {
            host: github
                .get("host")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            owner: github.get("owner")?.as_str()?.to_string(),
            repo: github.get("repo")?.as_str()?.to_string(),
            commit_sha: github.get("commitSha")?.as_str()?.to_string(),
            project_root_rel: github
                .get("projectRootRel")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            project_root: github
                .get("projectRoot")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    });
    Some(ArtifactLinkRenderOptions { github })
}

#[wasm_bindgen(js_name = generateSingleRequirementHtml)]
pub fn wasm_generate_single_requirement_html(
    requirement_json: &str,
    all_json: Option<String>,
    artifact_links_json: Option<String>,
) -> String {
    let parsed = match parse_json(requirement_json) {
        Ok(v) => v,
        Err(err) => return error_json("", &err, None),
    };
    let req = match requirement_from_json(&parsed) {
        Ok(r) => r,
        Err(err) => return error_json("", &err, None),
    };
    let all = if let Some(raw) = all_json.as_deref() {
        if raw.is_empty() {
            None
        } else {
            match parse_json(raw) {
                Ok(Value::Array(items)) => {
                    let mut list = Vec::new();
                    for item in items {
                        match requirement_from_json(&item) {
                            Ok(r) => list.push(r),
                            Err(err) => return error_json("", &err, None),
                        }
                    }
                    Some(list)
                }
                Ok(_) => return error_json("", "allRequirements must be a JSON array", None),
                Err(err) => return error_json("", &err, None),
            }
        }
    } else {
        None
    };
    let artifact_links = artifact_links_from_json(artifact_links_json.as_deref());
    generate_single_requirement_html_with_source_links(
        &req,
        all.as_deref(),
        &[],
        artifact_links.as_ref(),
    )
}

#[wasm_bindgen(js_name = parseRootMarker)]
pub fn wasm_parse_root_marker(yaml: &str, marker_label: &str) -> String {
    match parse_root_marker_yaml(yaml, marker_label, None) {
        Ok(cfg) => {
            let known = list_registered_profile_ids();
            if !known.contains(&cfg.profile.as_str()) {
                let mut sorted = known;
                sorted.sort();
                return error_json(
                    marker_label,
                    &format!(
                        "Unknown profile \"{}\". Known profiles: {}",
                        cfg.profile,
                        sorted.join(", ")
                    ),
                    None,
                );
            }
            json!({
                "profile": cfg.profile,
                "requirementDirs": cfg.requirement_dirs,
            })
            .to_string()
        }
        Err(err) => error_json(marker_label, &err.to_string(), None),
    }
}

#[wasm_bindgen(js_name = standardProfileId)]
pub fn wasm_standard_profile_id() -> String {
    STANDARD_PROFILE_ID.to_string()
}

#[wasm_bindgen(js_name = listRegisteredProfileIds)]
pub fn wasm_list_registered_profile_ids() -> String {
    json!(list_registered_profile_ids()).to_string()
}

#[wasm_bindgen(js_name = hasRequirementProfile)]
pub fn wasm_has_requirement_profile(id: &str) -> bool {
    get_requirement_profile(id).is_ok()
}
