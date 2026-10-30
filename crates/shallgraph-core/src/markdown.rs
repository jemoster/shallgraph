//! GRD-MD-001: Markdown report of the full set of information in requirement files.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use crate::artifact_links::{github_blob_url_for_path, ArtifactLinkRenderOptions};
use crate::parameters::{resolve_to_segments, SegmentKind};
use crate::types::{ArtifactRef, Refinement, RequirementWithSource, SourceLink, SourceLinkKind};

/// GRD-SYS-005: Placeholder character range for param spans (U+E000–E0FF); replaced after linking.
const PARAM_PLACEHOLDER_BASE: u32 = 0xe000;

struct IndexNode {
    requirements: Vec<usize>,
    children: BTreeMap<String, IndexNode>,
}

fn build_index_tree(requirements: &[RequirementWithSource]) -> IndexNode {
    let mut root = IndexNode {
        requirements: Vec::new(),
        children: BTreeMap::new(),
    };
    for (i, r) in requirements.iter().enumerate() {
        let path = r.category_path.as_deref().unwrap_or(&[]);
        let mut node = &mut root;
        for segment in path {
            node = node
                .children
                .entry(segment.clone())
                .or_insert_with(|| IndexNode {
                    requirements: Vec::new(),
                    children: BTreeMap::new(),
                });
        }
        node.requirements.push(i);
    }
    root
}

fn md_req_link(id: &str) -> String {
    format!("[{id}](#{id})")
}

fn render_index_node(
    node: &IndexNode,
    requirements: &[RequirementWithSource],
    by_id: &HashMap<String, &RequirementWithSource>,
    indent: &str,
) -> String {
    let mut parts = Vec::new();
    for (segment, child) in &node.children {
        parts.push(format!("{indent}- {segment}"));
        let child_indent = format!("{indent}  ");
        for &idx in &child.requirements {
            let r = &requirements[idx];
            let title = resolve_and_render_text(&r.title, &r.id, by_id, false);
            parts.push(format!("{child_indent}- {} – {title}", md_req_link(&r.id)));
        }
        let nested = render_index_node(child, requirements, by_id, &child_indent);
        if !nested.is_empty() {
            parts.push(nested);
        }
    }
    parts.join("\n")
}

/// GRD-MD-003: Top-level index of requirements grouped by category.
#[shallgraph::implements("GRD-MD-003")]
fn render_hierarchical_index(
    requirements: &[RequirementWithSource],
    by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    let root = build_index_tree(requirements);
    let mut parts = Vec::new();
    for &idx in &root.requirements {
        let r = &requirements[idx];
        let title = resolve_and_render_text(&r.title, &r.id, by_id, false);
        parts.push(format!("- {} – {title}", md_req_link(&r.id)));
    }
    let nested = render_index_node(&root, requirements, by_id, "");
    if !nested.is_empty() {
        parts.push(nested);
    }
    parts.join("\n")
}

fn format_attr_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn escape_table_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "<br>")
}

fn id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\b").expect("id pattern"))
}

fn protected_span_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)```.*?```|`[^`]*`|!\[[^\]]*\]\([^)]*\)|\[[^\]]*\]\([^)]*\)|<[^>]+>")
            .expect("protected span pattern")
    })
}

/// GRD-MD-006: Link requirement ID references in rendered text fields.
#[shallgraph::implements("GRD-MD-006")]
fn auto_link_requirement_refs(
    text: &str,
    by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    let mut protected: Vec<String> = Vec::new();
    let masked = protected_span_pattern().replace_all(text, |caps: &regex::Captures| {
        let idx = protected.len();
        protected.push(caps[0].to_string());
        let placeholder = char::from_u32(0xe100 + idx as u32).unwrap_or('\u{e100}');
        placeholder.to_string()
    });
    let re = id_pattern();
    let mut linked = re
        .replace_all(&masked, |caps: &regex::Captures| {
            let candidate = &caps[1];
            if by_id.contains_key(candidate) {
                md_req_link(candidate)
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();
    for (i, span) in protected.iter().enumerate() {
        let placeholder = char::from_u32(0xe100 + i as u32)
            .unwrap_or('\u{e100}')
            .to_string();
        linked = linked.replace(&placeholder, span);
    }
    linked
}

fn resolve_and_render_text(
    text: &str,
    current_req_id: &str,
    by_id: &HashMap<String, &RequirementWithSource>,
    use_markdown: bool,
) -> String {
    let segments = resolve_to_segments(text, current_req_id, by_id);
    let mut param_spans: Vec<String> = Vec::new();
    let mut resolved = String::new();
    for seg in segments {
        if seg.kind == SegmentKind::Plain {
            resolved.push_str(&seg.text);
        } else {
            let idx = param_spans.len();
            let placeholder =
                char::from_u32(PARAM_PLACEHOLDER_BASE + idx as u32).unwrap_or('\u{e000}');
            resolved.push(placeholder);
            let distinguished = format!("**{}**", seg.text);
            let span = if let Some(source) = &seg.source_req_id {
                format!("[{distinguished}](#{source})")
            } else {
                distinguished
            };
            param_spans.push(span);
        }
    }
    let mut out = if use_markdown {
        resolved.trim().to_string()
    } else {
        resolved
    };
    for (i, span) in param_spans.iter().enumerate() {
        let placeholder = char::from_u32(PARAM_PLACEHOLDER_BASE + i as u32)
            .unwrap_or('\u{e000}')
            .to_string();
        out = out.replace(&placeholder, span);
    }
    auto_link_requirement_refs(&out, by_id)
}

const META_ATTR_KEYS: &[&str] = &["status"];

/// GRD-MD-002: Collect requirement ids that link to each requirement (reverse lookup).
#[shallgraph::implements("GRD-MD-002")]
fn linked_from_map(requirements: &[RequirementWithSource]) -> HashMap<String, Vec<String>> {
    let id_set: HashSet<&str> = requirements.iter().map(|r| r.id.as_str()).collect();
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for r in requirements {
        for link in r.links.as_deref().unwrap_or(&[]) {
            for value in link.string_targets() {
                if value != r.id && id_set.contains(value.as_str()) {
                    let list = map.entry(value).or_default();
                    if !list.iter().any(|id| id == &r.id) {
                        list.push(r.id.clone());
                    }
                }
            }
        }
    }
    map
}

fn capitalize_label(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let rest: String = chars.flat_map(|c| c.to_lowercase()).collect();
            format!("{}{rest}", first.to_uppercase())
        }
    }
}

fn is_scalar_attr(v: &serde_json::Value) -> bool {
    matches!(
        v,
        serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Bool(_)
    )
}

fn github_file_href(
    path: &str,
    line_start: Option<u32>,
    line_end: Option<u32>,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> Option<String> {
    let github = artifact_links?.github.as_ref()?;
    Some(github_blob_url_for_path(path, github, line_start, line_end))
}

fn md_path(path: &str, href: Option<&str>) -> String {
    let code = format!("`{path}`");
    match href {
        Some(url) => format!("[{code}]({url})"),
        None => code,
    }
}

/// GRD-MD-008: Source YAML path is a GitHub blob link when origin is GitHub.
#[shallgraph::implements("GRD-MD-008")]
fn source_file_markdown(
    source_path: &str,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    match github_file_href(source_path, None, None, artifact_links) {
        Some(href) => format!("[{source_path}]({href})"),
        None => source_path.to_string(),
    }
}

fn artifact_refs_list_markdown(
    refs: &[ArtifactRef],
    by_id: &HashMap<String, &RequirementWithSource>,
    requirement_id: &str,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let mut items = Vec::new();
    for ref_item in refs {
        let artifact = ref_item.artifact.trim();
        if artifact.is_empty() {
            continue;
        }
        let lower = artifact.to_ascii_lowercase();
        let is_url = lower.starts_with("http://") || lower.starts_with("https://");
        let artifact_md = if is_url {
            format!("[{artifact}]({artifact})")
        } else {
            md_path(
                artifact,
                github_file_href(artifact, None, None, artifact_links).as_deref(),
            )
        };
        if let Some(desc) = ref_item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let desc_md = resolve_and_render_text(desc, requirement_id, by_id, false);
            items.push(format!("- {artifact_md} — {desc_md}"));
        } else {
            items.push(format!("- {artifact_md}"));
        }
    }
    items.join("\n")
}

fn format_linespace(lines: &[u32]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    let mut start = lines[0];
    let mut prev = lines[0];
    for &n in &lines[1..] {
        if n == prev + 1 {
            prev = n;
            continue;
        }
        parts.push(format_line_range(start, prev));
        start = n;
        prev = n;
    }
    parts.push(format_line_range(start, prev));
    parts.join(", ")
}

fn format_line_range(start: u32, end: u32) -> String {
    if start == end {
        format!("L{start}")
    } else {
        format!("L{start}–L{end}")
    }
}

fn source_link_item_markdown(
    link: &SourceLink,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let line_start = link.linespace.first().copied();
    let line_end = link.linespace.last().copied();
    let path_md = md_path(
        &link.path,
        github_file_href(link.path.as_str(), line_start, line_end, artifact_links).as_deref(),
    );
    format!(
        "- {path_md} {} {}",
        link.item,
        format_linespace(&link.linespace)
    )
}

/// GRD-MD-007: One Satisfied by / Verified by heading with origin groups (By comment, Rust).
#[shallgraph::implements("GRD-MD-007")]
fn combined_trace_section_markdown<'a, I>(
    heading: &str,
    yaml_refs: Option<&[ArtifactRef]>,
    source_links: I,
    by_id: &HashMap<String, &RequirementWithSource>,
    requirement_id: &str,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String
where
    I: IntoIterator<Item = &'a SourceLink>,
{
    let yaml_list = yaml_refs
        .filter(|refs| !refs.is_empty())
        .map(|refs| artifact_refs_list_markdown(refs, by_id, requirement_id, artifact_links))
        .unwrap_or_default();
    let source_list: String = source_links
        .into_iter()
        .map(|link| source_link_item_markdown(link, artifact_links))
        .collect::<Vec<_>>()
        .join("\n");
    let mut groups = Vec::new();
    if !yaml_list.is_empty() {
        groups.push(format!("*By comment*\n\n{yaml_list}"));
    }
    if !source_list.is_empty() {
        groups.push(format!("*Rust*\n\n{source_list}"));
    }
    if groups.is_empty() {
        return String::new();
    }
    format!("**{heading}**\n\n{}", groups.join("\n\n"))
}

fn source_links_for_requirement<'a>(
    source_links: &'a [SourceLink],
    requirement_id: &str,
) -> Vec<&'a SourceLink> {
    source_links
        .iter()
        .filter(|l| l.requirement_id == requirement_id)
        .collect()
}

fn labeled_block(label: &str, body: &str) -> String {
    format!("**{label}**\n\n{body}")
}

/// GRD-MD-009: Render a string refinement as Markdown, or a claim list as a numbered outline.
#[shallgraph::implements("GRD-MD-009")]
fn render_refinement_markdown(
    refinement: &Refinement,
    requirement_id: &str,
    by_id: &HashMap<String, &RequirementWithSource>,
    path: &[usize],
    indent: &str,
) -> String {
    match refinement {
        Refinement::Prose(text) if text.is_empty() => String::new(),
        Refinement::Prose(text) => {
            let rendered = resolve_and_render_text(text, requirement_id, by_id, true);
            if indent.is_empty() {
                rendered
            } else {
                indent_lines(&rendered, indent)
            }
        }
        Refinement::Claims(claims) if claims.is_empty() => String::new(),
        Refinement::Claims(claims) => {
            let depth = path.len();
            let mut lines = Vec::new();
            for (index, claim) in claims.iter().enumerate() {
                let mut claim_path = path.to_vec();
                claim_path.push(index);
                let marker = Refinement::outline_marker(depth, index);
                let text = resolve_and_render_text(&claim.text, requirement_id, by_id, true);
                lines.push(format_claim_item(indent, &marker, &text));
                let nested = render_refinement_markdown(
                    &claim.refinement,
                    requirement_id,
                    by_id,
                    &claim_path,
                    &format!("{indent}  "),
                );
                if !nested.is_empty() {
                    lines.push(nested);
                }
            }
            lines.join("\n")
        }
    }
}

fn format_claim_item(indent: &str, marker: &str, body: &str) -> String {
    let body = body.trim();
    let mut line_iter = body.lines();
    let first = line_iter.next().unwrap_or("");
    let mut out = format!("{indent}- {marker} {first}");
    let continuation = format!("{indent}  ");
    for line in line_iter {
        out.push('\n');
        if !line.is_empty() {
            out.push_str(&continuation);
        }
        out.push_str(line);
    }
    out
}

fn indent_lines(text: &str, indent: &str) -> String {
    text.trim()
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn requirement_detail_markdown(
    r: &RequirementWithSource,
    linked_from_ids: Option<&[String]>,
    by_id: &HashMap<String, &RequirementWithSource>,
    source_links: &[&SourceLink],
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let attrs = r.attributes.clone().unwrap_or_default();
    let attr_entries: Vec<(&String, &serde_json::Value)> =
        attrs.iter().filter(|(_, v)| is_scalar_attr(v)).collect();
    let meta_attr_parts: Vec<String> = attr_entries
        .iter()
        .filter(|(k, _)| META_ATTR_KEYS.contains(&k.as_str()))
        .map(|(k, v)| format!("{} {}", capitalize_label(k), format_attr_value(v)))
        .collect();
    let below_desc_attrs: Vec<(&String, &serde_json::Value)> = attr_entries
        .iter()
        .filter(|(k, _)| !META_ATTR_KEYS.contains(&k.as_str()))
        .copied()
        .collect();

    let mut satisfies_ids: Vec<String> = Vec::new();
    let mut other_link_parts: Vec<String> = Vec::new();
    for link in r.links.as_deref().unwrap_or(&[]) {
        if let Some(val) = &link.satisfies {
            let val_str = val.trim();
            if !val_str.is_empty() {
                satisfies_ids.push(val_str.to_string());
            }
        }
        for (key, val) in &link.extra {
            if key == "key" || key == "satisfies" {
                continue;
            }
            let val_str = match val {
                serde_json::Value::Null => continue,
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let val_str = val_str.trim();
            if val_str.is_empty() {
                continue;
            }
            let is_ref = Regex::new(r"^[A-Za-z0-9][A-Za-z0-9-]*$")
                .ok()
                .map(|re| re.is_match(val_str))
                .unwrap_or(false);
            let link_val = if is_ref && by_id.contains_key(val_str) {
                md_req_link(val_str)
            } else {
                val_str.to_string()
            };
            other_link_parts.push(format!("{} {link_val}", capitalize_label(key)));
        }
    }

    let meta_md = if meta_attr_parts.is_empty() {
        String::new()
    } else {
        meta_attr_parts.join(" | ")
    };

    let parameters_md = match &r.parameters {
        Some(params) if !params.is_empty() => {
            let mut rows = vec!["| Name | Value |".to_string(), "| --- | --- |".to_string()];
            for (name, value) in params {
                rows.push(format!(
                    "| {} | {} |",
                    escape_table_cell(name),
                    escape_table_cell(&value.as_display_string())
                ));
            }
            labeled_block("Parameters", &rows.join("\n"))
        }
        _ => String::new(),
    };

    let satisfies_md = if satisfies_ids.is_empty() {
        String::new()
    } else {
        let items: String = satisfies_ids
            .iter()
            .map(|id| format!("- {}", md_req_link(id)))
            .collect::<Vec<_>>()
            .join("\n");
        labeled_block("Satisfies", &items)
    };

    let linked_from_md = match linked_from_ids {
        Some(ids) if !ids.is_empty() => {
            let items: String = ids
                .iter()
                .map(|id| format!("- {}", md_req_link(id)))
                .collect::<Vec<_>>()
                .join("\n");
            labeled_block("Linked from", &items)
        }
        _ => String::new(),
    };

    let other_links_md = if other_link_parts.is_empty() {
        String::new()
    } else {
        labeled_block("Links", &other_link_parts.join(" | "))
    };

    let rationale_md: String = below_desc_attrs
        .iter()
        .map(|(k, v)| {
            let str_val = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let body = if k.as_str() == "rationale" {
                resolve_and_render_text(&str_val, &r.id, by_id, true)
            } else {
                resolve_and_render_text(&str_val, &r.id, by_id, false)
            };
            labeled_block(&capitalize_label(k), &body)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let satisfied_by_md = combined_trace_section_markdown(
        "Satisfied by",
        r.satisfied_by.as_deref(),
        source_links
            .iter()
            .copied()
            .filter(|l| l.kind == SourceLinkKind::Implements),
        by_id,
        &r.id,
        artifact_links,
    );
    let verified_by_md = combined_trace_section_markdown(
        "Verified by",
        r.verified_by.as_deref(),
        source_links
            .iter()
            .copied()
            .filter(|l| l.kind == SourceLinkKind::Verifies),
        by_id,
        &r.id,
        artifact_links,
    );

    let title_md = resolve_and_render_text(&r.title, &r.id, by_id, false);
    let require_md = resolve_and_render_text(&r.require, &r.id, by_id, false);
    let refinement_md = render_refinement_markdown(&r.refinement, &r.id, by_id, &[], "");
    let refinement_block = if refinement_md.is_empty() {
        String::new()
    } else {
        labeled_block("Refinement", &refinement_md)
    };

    let mut sections = vec![
        format!("<a id=\"{}\"></a>\n", r.id),
        format!("### {} – {title_md}", r.id),
    ];
    if !meta_md.is_empty() {
        sections.push(meta_md);
    }
    for block in [
        parameters_md,
        labeled_block("Require", &require_md),
        refinement_block,
        rationale_md,
        satisfied_by_md,
        verified_by_md,
        satisfies_md,
        linked_from_md,
        other_links_md,
        format!(
            "**Source file** {}",
            source_file_markdown(&r.source_path.display().to_string(), artifact_links)
        ),
    ] {
        if !block.is_empty() {
            sections.push(block);
        }
    }
    sections.join("\n\n")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnAlign {
    Left { colon: bool },
    Center,
    Right,
}

struct OpenFence {
    ch: char,
    len: usize,
}

/// GRD-MD-010: Pad Markdown table cells so column bars line up.
/// GRD-SYS-022 calls this for Markdown sections while formatting requirement YAML.
#[shallgraph::implements("GRD-MD-010")]
pub(crate) fn align_markdown_tables(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0;
    let mut fence: Option<OpenFence> = None;
    while index < lines.len() {
        let line = lines[index];
        if let Some(open) = &fence {
            out.push(line.to_string());
            if fence_closes(line, open) {
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some(open) = fence_opens(line) {
            fence = Some(open);
            out.push(line.to_string());
            index += 1;
            continue;
        }
        if let Some(span) = markdown_table_span(&lines[index..]) {
            out.extend(align_table(&lines[index..index + span]));
            index += span;
            continue;
        }
        out.push(line.to_string());
        index += 1;
    }
    out.join("\n")
}

fn fence_opens(line: &str) -> Option<OpenFence> {
    let trimmed = line.trim_start();
    let mut chars = trimmed.chars();
    let ch = chars.next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let mut len = 1usize;
    for next in chars {
        if next == ch {
            len += 1;
        } else {
            break;
        }
    }
    if len >= 3 {
        Some(OpenFence { ch, len })
    } else {
        None
    }
}

fn fence_closes(line: &str, open: &OpenFence) -> bool {
    let trimmed = line.trim_start();
    let mut len = 0usize;
    for ch in trimmed.chars() {
        if ch == open.ch {
            len += 1;
        } else {
            break;
        }
    }
    len >= open.len && trimmed.chars().skip(len).all(|ch| ch.is_whitespace())
}

fn markdown_table_span(lines: &[&str]) -> Option<usize> {
    if lines.len() < 2 || !is_table_row(lines[0]) || !is_delimiter_row(lines[1]) {
        return None;
    }
    let mut span = 2;
    while span < lines.len() && is_table_row(lines[span]) {
        span += 1;
    }
    Some(span)
}

fn is_table_row(line: &str) -> bool {
    split_table_cells(line).is_some_and(|cells| !cells.is_empty())
}

fn is_delimiter_row(line: &str) -> bool {
    split_table_cells(line)
        .is_some_and(|cells| !cells.is_empty() && cells.iter().all(|cell| is_delimiter_cell(cell)))
}

fn is_delimiter_cell(cell: &str) -> bool {
    let text = cell.trim();
    let mut chars = text.chars().peekable();
    if chars.peek() == Some(&':') {
        chars.next();
    }
    let mut dashes = 0;
    while chars.peek() == Some(&'-') {
        dashes += 1;
        chars.next();
    }
    if chars.peek() == Some(&':') {
        chars.next();
    }
    dashes >= 1 && chars.peek().is_none() && text.chars().count() >= 3
}

fn column_align(cell: &str) -> ColumnAlign {
    let text = cell.trim();
    let left = text.starts_with(':');
    let right = text.ends_with(':');
    match (left, right) {
        (true, true) => ColumnAlign::Center,
        (false, true) => ColumnAlign::Right,
        (true, false) => ColumnAlign::Left { colon: true },
        (false, false) => ColumnAlign::Left { colon: false },
    }
}

/// Split a Markdown table row on unescaped bars that are outside inline code.
/// Returns `None` when the line has no column separator.
fn split_table_cells(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = trimmed.chars().peekable();
    let mut in_code = false;
    let mut saw_separator = false;
    while let Some(ch) = chars.next() {
        if ch == '`' {
            in_code = !in_code;
            current.push(ch);
            continue;
        }
        if !in_code && ch == '\\' {
            if let Some(next) = chars.next() {
                current.push('\\');
                current.push(next);
                continue;
            }
        }
        if !in_code && ch == '|' {
            saw_separator = true;
            cells.push(std::mem::take(&mut current));
            continue;
        }
        current.push(ch);
    }
    if !saw_separator {
        return None;
    }
    cells.push(current);
    let starts_with_pipe = trimmed.starts_with('|');
    let ends_with_pipe = trimmed.ends_with('|') && !trimmed.ends_with("\\|");
    if starts_with_pipe && cells.first().is_some_and(|cell| cell.is_empty()) {
        cells.remove(0);
    }
    if ends_with_pipe && cells.last().is_some_and(|cell| cell.is_empty()) {
        cells.pop();
    }
    if cells.is_empty() {
        None
    } else {
        Some(cells)
    }
}

fn align_table(lines: &[&str]) -> Vec<String> {
    let indent_len = lines[0].len() - lines[0].trim_start().len();
    let indent = &lines[0][..indent_len];
    let rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| split_table_cells(line).unwrap_or_default())
        .collect();
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut aligns = vec![ColumnAlign::Left { colon: false }; column_count];
    if let Some(delimiter) = rows.get(1) {
        for (index, cell) in delimiter.iter().enumerate() {
            if index < column_count {
                aligns[index] = column_align(cell);
            }
        }
    }
    let mut widths = vec![3usize; column_count];
    for (row_index, row) in rows.iter().enumerate() {
        if row_index == 1 {
            continue;
        }
        for (index, cell) in row.iter().enumerate() {
            let width = cell.trim().chars().count();
            if index < widths.len() && width > widths[index] {
                widths[index] = width;
            }
        }
    }
    rows.iter()
        .enumerate()
        .map(|(row_index, row)| {
            let cells: Vec<String> = (0..column_count)
                .map(|index| {
                    let raw = row.get(index).map(String::as_str).unwrap_or("");
                    if row_index == 1 {
                        format_delimiter(widths[index], aligns[index])
                    } else {
                        pad_cell(raw.trim(), widths[index], aligns[index])
                    }
                })
                .collect();
            format!("{indent}| {} |", cells.join(" | "))
        })
        .collect()
}

fn format_delimiter(width: usize, align: ColumnAlign) -> String {
    let width = width.max(3);
    match align {
        ColumnAlign::Left { colon: false } => "-".repeat(width),
        ColumnAlign::Left { colon: true } => format!(":{}", "-".repeat(width - 1)),
        ColumnAlign::Right => format!("{}:", "-".repeat(width - 1)),
        ColumnAlign::Center => format!(":{}:", "-".repeat(width - 2)),
    }
}

fn pad_cell(content: &str, width: usize, align: ColumnAlign) -> String {
    let pad = width.saturating_sub(content.chars().count());
    match align {
        ColumnAlign::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{content}{}", " ".repeat(left), " ".repeat(right))
        }
        ColumnAlign::Right => format!("{}{content}", " ".repeat(pad)),
        ColumnAlign::Left { .. } => format!("{content}{}", " ".repeat(pad)),
    }
}

/// GRD-MD-001: Markdown report represents the full set of information in the requirements file.
pub fn generate_full_markdown(requirements: &[RequirementWithSource]) -> String {
    generate_full_markdown_with_source_links(requirements, &[])
}

/// GRD-MD-007: Present source-link records on each requirement (Satisfied by / Verified by).
pub fn generate_full_markdown_with_source_links(
    requirements: &[RequirementWithSource],
    source_links: &[SourceLink],
) -> String {
    generate_full_markdown_with_artifact_links(requirements, source_links, None)
}

/// GRD-MD-007 / GRD-MD-008: Full report with optional source-link records and GitHub file links.
#[shallgraph::implements("GRD-MD-001", "GRD-MD-004", "GRD-MD-005", "GRD-MD-007", "GRD-MD-008")]
pub fn generate_full_markdown_with_artifact_links(
    requirements: &[RequirementWithSource],
    source_links: &[SourceLink],
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let by_id: HashMap<String, &RequirementWithSource> =
        requirements.iter().map(|r| (r.id.clone(), r)).collect();
    let index_md = render_hierarchical_index(requirements, &by_id);
    let linked_from = linked_from_map(requirements);
    let details: String = requirements
        .iter()
        .map(|r| {
            let req_links = source_links_for_requirement(source_links, &r.id);
            requirement_detail_markdown(
                r,
                linked_from.get(&r.id).map(|v| v.as_slice()),
                &by_id,
                &req_links,
                artifact_links,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    align_markdown_tables(&format!(
        "# Requirements\n\nTotal: {count}\n\n## Index\n\n{index_md}\n\n## Details\n\n{details}\n",
        count = requirements.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_links::{ArtifactLinkRenderOptions, GithubArtifactLinkContext};
    use crate::types::{
        ArtifactRef, Link, ParameterValue, Requirement, RequirementWithSource, SourceLink,
        SourceLinkKind,
    };
    use indexmap::IndexMap;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn req(id: &str, title: &str) -> RequirementWithSource {
        RequirementWithSource::from_requirement(
            Requirement {
                id: id.to_string(),
                title: title.to_string(),
                require: "The system shall meet this requirement.".to_string(),
                refinement: "".into(),
                attributes: None,
                links: None,
                satisfied_by: None,
                verified_by: None,
                parameters: None,
                tags: None,
            },
            PathBuf::from(format!("/project/{id}.req.yml")),
        )
    }

    fn detail_for<'a>(md: &'a str, id: &str) -> &'a str {
        let marker = format!("<a id=\"{id}\"></a>");
        let start = md
            .find(&marker)
            .unwrap_or_else(|| panic!("missing detail section for {id}"));
        let after = start + marker.len();
        let end = md[after..]
            .find("\n<a id=\"")
            .map(|i| after + i)
            .unwrap_or(md.len());
        &md[start..end]
    }

    fn github_links() -> ArtifactLinkRenderOptions {
        ArtifactLinkRenderOptions {
            github: Some(GithubArtifactLinkContext {
                owner: "acme".into(),
                repo: "widgets".into(),
                commit_sha: "deadbeef".into(),
                project_root: "/workspace".into(),
                ..Default::default()
            }),
        }
    }

    #[shallgraph::verifies("GRD-MD-001")]
    #[test]
    fn full_report_index_and_details() {
        let md = generate_full_markdown(&[req("GRD-A-001", "First"), req("GRD-A-002", "Second")]);
        assert!(md.contains("# Requirements"));
        assert!(md.contains("Total: 2"));
        assert!(md.contains("## Index"));
        assert!(md.contains("## Details"));
        assert!(md.contains("[GRD-A-001](#GRD-A-001)"));
        assert!(md.contains("<a id=\"GRD-A-001\"></a>"));
        assert!(md.contains("First"));
        assert!(md.contains("**Require**"));
    }

    #[shallgraph::verifies("GRD-MD-001")]
    #[test]
    fn includes_attributes_and_links() {
        let mut r = req("GRD-MD-001", "Markdown report");
        r.attributes = Some(IndexMap::from([
            ("status".into(), serde_json::json!("active")),
            (
                "rationale".into(),
                serde_json::json!("Markdown output is easily consumed."),
            ),
        ]));
        let md = generate_full_markdown(&[r]);
        assert!(md.contains("Status"));
        assert!(md.contains("active"));
        assert!(md.contains("**Rationale**"));
        assert!(md.contains("Markdown output is easily consumed."));
    }

    #[shallgraph::verifies("GRD-MD-005")]
    #[test]
    fn parameters_table() {
        let mut r = req("GRD-TBL-001", "Table requirement");
        r.parameters = Some(IndexMap::from([
            ("alpha".into(), ParameterValue::String("one".into())),
            ("beta".into(), ParameterValue::String("two".into())),
        ]));
        let md = generate_full_markdown(&[r]);
        assert!(md.contains("**Parameters**"));
        assert!(md.contains(
            "| Name  | Value |\n| ----- | ----- |\n| alpha | one   |\n| beta  | two   |"
        ));
        assert_table_columns_aligned(&md);
    }

    #[shallgraph::verifies("GRD-MD-005")]
    #[test]
    fn no_parameters_section_when_absent() {
        let md = generate_full_markdown(&[req("GRD-NOP-001", "No params")]);
        assert!(!md.contains("**Parameters**"));
    }

    #[shallgraph::verifies("GRD-MD-005")]
    #[test]
    fn parameters_keep_yaml_insertion_order() {
        let mut r = req("GRD-ORD-001", "Order");
        r.parameters = Some(IndexMap::from([
            (
                "native_binary_os".into(),
                ParameterValue::String("Linux".into()),
            ),
            (
                "native_binary_arch".into(),
                ParameterValue::String("x86_64".into()),
            ),
        ]));
        let md = generate_full_markdown(&[r]);
        let os = md.find("| native_binary_os").expect("os row");
        let arch = md.find("| native_binary_arch").expect("arch row");
        assert!(os < arch, "parameter rows should keep insertion order");
        assert_table_columns_aligned(&md);
    }

    #[shallgraph::verifies("GRD-MD-002")]
    #[test]
    fn reverse_lookup() {
        let a = req("GRD-A", "Target");
        let mut b = req("GRD-B", "Linker");
        b.links = Some(vec![Link {
            satisfies: Some("GRD-A".into()),
            extra: BTreeMap::new(),
        }]);
        let mut c = req("GRD-C", "Also linker");
        c.links = Some(vec![Link {
            satisfies: Some("GRD-A".into()),
            extra: BTreeMap::new(),
        }]);
        let md = generate_full_markdown(&[a, b, c]);
        let detail = detail_for(&md, "GRD-A");
        assert!(detail.contains("**Linked from**"));
        assert!(detail.contains("[GRD-B](#GRD-B)"));
        assert!(detail.contains("[GRD-C](#GRD-C)"));
    }

    #[shallgraph::verifies("GRD-MD-003")]
    #[test]
    fn hierarchical_index() {
        let mut a = req("GRD-HTML-001", "HTML report");
        a.category_path = Some(vec!["html-report".into()]);
        let mut b = req("GRD-HTML-002", "Linked from");
        b.category_path = Some(vec!["html-report".into()]);
        let mut c = req("GRD-SYS-001", "Core");
        c.category_path = Some(vec!["sys".into()]);
        let md = generate_full_markdown(&[a, b, c]);
        let idx = &md[md.find("## Index").unwrap()..md.find("## Details").unwrap()];
        assert!(idx.contains("html-report"));
        assert!(idx.contains("sys"));
        assert!(idx.contains("[GRD-HTML-001](#GRD-HTML-001)"));
    }

    #[shallgraph::verifies("GRD-MD-004")]
    #[test]
    fn markdown_refinement_and_rationale() {
        let mut r = req("GRD-FMT-001", "Title");
        r.refinement = "Plain and **bold** and *italic* text.".into();
        let md = generate_full_markdown(&[r]);
        assert!(md.contains("Plain and **bold** and *italic* text."));

        let mut r = req("GRD-FMT-002", "Title");
        r.refinement = "Desc".into();
        r.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("Reason with `code` and **emphasis**."),
        )]));
        let md = generate_full_markdown(&[r]);
        assert!(md.contains("Reason with `code` and **emphasis**."));
    }

    #[shallgraph::verifies("GRD-MD-009")]
    #[test]
    fn markdown_claim_outline_keeps_string_refinements_unnumbered() {
        let mut prose = req("GRD-MD-PROSE", "Prose");
        prose.refinement = "* still text".into();
        let md = generate_full_markdown(&[prose]);
        assert!(md.contains("* still text"));
        assert!(!md.contains("(a)"));

        let mut listed = req("GRD-MD-LIST", "List");
        listed.refinement = Refinement::Claims(vec![
            crate::types::Claim {
                text: "with a list view".into(),
                refinement: Refinement::default(),
            },
            crate::types::Claim {
                text: "in the working directory".into(),
                refinement: Refinement::Claims(vec![crate::types::Claim {
                    text: "The directory shall be created.".into(),
                    refinement: Refinement::default(),
                }]),
            },
        ]);
        let md = generate_full_markdown(&[listed]);
        assert!(md.contains("- (a) with a list view"));
        assert!(md.contains("- (b) in the working directory"));
        assert!(md.contains("- (1) The directory shall be created."));
    }

    #[shallgraph::verifies("GRD-MD-006")]
    #[test]
    fn auto_link_known_ids() {
        let mut holder = req("GRD-REF-001", "Reference holder");
        holder.refinement = "See GRD-HTML-001 for base behavior.".into();
        holder.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("Also depends on GRD-HTML-002."),
        )]));
        let md = generate_full_markdown(&[
            req("GRD-HTML-001", "Target 1"),
            req("GRD-HTML-002", "Target 2"),
            holder,
        ]);
        let detail = detail_for(&md, "GRD-REF-001");
        assert!(detail.contains("[GRD-HTML-001](#GRD-HTML-001)"));
        assert!(detail.contains("[GRD-HTML-002](#GRD-HTML-002)"));
    }

    #[shallgraph::verifies("GRD-MD-006")]
    #[test]
    fn auto_link_skips_existing_markdown_links_and_code() {
        let mut holder = req("GRD-REF-002", "Protected");
        holder.refinement =
            "See [GRD-HTML-001](#GRD-HTML-001) and `GRD-HTML-002` plus GRD-HTML-001 again.".into();
        let md = generate_full_markdown(&[
            req("GRD-HTML-001", "Target 1"),
            req("GRD-HTML-002", "Target 2"),
            holder,
        ]);
        let detail = detail_for(&md, "GRD-REF-002");
        assert!(detail.contains("[GRD-HTML-001](#GRD-HTML-001)"));
        assert!(detail.contains("`GRD-HTML-002`"));
        assert!(!detail.contains("[`GRD-HTML-002`](#GRD-HTML-002)"));
        assert!(!detail.contains("[[GRD-HTML-001](#GRD-HTML-001)](#GRD-HTML-001)"));
    }

    #[shallgraph::verifies("GRD-MD-001")]
    #[test]
    fn param_values_in_markdown() {
        let mut r = req("GRD-P-001", "Limit is {{ :limit }}");
        r.refinement = "The maximum count is {{ :limit }} items.".into();
        r.parameters = Some(IndexMap::from([(
            "limit".into(),
            ParameterValue::Integer(42),
        )]));
        let md = generate_full_markdown(&[r]);
        assert!(md.contains("**42**"));
        assert!(md.contains("Limit is"));
        assert!(md.contains("The maximum count is"));
        assert!(md.contains("[**42**](#GRD-P-001)"));
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn artifacts() {
        let mut r = req("GRD-ART-001", "Artifacts");
        r.satisfied_by = Some(vec![
            ArtifactRef {
                artifact: "packages/core/src/foo.ts".into(),
                description: Some("Implements the feature.".into()),
            },
            ArtifactRef {
                artifact: "https://example.com/evidence".into(),
                description: None,
            },
        ]);
        r.verified_by = Some(vec![ArtifactRef {
            artifact: "packages/core/test/foo.test.ts".into(),
            description: None,
        }]);
        let md = generate_full_markdown(&[r]);
        let detail = detail_for(&md, "GRD-ART-001");
        assert!(detail.contains("**Satisfied by**"));
        assert!(detail.contains("*By comment*"));
        assert!(detail.contains("`packages/core/src/foo.ts`"));
        assert!(detail.contains("Implements the feature."));
        assert!(detail.contains("[https://example.com/evidence](https://example.com/evidence)"));
        assert!(detail.contains("**Verified by**"));
        assert!(!detail.contains("*Rust*"));
        assert!(!detail.contains("Implemented by"));
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn presents_source_links_by_kind() {
        let r = req("GRD-MD-007", "Source links");
        let implements = SourceLink::new(
            "GRD-MD-007",
            SourceLinkKind::Implements,
            "src/markdown.rs",
            "function",
            vec![10, 11, 12],
        )
        .unwrap();
        let verifies = SourceLink::new(
            "GRD-MD-007",
            SourceLinkKind::Verifies,
            "src/markdown.rs",
            "test",
            vec![80],
        )
        .unwrap();
        let other = SourceLink::new(
            "OTHER",
            SourceLinkKind::Implements,
            "src/other.rs",
            "function",
            vec![1],
        )
        .unwrap();
        let md = generate_full_markdown_with_source_links(&[r], &[implements, verifies, other]);
        let detail = detail_for(&md, "GRD-MD-007");
        assert!(detail.contains("**Satisfied by**"));
        assert!(detail.contains("*Rust*"));
        assert!(detail.contains("`src/markdown.rs`"));
        assert!(detail.contains("function"));
        assert!(detail.contains("L10–L12"));
        assert!(detail.contains("**Verified by**"));
        assert!(detail.contains("test"));
        assert!(detail.contains("L80"));
        assert!(!detail.contains("src/other.rs"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("*By comment*"));
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn omits_source_link_sections_without_records() {
        let md = generate_full_markdown(&[req("GRD-NONE-001", "No links")]);
        let detail = detail_for(&md, "GRD-NONE-001");
        assert!(!detail.contains("**Satisfied by**"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("**Verified by**"));
        assert!(!detail.contains("*By comment*"));
        assert!(!detail.contains("*Rust*"));
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn verified_by_combines_yaml_and_source_links() {
        let mut r = req("GRD-MIX-001", "Mixed");
        r.verified_by = Some(vec![ArtifactRef {
            artifact: "test/foo.test.ts".into(),
            description: None,
        }]);
        let verifies = SourceLink::new(
            "GRD-MIX-001",
            SourceLinkKind::Verifies,
            "src/lib.rs",
            "test",
            vec![4],
        )
        .unwrap();
        let md = generate_full_markdown_with_source_links(&[r], &[verifies]);
        let detail = detail_for(&md, "GRD-MIX-001");
        assert_eq!(detail.matches("**Verified by**").count(), 1);
        assert!(detail.contains("*By comment*"));
        assert!(detail.contains("*Rust*"));
        assert!(detail.contains("`test/foo.test.ts`"));
        assert!(detail.contains("`src/lib.rs`"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("**Satisfied by**"));
        let comment_pos = detail.find("*By comment*").unwrap();
        let rust_pos = detail.find("*Rust*").unwrap();
        assert!(comment_pos < rust_pos);
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn satisfied_by_combines_yaml_and_source_links() {
        let mut r = req("GRD-MIX-002", "Mixed impl");
        r.satisfied_by = Some(vec![ArtifactRef {
            artifact: "src/feature.ts".into(),
            description: Some("Primary implementation.".into()),
        }]);
        let implements = SourceLink::new(
            "GRD-MIX-002",
            SourceLinkKind::Implements,
            "src/lib.rs",
            "function",
            vec![8, 9, 10],
        )
        .unwrap();
        let md = generate_full_markdown_with_source_links(&[r], &[implements]);
        let detail = detail_for(&md, "GRD-MIX-002");
        assert_eq!(detail.matches("**Satisfied by**").count(), 1);
        assert!(detail.contains("*By comment*"));
        assert!(detail.contains("*Rust*"));
        assert!(detail.contains("`src/feature.ts`"));
        assert!(detail.contains("Primary implementation."));
        assert!(detail.contains("`src/lib.rs`"));
        assert!(detail.contains("L8–L10"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("**Verified by**"));
    }

    #[shallgraph::verifies("GRD-MD-007")]
    #[test]
    fn format_linespace_compresses_ranges() {
        assert_eq!(format_linespace(&[3]), "L3");
        assert_eq!(format_linespace(&[3, 4, 5]), "L3–L5");
        assert_eq!(format_linespace(&[1, 3, 4, 8]), "L1, L3–L4, L8");
    }

    #[shallgraph::verifies("GRD-MD-008")]
    #[test]
    fn github_links_wrap_source_file_yaml_artifacts_and_source_links() {
        let mut r = req("GRD-MD-008", "GitHub links");
        r.source_path = PathBuf::from("/workspace/requirements/md/GRD-MD-008.req.yml");
        r.satisfied_by = Some(vec![
            ArtifactRef {
                artifact: "crates/shallgraph-core/src/markdown.rs".into(),
                description: Some("Renders the report.".into()),
            },
            ArtifactRef {
                artifact: "https://example.com/evidence".into(),
                description: None,
            },
        ]);
        let implements = SourceLink::new(
            "GRD-MD-008",
            SourceLinkKind::Implements,
            "crates/shallgraph-core/src/markdown.rs",
            "function",
            vec![10, 11, 12],
        )
        .unwrap();
        let md =
            generate_full_markdown_with_artifact_links(&[r], &[implements], Some(&github_links()));
        let detail = detail_for(&md, "GRD-MD-008");
        assert!(detail.contains(
            "](https://github.com/acme/widgets/blob/deadbeef/requirements/md/GRD-MD-008.req.yml)"
        ));
        assert!(detail.contains(
            "](https://github.com/acme/widgets/blob/deadbeef/crates/shallgraph-core/src/markdown.rs)"
        ));
        assert!(detail.contains(
            "](https://github.com/acme/widgets/blob/deadbeef/crates/shallgraph-core/src/markdown.rs#L10-L12)"
        ));
        assert!(detail.contains("`crates/shallgraph-core/src/markdown.rs`"));
        assert!(detail.contains("[https://example.com/evidence](https://example.com/evidence)"));
        assert!(!detail.contains("cursor://"));
        assert!(!detail.contains("vscode://"));
        assert!(!detail.contains("target=\"_blank\""));
    }

    #[shallgraph::verifies("GRD-MD-008")]
    #[test]
    fn file_paths_stay_plain_without_github_context() {
        let mut r = req("GRD-MD-008", "No GitHub");
        r.source_path = PathBuf::from("/workspace/requirements/md/GRD-MD-008.req.yml");
        r.satisfied_by = Some(vec![ArtifactRef {
            artifact: "src/foo.ts".into(),
            description: None,
        }]);
        let md = generate_full_markdown(&[r]);
        assert!(!md.contains("github.com"));
        assert!(md.contains("`src/foo.ts`"));
        assert!(md.contains("/workspace/requirements/md/GRD-MD-008.req.yml"));
        assert!(!md.contains("](/workspace/requirements/md/GRD-MD-008.req.yml)"));
    }

    fn assert_table_columns_aligned(markdown: &str) {
        let mut fence: Option<OpenFence> = None;
        let lines: Vec<&str> = markdown.split('\n').collect();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            if let Some(open) = &fence {
                if fence_closes(line, open) {
                    fence = None;
                }
                index += 1;
                continue;
            }
            if let Some(open) = fence_opens(line) {
                fence = Some(open);
                index += 1;
                continue;
            }
            if let Some(span) = markdown_table_span(&lines[index..]) {
                let positions: Vec<Vec<usize>> = lines[index..index + span]
                    .iter()
                    .map(|row| unescaped_pipe_positions(row))
                    .collect();
                let expected = &positions[0];
                for row in &positions[1..] {
                    assert_eq!(row, expected, "table columns are not aligned:\n{markdown}");
                }
                index += span;
                continue;
            }
            index += 1;
        }
    }

    fn unescaped_pipe_positions(line: &str) -> Vec<usize> {
        let mut positions = Vec::new();
        let mut chars = line.char_indices().peekable();
        let mut in_code = false;
        while let Some((offset, ch)) = chars.next() {
            if ch == '`' {
                in_code = !in_code;
                continue;
            }
            if !in_code && ch == '\\' {
                chars.next();
                continue;
            }
            if !in_code && ch == '|' {
                positions.push(offset);
            }
        }
        positions
    }

    #[shallgraph::verifies("GRD-MD-010")]
    #[test]
    fn aligns_parameter_table_when_values_contain_bars() {
        let mut r = req("GRD-MD-010", "Bars");
        r.parameters = Some(IndexMap::from([
            (
                "dir".into(),
                ParameterValue::String("reqs.yaml | reqs.yml".into()),
            ),
            ("ext".into(), ParameterValue::String(".req.yml".into())),
        ]));
        let md = generate_full_markdown(&[r]);
        let detail = detail_for(&md, "GRD-MD-010");
        assert!(detail.contains(
            "| Name | Value                 |\n| ---- | --------------------- |\n| dir  | reqs.yaml \\| reqs.yml |\n| ext  | .req.yml              |"
        ));
        assert_table_columns_aligned(detail);
    }

    #[shallgraph::verifies("GRD-MD-010")]
    #[test]
    fn aligns_prose_tables_and_keeps_alignment_markers() {
        let src = "\
See a | b in prose.\n\
\n\
| Left | Center | Right |\n\
| :--- | :----: | ----: |\n\
| a | b | c |\n\
| aa | bb | cc |\n\
\n\
```\n\
| a | bb |\n\
| --- | --- |\n\
| c | d |\n\
```\n\
\n\
  | a | bb |\n\
  | --- | --- |\n\
  | ccc | d |\n";
        let expected = "\
See a | b in prose.\n\
\n\
| Left | Center | Right |\n\
| :--- | :----: | ----: |\n\
| a    |   b    |     c |\n\
| aa   |   bb   |    cc |\n\
\n\
```\n\
| a | bb |\n\
| --- | --- |\n\
| c | d |\n\
```\n\
\n\
  | a   | bb  |\n\
  | --- | --- |\n\
  | ccc | d   |\n";
        let once = align_markdown_tables(src);
        assert_eq!(once, expected);
        assert_eq!(align_markdown_tables(&once), once);

        let mut r = req("GRD-MD-010", "Prose table");
        r.refinement = "\
| Key | Meaning |\n\
| --- | --- |\n\
| a | alpha |\n\
| bb | beta |"
            .into();
        let md = generate_full_markdown(&[r]);
        let detail = detail_for(&md, "GRD-MD-010");
        assert!(detail.contains(
            "| Key | Meaning |\n| --- | ------- |\n| a   | alpha   |\n| bb  | beta    |"
        ));
        assert_table_columns_aligned(detail);

        let code = "\
| code | note |\n\
| --- | --- |\n\
| `a|b` | stays one cell |\n";
        assert_eq!(
            align_markdown_tables(code),
            "\
| code  | note           |\n\
| ----- | -------------- |\n\
| `a|b` | stays one cell |\n"
        );

        let mut linked = req("GRD-MD-010", "Linked table");
        linked.refinement = "\
| Ref | Note |\n\
| --- | --- |\n\
| GRD-HTML-001 | see it |"
            .into();
        let md = generate_full_markdown(&[req("GRD-HTML-001", "Target"), linked]);
        let detail = detail_for(&md, "GRD-MD-010");
        assert!(detail.contains("[GRD-HTML-001](#GRD-HTML-001)"));
        assert_table_columns_aligned(detail);
    }
}
