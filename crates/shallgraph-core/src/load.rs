//! Load all requirements from a project directory.
//! GRD-SYS-002 / GRD-SYS-004 / GRD-SYS-010.

use crate::types::RequirementWithSource;

#[cfg(feature = "std-fs")]
use crate::discovery::{discover_project, discover_requirement_paths, get_requirement_dirs};
#[cfg(feature = "std-fs")]
use crate::error::Error;
#[cfg(feature = "std-fs")]
use crate::profile::load_active_profile;
#[cfg(feature = "std-fs")]
use crate::types::{LoadResult, ValidationError};
#[cfg(feature = "std-fs")]
use std::path::{Path, PathBuf};

/// GRD-SYS-004: Compute category path segments for a requirement from its
/// source_path and the project's requirement_dirs.
#[cfg(feature = "std-fs")]
fn category_path_for(source_path: &Path, requirement_dirs: &[PathBuf]) -> Vec<String> {
    let file_dir = source_path.parent().unwrap_or(source_path);
    for dir in requirement_dirs {
        if file_dir == dir || file_dir.starts_with(dir) {
            let rel = match file_dir.strip_prefix(dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rel.as_os_str().is_empty() || rel == Path::new(".") {
                return Vec::new();
            }
            return rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
        }
    }
    Vec::new()
}

/// Load all requirements from a project directory. If `project_root` is provided,
/// uses that root; otherwise discovers root from `start_dir`.
/// Parses each requirement file and validates (schema + duplicate ids + broken links).
/// GRD-SYS-004: Sets category_path on each requirement from directory structure.
/// GRD-SYS-010: Parsing and validation use the active profile from project configuration.
#[cfg(feature = "std-fs")]
pub fn load_requirements(
    start_dir: &Path,
    project_root: Option<&Path>,
) -> Result<LoadResult, Error> {
    let (root, requirement_paths) = if let Some(root) = project_root {
        (root.to_path_buf(), discover_requirement_paths(root)?)
    } else {
        let discovered = discover_project(start_dir)?;
        (discovered.root_dir, discovered.requirement_paths)
    };

    let profile = load_active_profile(&root)?;
    let requirement_dirs = get_requirement_dirs(&root)?;
    let mut requirements: Vec<RequirementWithSource> = Vec::new();
    let mut errors: Vec<ValidationError> = Vec::new();

    for file_path in requirement_paths {
        // GRD-SYS-020: each YAML document in the file is its own requirement.
        let raw = match std::fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(err) => {
                errors.push(ValidationError::new(
                    file_path.display().to_string(),
                    err.to_string(),
                ));
                continue;
            }
        };
        let parsed = profile.parse_requirement_documents(&raw, &file_path);
        errors.extend(parsed.errors);
        for mut req in parsed.requirements {
            req.category_path = Some(category_path_for(&file_path, &requirement_dirs));
            requirements.push(req);
        }
    }

    errors.extend(profile.validate_requirements(&requirements));
    // GRD-SYS-019: custom tag configuration and requirement tag values.
    if let Some(marker_path) = crate::discovery::find_root_marker_path(&root) {
        if let Ok(raw) = std::fs::read_to_string(&marker_path) {
            let tag_set =
                crate::tags::custom_tags_from_marker_yaml(&raw, &marker_path.display().to_string());
            errors.extend(tag_set.errors.clone());
            errors.extend(crate::tags::validate_requirement_tags(
                &tag_set,
                &requirements,
            ));
        }
    }
    Ok(LoadResult {
        requirements,
        errors,
    })
}

/// Get requirements with satisfies links resolved (same list; links are already on each requirement).
pub fn get_requirements_with_links(
    requirements: &[RequirementWithSource],
) -> Vec<RequirementWithSource> {
    requirements.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_requirement_content;
    use crate::requirement_files::REQUIREMENT_FILE_EXTENSION;
    use crate::ROOT_MARKER;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("shallgraph-load-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn category_path_from_directory_structure() {
        let root = temp_project();
        let html_dir = root.join("requirements/html-report");
        let sys_dir = root.join("requirements/sys");
        fs::create_dir_all(&html_dir).unwrap();
        fs::create_dir_all(&sys_dir).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        fs::write(
            html_dir.join(format!("GRD-HTML-001{REQUIREMENT_FILE_EXTENSION}")),
            "id: GRD-HTML-001\ntitle: HTML report\nrequire: The system shall produce a full report.\n",
        )
        .unwrap();
        fs::write(
            sys_dir.join(format!("GRD-SYS-001{REQUIREMENT_FILE_EXTENSION}")),
            "id: GRD-SYS-001\ntitle: Core\nrequire: The system shall provide core behavior.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(result.requirements.len(), 2);
        let html = result
            .requirements
            .iter()
            .find(|r| r.id == "GRD-HTML-001")
            .unwrap();
        let sys = result
            .requirements
            .iter()
            .find(|r| r.id == "GRD-SYS-001")
            .unwrap();
        assert_eq!(
            html.category_path.as_deref(),
            Some(["html-report".to_string()].as_slice())
        );
        assert_eq!(
            sys.category_path.as_deref(),
            Some(["sys".to_string()].as_slice())
        );
    }

    #[test]
    fn empty_category_path_when_directly_under_dir() {
        let root = temp_project();
        let reqs = root.join("reqs");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(root.join(ROOT_MARKER), "requirement_dirs:\n  - reqs\n").unwrap();
        fs::write(
            reqs.join(format!("GRD-TOP-001{REQUIREMENT_FILE_EXTENSION}")),
            "id: GRD-TOP-001\ntitle: Top\nrequire: The system shall be at top.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(
            result.requirements[0].category_path.as_deref(),
            Some([].as_slice() as &[String])
        );
    }

    #[test]
    fn nested_category_path() {
        let root = temp_project();
        let deep = root.join("requirements/cli/sub");
        fs::create_dir_all(&deep).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        fs::write(
            deep.join(format!("GRD-CLI-SUB-001{REQUIREMENT_FILE_EXTENSION}")),
            "id: GRD-CLI-SUB-001\ntitle: Nested\nrequire: The system shall be nested.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.is_empty());
        assert_eq!(
            result.requirements[0].category_path.as_deref(),
            Some(["cli".to_string(), "sub".to_string()].as_slice())
        );
    }

    #[test]
    fn parses_parameters_from_content() {
        let content = r#"
id: GRD-PARAM-001
title: Param requirement
require: Limit is {{ :limit }} shall apply.
refinement: ""
parameters:
  limit: 100
  name: foo
  enabled: true
"#;
        let result = parse_requirement_content(content, Path::new("/req.req.yml")).unwrap();
        let params = result.parameters.unwrap();
        assert_eq!(
            params.get("limit"),
            Some(&crate::types::ParameterValue::Integer(100))
        );
        assert_eq!(
            params.get("name"),
            Some(&crate::types::ParameterValue::String("foo".into()))
        );
        assert_eq!(
            params.get("enabled"),
            Some(&crate::types::ParameterValue::Bool(true))
        );
        assert!(result.require.contains("{{ :limit }}"));
    }

    #[shallgraph::verifies("GRD-SYS-020", "GRD-VALID-002", "GRD-VALID-003")]
    #[test]
    fn loads_multi_document_file_without_filename_match() {
        let root = temp_project();
        let reqs = root.join("requirements");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        fs::write(
            reqs.join(format!("bundle{REQUIREMENT_FILE_EXTENSION}")),
            "id: SYS-001\ntitle: One\nrequire: The system shall be first.\n---\nid: SYS-002\ntitle: Two\nrequire: The system shall be second.\n",
        )
        .unwrap();
        fs::write(
            reqs.join(format!("SYS-003{REQUIREMENT_FILE_EXTENSION}")),
            "---\nid: SYS-003\ntitle: Three\nrequire: The system shall be third.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.requirements.len(), 3);
        let bundled: Vec<_> = result
            .requirements
            .iter()
            .filter(|r| r.id == "SYS-001" || r.id == "SYS-002")
            .collect();
        assert_eq!(bundled.len(), 2);
        assert_eq!(bundled[0].id, "SYS-001");
        assert_eq!(bundled[1].id, "SYS-002");
        assert!(bundled.iter().all(|r| r.multi_document));
        assert_eq!(bundled[0].source_path, bundled[1].source_path);
        let single = result
            .requirements
            .iter()
            .find(|r| r.id == "SYS-003")
            .unwrap();
        assert!(!single.multi_document);
    }

    #[shallgraph::verifies("GRD-VALID-002")]
    #[test]
    fn single_document_filename_mismatch_still_fails() {
        let root = temp_project();
        let reqs = root.join("requirements");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        fs::write(
            reqs.join(format!("WRONG{REQUIREMENT_FILE_EXTENSION}")),
            "---\nid: SYS-009\ntitle: Mismatch\nrequire: The system shall match its filename.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("does not match filename")));
    }

    #[shallgraph::verifies("GRD-VALID-003", "GRD-SYS-020")]
    #[test]
    fn duplicate_id_inside_one_file_is_reported() {
        let root = temp_project();
        let reqs = root.join("requirements");
        fs::create_dir_all(&reqs).unwrap();
        fs::write(
            root.join(ROOT_MARKER),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        let file = reqs.join(format!("bundle{REQUIREMENT_FILE_EXTENSION}"));
        fs::write(
            &file,
            "id: SYS-001\ntitle: One\nrequire: The system shall be first.\n---\nid: SYS-001\ntitle: Again\nrequire: The system shall be first again.\n",
        )
        .unwrap();
        let result = load_requirements(&root, Some(&root)).unwrap();
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Duplicate requirement id") && e.line.is_some()));
    }
}
