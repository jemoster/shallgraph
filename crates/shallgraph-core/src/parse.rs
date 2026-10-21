//! GRD-SYS-001 / GRD-SYS-009: Parse requirement YAML from content or disk.
//! GRD-SYS-020: A requirement file may contain several YAML documents.

use std::path::Path;

use serde_yaml::Value;

use crate::schema::parse_requirement_value;
use crate::types::{RequirementWithSource, ValidationError};

/// Requirements and errors from one requirement file (GRD-SYS-020).
#[derive(Debug, Clone, PartialEq)]
pub struct RequirementDocumentParse {
    pub requirements: Vec<RequirementWithSource>,
    pub errors: Vec<ValidationError>,
}

/// Parse YAML object data into a requirement (GRD-SYS-010).
pub fn parse_requirement_data(
    data: &Value,
    file_path: &Path,
) -> Result<RequirementWithSource, ValidationError> {
    match parse_requirement_value(data) {
        Ok(req) => Ok(RequirementWithSource::from_requirement(
            req,
            file_path.to_path_buf(),
        )),
        Err(message) => Err(ValidationError::new(
            file_path.display().to_string(),
            message,
        )),
    }
}

/// Parse requirement YAML from a content string (GRD-SYS-010 profiles).
/// A file with more than one requirement document is an error here; use
/// [`parse_requirement_documents`] to load every document.
pub fn parse_requirement_content(
    content: &str,
    file_path: &Path,
) -> Result<RequirementWithSource, ValidationError> {
    let parsed = parse_requirement_documents(content, file_path);
    if let Some(err) = parsed.errors.first() {
        return Err(err.clone());
    }
    match parsed.requirements.len() {
        1 => Ok(parsed.requirements.into_iter().next().unwrap()),
        0 => Err(ValidationError::new(
            file_path.display().to_string(),
            "Requirement file contains no requirement document",
        )),
        _ => Err(ValidationError::new(
            file_path.display().to_string(),
            "Requirement file contains multiple requirement documents",
        )),
    }
}

/// GRD-SYS-020: Load every requirement document from a YAML stream.
/// Documents are separated by a line whose only content is `---`. A leading
/// `---` does not add a document. A trailing `---` is ignored. An empty
/// document between two requirement documents is rejected. Each loaded
/// requirement shares `file_path`.
#[shallgraph::implements("GRD-SYS-020")]
pub fn parse_requirement_documents(content: &str, file_path: &Path) -> RequirementDocumentParse {
    let path = file_path.display().to_string();
    let raw_docs = split_yaml_stream(content);
    if raw_docs.is_empty() {
        return RequirementDocumentParse {
            requirements: Vec::new(),
            errors: vec![ValidationError::with_line(
                path,
                "Requirement file contains no requirement document",
                1,
            )],
        };
    }

    let mut slots: Vec<DocSlot> = Vec::new();
    for doc in raw_docs {
        if is_ignorable_document(&doc.body) {
            slots.push(DocSlot::Empty {
                line: doc.marker_line,
            });
            continue;
        }
        match serde_yaml::from_str::<Value>(&doc.body) {
            Ok(Value::Null) => slots.push(DocSlot::Empty {
                line: doc.marker_line,
            }),
            Ok(value) => slots.push(DocSlot::Value {
                line: doc.body_line,
                value,
            }),
            Err(err) => {
                let line = err
                    .location()
                    .map(|loc| doc.body_line + loc.line() - 1)
                    .unwrap_or(doc.body_line);
                slots.push(DocSlot::Invalid {
                    line,
                    message: err.to_string(),
                });
            }
        }
    }

    let mut errors = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        let DocSlot::Empty { line } = slot else {
            continue;
        };
        let has_before = slots[..index].iter().any(DocSlot::occupies);
        let has_after = slots[index + 1..].iter().any(DocSlot::occupies);
        if has_before && has_after {
            errors.push(ValidationError::with_line(
                path.clone(),
                "Empty requirement document between requirements",
                *line,
            ));
        }
    }

    for slot in &slots {
        if let DocSlot::Invalid { line, message } = slot {
            errors.push(ValidationError::with_line(
                path.clone(),
                message.clone(),
                *line,
            ));
        }
    }

    let mut requirements = Vec::new();
    for slot in &slots {
        let DocSlot::Value { line, value } = slot else {
            continue;
        };
        match parse_requirement_data(value, file_path) {
            Ok(mut req) => {
                req.document_line = Some(*line);
                requirements.push(req);
            }
            Err(err) => {
                errors.push(ValidationError::with_line(err.path, err.message, *line));
            }
        }
    }

    let multi_document = requirements.len() > 1;
    for req in &mut requirements {
        req.multi_document = multi_document;
    }

    if requirements.is_empty() && errors.is_empty() {
        errors.push(ValidationError::with_line(
            path,
            "Requirement file contains no requirement document",
            1,
        ));
    }

    RequirementDocumentParse {
        requirements,
        errors,
    }
}

/// Parse a single requirement file. Errors when the file contains more than one
/// requirement document; use [`parse_requirement_documents_file`] to load every document.
#[cfg(feature = "std-fs")]
pub fn parse_requirement_file(file_path: &Path) -> Result<RequirementWithSource, ValidationError> {
    let raw = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(err) => {
            return Err(ValidationError::new(
                file_path.display().to_string(),
                err.to_string(),
            ));
        }
    };
    parse_requirement_content(&raw, file_path)
}

/// GRD-SYS-020: Read a requirement file and load every requirement document in it.
#[cfg(feature = "std-fs")]
pub fn parse_requirement_documents_file(file_path: &Path) -> RequirementDocumentParse {
    let raw = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(err) => {
            return RequirementDocumentParse {
                requirements: Vec::new(),
                errors: vec![ValidationError::new(
                    file_path.display().to_string(),
                    err.to_string(),
                )],
            };
        }
    };
    parse_requirement_documents(&raw, file_path)
}

enum DocSlot {
    Empty { line: usize },
    Value { line: usize, value: Value },
    Invalid { line: usize, message: String },
}

impl DocSlot {
    fn occupies(&self) -> bool {
        !matches!(self, DocSlot::Empty { .. })
    }
}

struct RawDocument {
    /// 1-based line of the first body line. Serde locations are relative to this.
    body_line: usize,
    /// 1-based line of the document-start marker, or `body_line` when there is none.
    marker_line: usize,
    body: String,
}

#[derive(Clone, Copy)]
enum DocMarker {
    Start,
    End,
}

fn is_doc_marker(line: &str) -> Option<DocMarker> {
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let (kind, rest) = if let Some(rest) = line.strip_prefix("---") {
        (DocMarker::Start, rest)
    } else if let Some(rest) = line.strip_prefix("...") {
        (DocMarker::End, rest)
    } else {
        return None;
    };
    if rest.is_empty() || rest.starts_with([' ', '\t']) {
        Some(kind)
    } else {
        None
    }
}

fn is_ignorable_document(body: &str) -> bool {
    body.lines().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty() || trimmed.starts_with('#')
    })
}

fn split_yaml_stream(content: &str) -> Vec<RawDocument> {
    let mut docs = Vec::new();
    let mut body: Vec<String> = Vec::new();
    let mut body_line = 1usize;
    let mut marker_line: Option<usize> = None;
    let mut open = false;

    let mut line_no = 1usize;
    let mut rest = content;
    if !content.is_empty() {
        loop {
            let (line, next) = match rest.find('\n') {
                Some(i) => {
                    let (raw, after) = rest.split_at(i);
                    (raw.trim_end_matches('\r'), &after[1..])
                }
                None => (rest.trim_end_matches('\r'), ""),
            };
            match is_doc_marker(line) {
                Some(DocMarker::Start) => {
                    if open {
                        docs.push(finish_doc(marker_line, body_line, &body));
                        body.clear();
                    }
                    marker_line = Some(line_no);
                    body_line = line_no + 1;
                    open = true;
                }
                Some(DocMarker::End) => {
                    if open {
                        docs.push(finish_doc(marker_line, body_line, &body));
                        body.clear();
                    }
                    marker_line = None;
                    body_line = line_no + 1;
                    open = false;
                }
                None => {
                    if !open {
                        open = true;
                        marker_line = None;
                        body_line = line_no;
                    }
                    body.push(line.to_string());
                }
            }
            if next.is_empty() {
                break;
            }
            rest = next;
            line_no += 1;
        }
    }
    if open {
        docs.push(finish_doc(marker_line, body_line, &body));
    }
    docs
}

fn finish_doc(marker_line: Option<usize>, body_line: usize, body: &[String]) -> RawDocument {
    let marker_line = marker_line.unwrap_or(body_line);
    RawDocument {
        body_line,
        marker_line,
        body: body.join("\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn path() -> &'static Path {
        Path::new("/project/requirements/bundle.req.yml")
    }

    fn one(id: &str, title: &str, require: &str) -> String {
        format!("id: {id}\ntitle: {title}\nrequire: {require}\n")
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn loads_every_document_and_shares_the_source_path() {
        let yaml = format!(
            "{}\n---\n{}",
            one(
                "SYS-001",
                "End-to-End Encryption",
                "The system shall encrypt all video streams."
            ),
            one(
                "SYS-002",
                "Multi-Stream Support",
                "The backend shall support concurrent streaming."
            )
        );
        let parsed = parse_requirement_documents(&yaml, path());
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.requirements.len(), 2);
        assert!(parsed.requirements.iter().all(|r| r.multi_document));
        assert!(parsed.requirements.iter().all(|r| r.source_path == path()));
        assert_eq!(parsed.requirements[0].id, "SYS-001");
        assert_eq!(parsed.requirements[1].id, "SYS-002");
        assert_eq!(parsed.requirements[0].document_line, Some(1));
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn leading_and_trailing_markers_do_not_add_documents() {
        let yaml = format!(
            "---\n{}\n...\n---\n",
            one("SYS-001", "One", "The system shall stay single.")
        );
        let parsed = parse_requirement_documents(&yaml, Path::new("/project/SYS-001.req.yml"));
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(parsed.requirements.len(), 1);
        assert!(!parsed.requirements[0].multi_document);
        assert_eq!(parsed.requirements[0].id, "SYS-001");
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn document_end_marker_and_trailing_start_do_not_add_requirements() {
        let yaml = format!(
            "{}\n...\n---\n{}\n---\n",
            one("SYS-001", "One", "The system shall be first."),
            one("SYS-002", "Two", "The system shall be second.")
        );
        let parsed = parse_requirement_documents(&yaml, path());
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        assert_eq!(
            parsed
                .requirements
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["SYS-001", "SYS-002"]
        );
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn empty_document_between_requirements_is_rejected() {
        let yaml = format!(
            "{}\n---\n---\n{}",
            one("SYS-001", "One", "The system shall be first."),
            one("SYS-002", "Two", "The system shall be second.")
        );
        let parsed = parse_requirement_documents(&yaml, path());
        assert!(parsed
            .errors
            .iter()
            .any(|e| e.message.contains("Empty requirement document") && e.line.is_some()));
        assert_eq!(parsed.requirements.len(), 2);
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn empty_file_is_rejected() {
        let parsed = parse_requirement_documents("", path());
        assert_eq!(parsed.requirements.len(), 0);
        assert!(parsed.errors[0].message.contains("no requirement document"));
        assert_eq!(parsed.errors[0].line, Some(1));
    }

    #[shallgraph::verifies("GRD-SYS-020")]
    #[test]
    fn schema_error_reports_the_document_line() {
        let yaml = format!(
            "{}\n---\ntitle: Missing id\n",
            one("SYS-001", "One", "The system shall be first.")
        );
        let parsed = parse_requirement_documents(&yaml, path());
        assert!(parsed.requirements.iter().any(|r| r.id == "SYS-001"));
        let err = parsed
            .errors
            .iter()
            .find(|e| e.message.contains("id"))
            .expect("schema error");
        assert!(err.line.is_some());
        assert_eq!(err.path, path().display().to_string());
    }

    #[test]
    fn single_document_api_rejects_multiple_documents() {
        let yaml = format!(
            "{}\n---\n{}",
            one("SYS-001", "One", "The system shall be first."),
            one("SYS-002", "Two", "The system shall be second.")
        );
        let err = parse_requirement_content(&yaml, path()).unwrap_err();
        assert!(err.message.contains("multiple requirement documents"));
    }
}
