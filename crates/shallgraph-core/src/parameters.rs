//! GRD-SYS-005: Requirement parameterization — template resolution.
//! GRD-SYS-006: Template syntax reflects Jinja2-style: double curly braces denote template;
//! quoted string inside {{ }} is a literal and is not processed by the template engine.
//! Syntax: {{ "literal" }} / {{ 'literal' }}, {{ :parameter_name }} (local), {{ requirement_id:parameter_name }} (cross-requirement).

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::types::RequirementWithSource;

/// Segment of resolved text: either plain or a parameter value with source.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSegment {
    pub kind: SegmentKind,
    pub text: String,
    /// Set when kind is Param: requirement id where the parameter is defined.
    pub source_req_id: Option<String>,
    /// Set when kind is Param: parameter name.
    pub param_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Plain,
    Param,
}

/// GRD-SYS-006: Unescape a double-quoted literal (\\ → \, \" → ").
fn unescape_double_quoted(s: &str) -> String {
    unescape_quoted(s, '"')
}

/// GRD-SYS-006: Unescape a single-quoted literal (\\ → \, \' → ').
fn unescape_single_quoted(s: &str) -> String {
    unescape_quoted(s, '\'')
}

fn unescape_quoted(s: &str, quote: char) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                if next == quote || next == '\\' {
                    out.push(next);
                } else {
                    out.push('\\');
                    out.push(next);
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn combined_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"\{\{\s*(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|:([A-Za-z0-9_-]+)|([A-Za-z0-9][A-Za-z0-9-]*):([A-Za-z0-9_-]+))\s*\}\}"#,
        )
        .expect("parameter template regex")
    })
}

/// GRD-SYS-005: Resolve template references in text to segments (plain or parameter value).
/// GRD-SYS-006: Quoted strings inside {{ }} are emitted as plain literals (not processed).
/// Unresolved parameter references are left as placeholder text so export remains unambiguous.
pub fn resolve_to_segments(
    text: &str,
    current_req_id: &str,
    requirements_by_id: &HashMap<String, &RequirementWithSource>,
) -> Vec<ResolvedSegment> {
    let mut segments: Vec<ResolvedSegment> = Vec::new();
    let mut last_index = 0usize;
    let re = combined_re();

    for caps in re.captures_iter(text) {
        let full = caps.get(0).unwrap();
        if last_index < full.start() {
            segments.push(ResolvedSegment {
                kind: SegmentKind::Plain,
                text: text[last_index..full.start()].to_string(),
                source_req_id: None,
                param_name: None,
            });
        }

        if let Some(double_quoted) = caps.get(1) {
            segments.push(ResolvedSegment {
                kind: SegmentKind::Plain,
                text: unescape_double_quoted(double_quoted.as_str()),
                source_req_id: None,
                param_name: None,
            });
            last_index = full.end();
            continue;
        }
        if let Some(single_quoted) = caps.get(2) {
            segments.push(ResolvedSegment {
                kind: SegmentKind::Plain,
                text: unescape_single_quoted(single_quoted.as_str()),
                source_req_id: None,
                param_name: None,
            });
            last_index = full.end();
            continue;
        }

        let (source_req_id, param_name, value) = if let Some(local) = caps.get(3) {
            let param_name = local.as_str().to_string();
            let req = requirements_by_id.get(current_req_id);
            let value = req
                .and_then(|r| r.parameters.as_ref())
                .and_then(|p| p.get(&param_name))
                .map(|v| v.as_display_string())
                .unwrap_or_else(|| format!("[param :{param_name} not found]"));
            (current_req_id.to_string(), param_name, value)
        } else if let (Some(cross_id), Some(cross_param)) = (caps.get(4), caps.get(5)) {
            let source_req_id = cross_id.as_str().to_string();
            let param_name = cross_param.as_str().to_string();
            let req = requirements_by_id.get(&source_req_id);
            let value = req
                .and_then(|r| r.parameters.as_ref())
                .and_then(|p| p.get(&param_name))
                .map(|v| v.as_display_string())
                .unwrap_or_else(|| format!("[param {source_req_id}:{param_name} not found]"));
            (source_req_id, param_name, value)
        } else {
            last_index = full.end();
            continue;
        };

        segments.push(ResolvedSegment {
            kind: SegmentKind::Param,
            text: value,
            source_req_id: Some(source_req_id),
            param_name: Some(param_name),
        });
        last_index = full.end();
    }

    if last_index < text.len() {
        segments.push(ResolvedSegment {
            kind: SegmentKind::Plain,
            text: text[last_index..].to_string(),
            source_req_id: None,
            param_name: None,
        });
    }

    if segments.is_empty() && !text.is_empty() {
        segments.push(ResolvedSegment {
            kind: SegmentKind::Plain,
            text: text.to_string(),
            source_req_id: None,
            param_name: None,
        });
    }

    segments
}

/// GRD-SYS-005: Resolve template references in text to a single string (all params substituted).
pub fn resolve_text(
    text: &str,
    current_req_id: &str,
    requirements_by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    resolve_to_segments(text, current_req_id, requirements_by_id)
        .into_iter()
        .map(|s| s.text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ParameterValue, Requirement, RequirementWithSource};
    use indexmap::IndexMap;
    use std::path::PathBuf;

    fn r(id: &str, params: Option<IndexMap<String, ParameterValue>>) -> RequirementWithSource {
        RequirementWithSource::from_requirement(
            Requirement {
                id: id.to_string(),
                title: "T".to_string(),
                require: "The system shall test.".to_string(),
                refinement: String::new(),
                attributes: None,
                links: None,
                satisfied_by: None,
                verified_by: None,
                parameters: params,
                tags: None,
            },
            PathBuf::from(format!("/{id}.req.yml")),
        )
    }

    fn map_one(req: &RequirementWithSource) -> HashMap<String, &RequirementWithSource> {
        let mut m = HashMap::new();
        m.insert(req.id.clone(), req);
        m
    }

    #[test]
    fn returns_plain_segment_when_no_template() {
        let req = r(
            "A",
            Some(IndexMap::from([(
                "x".into(),
                ParameterValue::String("1".into()),
            )])),
        );
        let by_id = map_one(&req);
        let segs = resolve_to_segments("hello world", "A", &by_id);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].kind, SegmentKind::Plain);
        assert_eq!(segs[0].text, "hello world");
    }

    #[test]
    fn resolves_local_parameter() {
        let req = r(
            "R",
            Some(IndexMap::from([
                ("limit".into(), ParameterValue::Integer(10)),
                ("name".into(), ParameterValue::String("foo".into())),
            ])),
        );
        let by_id = map_one(&req);
        let segs = resolve_to_segments("Limit is {{ :limit }} and {{ :name }}.", "R", &by_id);
        assert_eq!(segs.len(), 5);
        assert_eq!(segs[0].text, "Limit is ");
        assert_eq!(segs[1].kind, SegmentKind::Param);
        assert_eq!(segs[1].text, "10");
        assert_eq!(segs[1].source_req_id.as_deref(), Some("R"));
        assert_eq!(segs[1].param_name.as_deref(), Some("limit"));
        assert_eq!(segs[2].text, " and ");
        assert_eq!(segs[3].text, "foo");
        assert_eq!(segs[4].text, ".");
    }

    #[test]
    fn resolves_cross_requirement() {
        let a = r(
            "GRD-A",
            Some(IndexMap::from([(
                "max".into(),
                ParameterValue::Integer(100),
            )])),
        );
        let b = r("GRD-B", None);
        let mut by_id = HashMap::new();
        by_id.insert(a.id.clone(), &a);
        by_id.insert(b.id.clone(), &b);
        let segs = resolve_to_segments("Max from A is {{ GRD-A:max }}.", "GRD-B", &by_id);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[1].kind, SegmentKind::Param);
        assert_eq!(segs[1].text, "100");
        assert_eq!(segs[1].source_req_id.as_deref(), Some("GRD-A"));
        assert_eq!(segs[1].param_name.as_deref(), Some("max"));
    }

    #[test]
    fn unresolved_placeholders() {
        let req = r("R", None);
        let by_id = map_one(&req);
        let segs = resolve_to_segments("Value {{ :missing }} here", "R", &by_id);
        assert_eq!(segs[1].text, "[param :missing not found]");
    }

    #[test]
    fn quoted_literals() {
        let req = r(
            "R",
            Some(IndexMap::from([(
                "x".into(),
                ParameterValue::String("X".into()),
            )])),
        );
        let by_id = map_one(&req);
        let segs = resolve_to_segments(r#"Prefix {{ "literal text" }} suffix"#, "R", &by_id);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[1].kind, SegmentKind::Plain);
        assert_eq!(segs[1].text, "literal text");

        let out = resolve_text(
            r#"Literal {{ "{{ :name }}" }} and param {{ :name }}"#,
            "R",
            &{
                let req2 = r(
                    "R",
                    Some(IndexMap::from([(
                        "name".into(),
                        ParameterValue::String("resolved".into()),
                    )])),
                );
                // can't mix lifetimes easily — redo
                let _ = req2;
                by_id
            },
        );
        // The previous by_id has x not name; dedicated test below.
        let _ = out;
    }

    #[test]
    fn does_not_substitute_inside_quoted_literals() {
        let req = r(
            "R",
            Some(IndexMap::from([(
                "name".into(),
                ParameterValue::String("resolved".into()),
            )])),
        );
        let by_id = map_one(&req);
        let out = resolve_text(
            r#"Literal {{ "{{ :name }}" }} and param {{ :name }}"#,
            "R",
            &by_id,
        );
        assert_eq!(out, "Literal {{ :name }} and param resolved");
    }

    #[test]
    fn unescapes_in_quoted_literals() {
        let req = r("R", None);
        let by_id = map_one(&req);
        let segs = resolve_to_segments(r#"{{ "a\\b\"c" }}"#, "R", &by_id);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, r#"a\b"c"#);

        let segs = resolve_to_segments(r#"{{ 'x\\y\'z' }}"#, "R", &by_id);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].text, r#"x\y'z"#);
    }

    #[test]
    fn resolve_text_concatenates() {
        let req = r(
            "R",
            Some(IndexMap::from([(
                "x".into(),
                ParameterValue::String("X".into()),
            )])),
        );
        let by_id = map_one(&req);
        assert_eq!(resolve_text("a {{ :x }} b", "R", &by_id), "a X b");
        let req = r(
            "R",
            Some(IndexMap::from([(
                "limit".into(),
                ParameterValue::Integer(10),
            )])),
        );
        let by_id = map_one(&req);
        assert_eq!(
            resolve_text(r#"Limit is {{ "max" }}: {{ :limit }}"#, "R", &by_id),
            "Limit is max: 10"
        );
    }

    #[test]
    fn coerces_number_and_boolean() {
        let req = r(
            "R",
            Some(IndexMap::from([
                ("n".into(), ParameterValue::Integer(42)),
                ("flag".into(), ParameterValue::Bool(true)),
            ])),
        );
        let by_id = map_one(&req);
        let segs = resolve_to_segments("n={{ :n }} flag={{ :flag }}", "R", &by_id);
        assert_eq!(segs[1].text, "42");
        assert_eq!(segs[3].text, "true");
    }
}
