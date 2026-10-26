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

/// GRD-SYS-021: Supporting detail, either prose or an ordered claim outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refinement {
    /// A Markdown string. An empty string is a missing refinement.
    Prose(String),
    /// Ordered claims, details, and assertions. An empty list is still a list.
    Claims(Vec<Claim>),
}

/// One claim in a refinement outline. A string item has no nested refinement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub text: String,
    pub refinement: Refinement,
}

impl Default for Refinement {
    fn default() -> Self {
        Refinement::Prose(String::new())
    }
}

impl From<&str> for Refinement {
    fn from(value: &str) -> Self {
        Refinement::Prose(value.to_string())
    }
}

impl From<String> for Refinement {
    fn from(value: String) -> Self {
        Refinement::Prose(value)
    }
}

impl PartialEq<&str> for Refinement {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, Refinement::Prose(text) if text == other)
    }
}

impl PartialEq<String> for Refinement {
    fn eq(&self, other: &String) -> bool {
        matches!(self, Refinement::Prose(text) if text == other)
    }
}

impl Refinement {
    /// True when this is empty prose, which documents omit.
    pub fn is_empty_prose(&self) -> bool {
        matches!(self, Refinement::Prose(text) if text.is_empty())
    }

    /// True when there is nothing to render.
    pub fn is_render_blank(&self) -> bool {
        match self {
            Refinement::Prose(text) => text.is_empty(),
            Refinement::Claims(claims) => claims.is_empty(),
        }
    }

    /// GRD-SYS-021: Citation marker for a claim at `depth` (0-based) and `index` (0-based).
    /// The series is `(a)`, `(1)`, `(A)`, `(i)`, `(I)`, then it repeats.
    pub fn outline_marker(depth: usize, index: usize) -> String {
        let n = index + 1;
        let body = match depth % 5 {
            0 => to_alpha(n, false),
            1 => n.to_string(),
            2 => to_alpha(n, true),
            3 => to_roman(n, false),
            _ => to_roman(n, true),
        };
        format!("({body})")
    }

    /// Citation of a claim, such as `GRD-CLI-002(b)(1)`. `path` is 0-based indexes from the root.
    pub fn citation(requirement_id: &str, path: &[usize]) -> String {
        let mut out = requirement_id.to_string();
        for (depth, index) in path.iter().enumerate() {
            out.push_str(&Self::outline_marker(depth, *index));
        }
        out
    }
}

fn to_alpha(n: usize, upper: bool) -> String {
    let mut x = n;
    let mut chars = Vec::new();
    while x > 0 {
        x -= 1;
        chars.push((b'a' + (x % 26) as u8) as char);
        x /= 26;
    }
    chars.reverse();
    let text: String = chars.into_iter().collect();
    if upper {
        text.to_ascii_uppercase()
    } else {
        text
    }
}

fn to_roman(mut n: usize, upper: bool) -> String {
    const VALUES: [usize; 13] = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    const LOWER: [&str; 13] = [
        "m", "cm", "d", "cd", "c", "xc", "l", "xl", "x", "ix", "v", "iv", "i",
    ];
    let mut out = String::new();
    for (value, lower) in VALUES.iter().zip(LOWER.iter()) {
        while n >= *value {
            out.push_str(lower);
            n -= value;
        }
    }
    if upper {
        out.to_ascii_uppercase()
    } else {
        out
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
    /// GRD-SYS-021: Prose (Markdown) or an ordered claim outline.
    pub refinement: Refinement,
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
    pub refinement: Refinement,
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
