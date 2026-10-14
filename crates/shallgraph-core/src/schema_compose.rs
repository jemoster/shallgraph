//! GRD-CLI-005 / GRD-SYS-009: Build compose options for the requirement JSON Schema from the
//! project root marker, when configuration affects the exported schema.

#[cfg(feature = "std-fs")]
use std::path::Path;

#[cfg(feature = "std-fs")]
use crate::discovery::find_root_marker_path;

#[cfg(feature = "std-fs")]
use crate::tags::{compose_options_from_tag_set, custom_tags_from_marker_yaml};

#[cfg(feature = "std-fs")]
use crate::types::RequirementSchemaComposeOptions;

/// GRD-SYS-009 / GRD-SYS-019: custom tags in the project root marker become schema compose options.
#[cfg(feature = "std-fs")]
pub fn requirement_schema_compose_options_for_project(
    project_root: &Path,
) -> Option<RequirementSchemaComposeOptions> {
    let marker = find_root_marker_path(project_root)?;
    let raw = std::fs::read_to_string(&marker).ok()?;
    let set = custom_tags_from_marker_yaml(&raw, &marker.display().to_string());
    compose_options_from_tag_set(&set)
}

#[cfg(all(test, feature = "std-fs"))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("shallgraph-compose-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn returns_none_without_marker() {
        let dir = temp_dir();
        assert!(requirement_schema_compose_options_for_project(&dir).is_none());
    }

    #[test]
    fn does_not_throw_when_marker_exists() {
        let dir = temp_dir();
        fs::write(
            dir.join("shallgraph.yaml"),
            "requirement_dirs:\n  - requirements\n",
        )
        .unwrap();
        let _ = requirement_schema_compose_options_for_project(&dir);
    }
}
