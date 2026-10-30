//! GRD-VALID-007: A Skylark file named by the project root marker validates requirement ids.
//!
//! Skylark is the Starlark language. The file defines `validate_requirement_id`.

use std::path::{Path, PathBuf};

use serde_yaml::Value;
use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::Value as StarlarkValue;

use crate::discovery::normalize_path;
use crate::types::{RequirementWithSource, ValidationError};

/// Name of the function a Skylark file defines to validate one requirement id.
pub const VALIDATE_REQUIREMENT_ID: &str = "validate_requirement_id";

/// `skylark` entry parsed from a project root marker.
#[derive(Debug, Clone, PartialEq)]
pub struct SkylarkMarkerConfig {
    /// Path as written in the marker, relative to the project root.
    pub relative_path: Option<String>,
    pub errors: Vec<ValidationError>,
}

/// Parse the optional `skylark` key. Invalid marker YAML is ignored here; discovery reports that failure.
#[shallgraph::implements("GRD-VALID-007")]
pub fn skylark_from_marker_yaml(raw: &str, marker_path: &str) -> SkylarkMarkerConfig {
    let mut config = SkylarkMarkerConfig {
        relative_path: None,
        errors: Vec::new(),
    };
    let Ok(data) = serde_yaml::from_str::<Value>(raw) else {
        return config;
    };
    let Value::Mapping(obj) = data else {
        return config;
    };
    let Some(value) = obj.get(Value::String("skylark".to_string())) else {
        return config;
    };
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                config.errors.push(ValidationError::new(
                    marker_path,
                    "skylark: expected a non-empty string",
                ));
            } else {
                config.relative_path = Some(trimmed.to_string());
            }
        }
        _ => config.errors.push(ValidationError::new(
            marker_path,
            "skylark: expected a non-empty string",
        )),
    }
    config
}

/// Load the Skylark file named by `relative_path` and check each requirement id.
#[shallgraph::implements("GRD-VALID-007")]
pub fn validate_requirement_ids(
    project_root: &Path,
    relative_path: &str,
    marker_path: &str,
    requirements: &[RequirementWithSource],
) -> Vec<ValidationError> {
    let resolved = match resolve_skylark_file(project_root, relative_path) {
        Ok(path) => path,
        Err(message) => return vec![ValidationError::new(marker_path, message)],
    };
    let source = match std::fs::read_to_string(&resolved) {
        Ok(source) => source,
        Err(err) => {
            return vec![ValidationError::new(
                resolved.display().to_string(),
                format!("Skylark: {err}"),
            )];
        }
    };
    validate_requirement_ids_from_source(&resolved.display().to_string(), &source, requirements)
}

/// GRD-VALID-007: Evaluate Skylark source and check each requirement id.
///
/// `validate_requirement_id` receives the id. `None` and `True` accept it. `False` rejects it.
/// A string rejects it and is included in the error. Any other return value is an error.
#[shallgraph::implements("GRD-VALID-007")]
pub fn validate_requirement_ids_from_source(
    source_path: &str,
    source: &str,
    requirements: &[RequirementWithSource],
) -> Vec<ValidationError> {
    let ast = match AstModule::parse(source_path, source.to_owned(), &Dialect::Standard) {
        Ok(ast) => ast,
        Err(err) => return vec![skylark_error(source_path, &err)],
    };
    let module = Module::new();
    let mut eval = Evaluator::new(&module);
    if let Err(err) = eval.eval_module(ast, &Globals::standard()) {
        return vec![skylark_error(source_path, &err)];
    }
    let Some(function) = module.get(VALIDATE_REQUIREMENT_ID) else {
        return vec![ValidationError::new(
            source_path,
            format!("Skylark file must define {VALIDATE_REQUIREMENT_ID}(requirement_id)"),
        )];
    };
    if function.get_type() != "function" {
        return vec![ValidationError::new(
            source_path,
            format!(
                "Skylark: {VALIDATE_REQUIREMENT_ID} must be a function (got {})",
                function.get_type()
            ),
        )];
    }

    let mut errors = Vec::new();
    for requirement in requirements {
        let arg = module.heap().alloc_str(&requirement.id).to_value();
        match eval.eval_function(function, &[arg], &[]) {
            Ok(value) => {
                if let Some(message) = rejection_message(&requirement.id, value) {
                    errors.push(requirement_error(requirement, message));
                }
            }
            Err(err) => errors.push(requirement_error(
                requirement,
                format!(
                    "requirement id \"{}\" failed Skylark validation: {err}",
                    requirement.id
                ),
            )),
        }
    }
    errors
}

fn resolve_skylark_file(project_root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("skylark: path must be relative to the project root".to_string());
    }
    let resolved = normalize_path(&project_root.join(path));
    if !resolved.exists() {
        return Err(format!("skylark: file not found: {relative_path}"));
    }
    if !resolved.is_file() {
        return Err(format!("skylark: not a file: {relative_path}"));
    }
    Ok(resolved)
}

fn skylark_error(path: &str, err: &starlark::Error) -> ValidationError {
    ValidationError::new(path, format!("Skylark: {err}"))
}

fn rejection_message(requirement_id: &str, value: StarlarkValue) -> Option<String> {
    if value.is_none() {
        return None;
    }
    if let Some(accepted) = value.unpack_bool() {
        return if accepted {
            None
        } else {
            Some(format!(
                "requirement id \"{requirement_id}\" failed Skylark validation"
            ))
        };
    }
    if let Some(message) = value.unpack_str() {
        let detail = message.trim();
        return Some(if detail.is_empty() {
            format!("requirement id \"{requirement_id}\" failed Skylark validation")
        } else {
            format!("requirement id \"{requirement_id}\" failed Skylark validation: {detail}")
        });
    }
    Some(format!(
        "requirement id \"{requirement_id}\" failed Skylark validation: {VALIDATE_REQUIREMENT_ID} returned {}",
        value.get_type()
    ))
}

fn requirement_error(requirement: &RequirementWithSource, message: String) -> ValidationError {
    let path = requirement.source_path.display().to_string();
    match requirement.document_line {
        Some(line) => ValidationError::with_line(path, message, line),
        None => ValidationError::new(path, message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_requirements;
    use crate::types::Requirement;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("shallgraph-skylark-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("requirements")).unwrap();
        dir
    }

    fn req(id: &str) -> RequirementWithSource {
        RequirementWithSource::from_requirement(
            Requirement {
                id: id.to_string(),
                title: format!("Title {id}"),
                require: "The system shall test.".to_string(),
                refinement: "".into(),
                attributes: None,
                links: None,
                satisfied_by: None,
                verified_by: None,
                parameters: None,
                tags: None,
            },
            PathBuf::from(format!("/requirements/{id}.req.yml")),
        )
    }

    fn write_req(dir: &Path, id: &str) {
        fs::write(
            dir.join("requirements").join(format!("{id}.req.yml")),
            format!("id: {id}\ntitle: Title {id}\nrequire: The system shall test.\n"),
        )
        .unwrap();
    }

    fn example_source() -> String {
        fs::read_to_string(example_path()).unwrap()
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn example_accepts_grd_some_010_and_rejects_grd_some_10() {
        let source = example_source();
        let errors = validate_requirement_ids_from_source(
            "grd_requirement_id.skylark",
            &source,
            &[
                req("GRD-SOME-010"),
                req("GRD-SOME-THING-010"),
                req("GRD-A-001"),
            ],
        );
        assert!(errors.is_empty(), "{errors:?}");

        let rejected = [
            "GRD-some_10",
            "GRD-some-010",
            "GRD-SOME-10",
            "GRD-010",
            "GRD-SOME_THING-010",
            "grd-SOME-010",
        ];
        for id in rejected {
            let errors = validate_requirement_ids_from_source(
                "grd_requirement_id.skylark",
                &source,
                &[req(id)],
            );
            assert_eq!(errors.len(), 1, "{id}: {errors:?}");
            assert!(
                errors[0].message.contains(id),
                "{id}: {}",
                errors[0].message
            );
            assert!(
                errors[0].message.contains("failed Skylark validation"),
                "{id}: {}",
                errors[0].message
            );
        }
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn omitted_key_skips_id_check() {
        let config =
            skylark_from_marker_yaml("requirement_dirs:\n  - requirements\n", "shallgraph.yaml");
        assert!(config.relative_path.is_none());
        assert!(config.errors.is_empty());

        let root = temp_project();
        fs::write(
            root.join("shallgraph.yaml"),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        write_req(&root, "SYS-001");
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn bad_marker_value_is_reported_on_the_marker() {
        let config = skylark_from_marker_yaml(
            "requirement_dirs:\n  - requirements\nskylark: 1\n",
            "shallgraph.yaml",
        );
        assert!(config.relative_path.is_none());
        assert_eq!(config.errors.len(), 1);
        assert_eq!(config.errors[0].path, "shallgraph.yaml");
        assert!(config.errors[0].message.contains("non-empty string"));

        let empty = skylark_from_marker_yaml(
            "requirement_dirs:\n  - requirements\nskylark: \"  \"\n",
            "shallgraph.yaml",
        );
        assert_eq!(empty.errors.len(), 1);
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn missing_file_absolute_path_and_syntax_error() {
        let root = temp_project();
        let missing = validate_requirement_ids(&root, "missing.skylark", "shallgraph.yaml", &[]);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].message.contains("file not found"));
        assert_eq!(missing[0].path, "shallgraph.yaml");

        let absolute = validate_requirement_ids(&root, "/tmp/ids.skylark", "shallgraph.yaml", &[]);
        assert!(absolute[0].message.contains("relative"));

        fs::create_dir(root.join("not-a-file.skylark")).unwrap();
        let directory =
            validate_requirement_ids(&root, "not-a-file.skylark", "shallgraph.yaml", &[]);
        assert!(directory[0].message.contains("not a file"));

        let syntax = validate_requirement_ids_from_source("bad.skylark", "def (\n", &[]);
        assert_eq!(syntax.len(), 1);
        assert!(syntax[0].message.starts_with("Skylark:"));
        assert_eq!(syntax[0].path, "bad.skylark");
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn missing_function_wrong_type_and_runtime_error() {
        let missing = validate_requirement_ids_from_source("ids.skylark", "x = 1\n", &[req("A")]);
        assert!(missing[0]
            .message
            .contains("must define validate_requirement_id"));

        let not_fn = validate_requirement_ids_from_source(
            "ids.skylark",
            "validate_requirement_id = 1\n",
            &[req("A")],
        );
        assert!(not_fn[0].message.contains("must be a function"));

        let exploded = validate_requirement_ids_from_source(
            "ids.skylark",
            "def validate_requirement_id(requirement_id):\n  fail('boom')\n",
            &[req("A")],
        );
        assert_eq!(exploded.len(), 1);
        assert!(exploded[0].message.contains("boom"));
        assert!(exploded[0].path.contains("A.req.yml"));
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn return_values_none_true_false_and_string() {
        let source = r#"
def validate_requirement_id(requirement_id):
    if requirement_id == "OK-NONE":
        return None
    if requirement_id == "OK-TRUE":
        return True
    if requirement_id == "BAD-FALSE":
        return False
    if requirement_id == "BAD-MSG":
        return "needs a prefix"
    if requirement_id == "BAD-BLANK":
        return "  "
    return 1
"#;
        let errors = validate_requirement_ids_from_source(
            "ids.skylark",
            source,
            &[
                req("OK-NONE"),
                req("OK-TRUE"),
                req("BAD-FALSE"),
                req("BAD-MSG"),
                req("BAD-BLANK"),
                req("BAD-TYPE"),
            ],
        );
        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors
            .iter()
            .any(|e| { e.message == "requirement id \"BAD-FALSE\" failed Skylark validation" }));
        assert!(errors
            .iter()
            .any(|e| { e.message == "requirement id \"BAD-BLANK\" failed Skylark validation" }));
        assert!(errors
            .iter()
            .any(|e| e.message.contains("BAD-MSG") && e.message.contains("needs a prefix")));
        assert!(errors
            .iter()
            .any(|e| e.message.contains("BAD-TYPE") && e.message.contains("int")));
    }

    #[shallgraph::verifies("GRD-VALID-007")]
    #[test]
    fn load_applies_example_file() {
        let root = temp_project();
        fs::copy(example_path(), root.join("ids.skylark")).unwrap();
        fs::write(
            root.join("shallgraph.yaml"),
            "requirement_dirs:\n  - requirements\nskylark: ids.skylark\n",
        )
        .unwrap();
        write_req(&root, "GRD-SOME-010");
        write_req(&root, "GRD-some_10");
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert_eq!(result.requirements.len(), 2);
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].message.contains("GRD-some_10"));
        assert!(result.errors[0].path.contains("GRD-some_10.req.yml"));
    }

    fn example_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/grd_requirement_id.skylark")
    }
}
