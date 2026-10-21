//! Core types for shallgraph requirement documents (GRD-SYS-001).

use std::collections::BTreeMap;
use std::path::PathBuf;

use indexmap::IndexMap;

/// GRD-SYS-005: Parameter value type (string, number, or boolean).
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

impl ParameterValue {
    pub fn as_display_string(&self) -> String {
        match self {
            ParameterValue::String(s) => s.clone(),
            ParameterValue::Integer(n) => n.to_string(),
            ParameterValue::Float(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            ParameterValue::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
        }
    }
}

/// Link object on a requirement (e.g. `satisfies`). GRD-SYS-001.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Link {
    pub satisfies: Option<String>,
    /// Additional link keys (e.g. `refines`) mapped to JSON values.
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl Link {
    /// All string-valued link targets (satisfies plus extra string fields).
    pub fn string_targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        if let Some(id) = &self.satisfies {
            if !id.is_empty() {
                targets.push(id.clone());
            }
        }
        for (key, value) in &self.extra {
            if key == "satisfies" {
                continue;
            }
            if let Some(s) = value.as_str() {
                if !s.is_empty() {
                    targets.push(s.to_string());
                }
            }
        }
        targets
    }
}

/// GRD-SYS-016: Reference to an implementation or verification artifact (path or URL).
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRef {
    pub artifact: String,
    pub description: Option<String>,
}

/// Kind of a collected source-link record.
#[shallgraph::implements("GRD-SYS-017")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceLinkKind {
    Implements,
    Verifies,
}

/// GRD-SYS-017: Collected association of a requirement with a located source artifact.
#[shallgraph::implements("GRD-SYS-017")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLink {
    pub requirement_id: String,
    pub kind: SourceLinkKind,
    pub path: String,
    pub item: String,
    pub linespace: Vec<u32>,
}

impl SourceLink {
    /// Build a record, keeping `linespace` unique and strictly increasing.
    /// Returns `None` when `linespace` is empty.
    pub fn new(
        requirement_id: impl Into<String>,
        kind: SourceLinkKind,
        path: impl Into<String>,
        item: impl Into<String>,
        mut linespace: Vec<u32>,
    ) -> Option<Self> {
        if linespace.is_empty() {
            return None;
        }
        linespace.sort_unstable();
        linespace.dedup();
        Some(Self {
            requirement_id: requirement_id.into(),
            kind,
            path: path.into(),
            item: item.into(),
            linespace,
        })
    }
}

/// Requirement shape for YAML files. Runtime validation is enforced by the schema
/// in `schema` (GRD-SYS-009); this struct is the Rust contract.
#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    pub id: String,
    pub title: String,
    /// Single normative Shall/Should/May statement for this requirement ID.
    pub require: String,
    /// Supporting prose (Markdown in HTML reports).
    pub refinement: String,
    pub attributes: Option<IndexMap<String, serde_json::Value>>,
    pub links: Option<Vec<Link>>,
    /// GRD-SYS-016: Artifacts that implement or satisfy this requirement.
    pub satisfied_by: Option<Vec<ArtifactRef>>,
    /// GRD-SYS-016: Artifacts that verify this requirement was met.
    pub verified_by: Option<Vec<ArtifactRef>>,
    /// GRD-SYS-005: Named parameters for templating in text fields.
    pub parameters: Option<IndexMap<String, ParameterValue>>,
    /// GRD-SYS-019: Custom tag entries. `None` when the `tags` key is omitted.
    pub tags: Option<IndexMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RequirementWithSource {
    pub id: String,
    pub title: String,
    pub require: String,
    pub refinement: String,
    pub attributes: Option<IndexMap<String, serde_json::Value>>,
    pub links: Option<Vec<Link>>,
    pub satisfied_by: Option<Vec<ArtifactRef>>,
    pub verified_by: Option<Vec<ArtifactRef>>,
    pub parameters: Option<IndexMap<String, ParameterValue>>,
    /// GRD-SYS-019: Custom tag entries. `None` when the `tags` key is omitted.
    pub tags: Option<IndexMap<String, serde_json::Value>>,
    /// Path to the YAML file this requirement was loaded from.
    pub source_path: PathBuf,
    /// GRD-SYS-004: Path segments from the requirement_dir that contains this file
    /// to the file's directory (relative). Empty = file is directly under a requirement_dir.
    pub category_path: Option<Vec<String>>,
    /// GRD-SYS-020: True when this requirement was loaded from a file that contains
    /// more than one requirement document. Filename/id matching is skipped in that case.
    pub multi_document: bool,
    /// GRD-SYS-020: 1-based line where this requirement document starts.
    pub document_line: Option<usize>,
}

impl RequirementWithSource {
    pub fn from_requirement(req: Requirement, source_path: PathBuf) -> Self {
        Self {
            id: req.id,
            title: req.title,
            require: req.require,
            refinement: req.refinement,
            attributes: req.attributes,
            links: req.links,
            satisfied_by: req.satisfied_by,
            verified_by: req.verified_by,
            parameters: req.parameters,
            tags: req.tags,
            source_path,
            category_path: None,
            multi_document: false,
            document_line: None,
        }
    }

    /// Payload for canonical YAML serialization (GRD-SYS-011); omits source metadata.
    pub fn to_requirement(&self) -> Requirement {
        Requirement {
            id: self.id.clone(),
            title: self.title.clone(),
            require: self.require.clone(),
            refinement: self.refinement.clone(),
            attributes: self.attributes.clone(),
            links: self.links.clone(),
            satisfied_by: self.satisfied_by.clone(),
            verified_by: self.verified_by.clone(),
            parameters: self.parameters.clone(),
            tags: self.tags.clone(),
        }
    }
}

pub struct ProjectInfo {
    /// Absolute path to the project root (directory containing shallgraph.yaml or shallgraph.yml).
    pub root_dir: PathBuf,
    /// Absolute paths to requirement YAML files under the project.
    pub requirement_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub line: Option<usize>,
}

impl ValidationError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            line: None,
        }
    }

    pub fn with_line(path: impl Into<String>, message: impl Into<String>, line: usize) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            line: Some(line),
        }
    }
}

pub struct LoadResult {
    pub requirements: Vec<RequirementWithSource>,
    pub errors: Vec<ValidationError>,
}

/// GRD-SYS-019: One custom tag defined in the project root marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomTagDef {
    pub name: String,
    pub values: Vec<String>,
    pub required: bool,
    pub multiple: bool,
    pub allow_null: bool,
    pub allow_empty: bool,
    pub allow_null_entry: bool,
    pub max_entries: Option<u64>,
}

/// Optional inputs when the JSON Schema must reflect project runtime configuration (GRD-SYS-009).
/// Profiles (GRD-SYS-010) pass compose options from project configuration into schema export.
/// GRD-SYS-019: `tags` carries valid custom-tag definitions for the project.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequirementSchemaComposeOptions {
    pub tags: Vec<CustomTagDef>,
}
