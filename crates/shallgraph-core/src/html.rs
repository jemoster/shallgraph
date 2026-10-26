//! GRD-HTML-001: HTML report of the full set of information in requirement files.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

use pulldown_cmark::{html as md_html, Event, Options, Parser};
use regex::Regex;

use crate::artifact_links::{github_blob_url_for_path, ArtifactLinkRenderOptions};
use crate::parameters::{resolve_to_segments, SegmentKind};
use crate::types::{ArtifactRef, Refinement, RequirementWithSource, SourceLink, SourceLinkKind};

/// GRD-SYS-005: Placeholder character range for param spans (U+E000–E0FF); replaced after markdown.
const PARAM_PLACEHOLDER_BASE: u32 = 0xe000;

const REFINEMENT_OUTLINE_CSS: &str = r#"
    .refinement-outline { list-style: none; margin: 0.35rem 0 0; padding-left: 0; }
    .refinement-outline .refinement-outline { margin-top: 0.2rem; padding-left: 1.25rem; }
    .refinement-outline > li { margin: 0.2rem 0; }
    .claim-cite { font-weight: 600; margin-right: 0.35rem; }
    .claim-text > p:only-child { display: inline; margin: 0; }
    .claim-prose { margin: 0.25rem 0 0 1.6rem; }"#;

/// GRD-HTML-009: Render a string refinement as Markdown, or a claim list as a numbered outline.
#[shallgraph::implements("GRD-HTML-009")]
fn render_refinement_html(
    refinement: &Refinement,
    requirement_id: &str,
    by_id: &HashMap<String, &RequirementWithSource>,
    path: &[usize],
) -> String {
    match refinement {
        Refinement::Prose(text) if text.is_empty() => String::new(),
        Refinement::Prose(text) => resolve_and_render_text(text, requirement_id, by_id, true),
        Refinement::Claims(claims) if claims.is_empty() => String::new(),
        Refinement::Claims(claims) => {
            let depth = path.len();
            let items: String = claims
                .iter()
                .enumerate()
                .map(|(index, claim)| {
                    let mut claim_path = path.to_vec();
                    claim_path.push(index);
                    let marker = Refinement::outline_marker(depth, index);
                    let citation = Refinement::citation(requirement_id, &claim_path);
                    let text = resolve_and_render_text(&claim.text, requirement_id, by_id, true);
                    let nested = render_refinement_html(
                        &claim.refinement,
                        requirement_id,
                        by_id,
                        &claim_path,
                    );
                    let nested_html = match &claim.refinement {
                        Refinement::Prose(prose) if !prose.is_empty() => {
                            format!("<div class=\"claim-prose\">{nested}</div>")
                        }
                        _ => nested,
                    };
                    format!(
                        "<li id=\"{}\"><span class=\"claim-cite\">{}</span><div class=\"claim-text\">{text}</div>{nested_html}</li>",
                        escape_html(&citation),
                        escape_html(&marker),
                        text = text,
                        nested_html = nested_html,
                    )
                })
                .collect();
            format!("<ol class=\"refinement-outline\">{items}</ol>")
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

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

fn render_index_node(
    node: &IndexNode,
    requirements: &[RequirementWithSource],
    by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    let mut parts = Vec::new();
    for (segment, child) in &node.children {
        let has_reqs = !child.requirements.is_empty();
        let has_children = !child.children.is_empty();
        parts.push(format!(
            "<li><span class=\"index-category\">{}</span>",
            escape_html(segment)
        ));
        if has_reqs || has_children {
            parts.push("<ul>".to_string());
            for &idx in &child.requirements {
                let r = &requirements[idx];
                let title_html = resolve_and_render_text(&r.title, &r.id, by_id, false);
                parts.push(format!(
                    "  <li><a href=\"#{}\">{}</a> – {title_html}</li>",
                    escape_html(&r.id),
                    escape_html(&r.id)
                ));
            }
            parts.push(render_index_node(child, requirements, by_id));
            parts.push("</ul>".to_string());
        }
        parts.push("</li>".to_string());
    }
    parts.join("\n")
}

/// GRD-HTML-003: Top-level index of requirements grouped by category.
fn render_hierarchical_index(
    requirements: &[RequirementWithSource],
    by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    let root = build_index_tree(requirements);
    let mut parts = Vec::new();
    for &idx in &root.requirements {
        let r = &requirements[idx];
        let title_html = resolve_and_render_text(&r.title, &r.id, by_id, false);
        parts.push(format!(
            "    <li><a href=\"#{}\">{}</a> – {title_html}</li>",
            escape_html(&r.id),
            escape_html(&r.id)
        ));
    }
    let root_list = if parts.is_empty() {
        String::new()
    } else {
        format!("\n  <ul>\n{}\n  </ul>\n", parts.join("\n"))
    };
    let child_list = if root.children.is_empty() {
        String::new()
    } else {
        format!(
            "\n  <ul class=\"index-by-category\">\n{}\n  </ul>\n",
            render_index_node(&root, requirements, by_id)
        )
    };
    format!("{root_list}{child_list}")
}

fn format_attr_value(v: &serde_json::Value) -> String {
    let s = match v {
        serde_json::Value::Null => return String::new(),
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
    };
    escape_html(&s).replace('\n', "<br>")
}

fn parameter_value_html(value: &crate::types::ParameterValue) -> String {
    escape_html(&value.as_display_string()).replace('\n', "<br>")
}

/// GRD-HTML-004: Render markdown to HTML for refinement and rationale, matching the
/// TypeScript report (`markdown-it` with `html: false`): HTML in the source is escaped,
/// CommonMark plus tables/strikethrough, quotes escaped as `&quot;`.
fn markdown_to_html(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(text.trim(), options).map(|event| match event {
        // markdown-it `html: false` treats raw HTML as text rather than passing it through.
        Event::Html(html) | Event::InlineHtml(html) => Event::Text(html),
        other => other,
    });
    let mut out = String::new();
    md_html::push_html(&mut out, parser);
    normalize_markdown_it_html(&out).trim().to_string()
}

fn img_void_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<img([^>]*?)\s*/>").expect("img void pattern"))
}

fn table_block_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<table\b.*?</table>").expect("table block pattern"))
}

/// Align pulldown-cmark HTML with markdown-it (`xhtmlOut: false`, strikethrough `<s>`,
/// pretty-printed tables, quotes escaped in text nodes).
fn normalize_markdown_it_html(html: &str) -> String {
    let mut s = html.replace("<br />", "<br>");
    s = s.replace("<hr />", "<hr>");
    s = s.replace("<del>", "<s>");
    s = s.replace("</del>", "</s>");
    s = img_void_pattern().replace_all(&s, "<img$1>").into_owned();
    s = pretty_print_markdown_it_tables(&s);
    escape_quotes_outside_tags(&s)
}

fn pretty_print_markdown_it_tables(html: &str) -> String {
    table_block_pattern()
        .replace_all(html, |caps: &regex::Captures| {
            let mut table = caps[0].to_string();
            for (from, to) in [
                ("<table>", "<table>\n"),
                ("<thead>", "<thead>\n"),
                ("<tbody>", "<tbody>\n"),
                ("<tr>", "<tr>\n"),
                ("</tr>", "</tr>\n"),
                ("</th>", "</th>\n"),
                ("</td>", "</td>\n"),
                ("</thead>", "</thead>\n"),
                ("</tbody>", "</tbody>\n"),
            ] {
                table = table.replace(from, to);
            }
            while table.contains("\n\n") {
                table = table.replace("\n\n", "\n");
            }
            if table.ends_with('\n') {
                table.pop();
            }
            table
        })
        .into_owned()
}

fn escape_quotes_outside_tags(html: &str) -> String {
    split_keeping_tags(html)
        .into_iter()
        .map(|token| {
            if token.starts_with('<') {
                token.to_string()
            } else {
                token.replace('"', "&quot;")
            }
        })
        .collect()
}

fn id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\b").expect("id pattern"))
}

/// GRD-HTML-006: Link requirement ID references in rendered text fields.
fn auto_link_requirement_refs(
    rendered_html: &str,
    by_id: &HashMap<String, &RequirementWithSource>,
) -> String {
    let tokens: Vec<&str> = split_keeping_tags(rendered_html);
    let mut anchor_depth = 0i32;
    let re = id_pattern();
    let mut out = String::new();
    for token in tokens {
        if token.starts_with('<') {
            let lower = token.to_ascii_lowercase();
            if lower.starts_with("<a ") || lower.starts_with("<a>") {
                anchor_depth += 1;
            }
            if lower.starts_with("</a>") {
                anchor_depth = (anchor_depth - 1).max(0);
            }
            out.push_str(token);
            continue;
        }
        if anchor_depth > 0 {
            out.push_str(token);
            continue;
        }
        let replaced = re.replace_all(token, |caps: &regex::Captures| {
            let candidate = &caps[1];
            if by_id.contains_key(candidate) {
                format!(
                    "<a href=\"#{}\">{}</a>",
                    escape_html(candidate),
                    escape_html(candidate)
                )
            } else {
                caps[0].to_string()
            }
        });
        out.push_str(&replaced);
    }
    out
}

fn split_keeping_tags(html: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut start = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if start < i {
                out.push(&html[start..i]);
            }
            let tag_start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            out.push(&html[tag_start..i]);
            start = i;
        } else {
            i += 1;
        }
    }
    if start < html.len() {
        out.push(&html[start..]);
    }
    out
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
            let link = if let Some(source) = &seg.source_req_id {
                format!(
                    "<a href=\"#{}\" title=\"Parameter {} from {}\">{}</a>",
                    escape_html(source),
                    escape_html(seg.param_name.as_deref().unwrap_or("")),
                    escape_html(source),
                    escape_html(&seg.text)
                )
            } else {
                escape_html(&seg.text)
            };
            param_spans.push(format!(
                "<span class=\"param-value\" data-source-req=\"{}\" data-param=\"{}\">{link}</span>",
                escape_html(seg.source_req_id.as_deref().unwrap_or("")),
                escape_html(seg.param_name.as_deref().unwrap_or(""))
            ));
        }
    }
    let mut out = if use_markdown {
        markdown_to_html(&resolved)
    } else {
        escape_html(&resolved).replace('\n', "<br>")
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

/// GRD-HTML-002: Collect requirement ids that link to each requirement (reverse lookup).
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

fn requirement_detail_html(
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
        .map(|(k, v)| {
            format!(
                "<span class=\"attr\"><span class=\"label\">{}</span> {}</span>",
                escape_html(&capitalize_label(k)),
                format_attr_value(v)
            )
        })
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
            let link_val = if is_ref {
                format!(
                    "<a href=\"#{}\">{}</a>",
                    escape_html(val_str),
                    escape_html(val_str)
                )
            } else {
                escape_html(val_str)
            };
            other_link_parts.push(format!(
                "<span class=\"link\"><span class=\"label\">{}</span> {link_val}</span>",
                escape_html(&capitalize_label(key))
            ));
        }
    }

    let meta_html = format!(
        "<p class=\"meta\">{}</p>",
        if meta_attr_parts.is_empty() {
            "—".to_string()
        } else {
            meta_attr_parts.join(" | ")
        }
    );

    let parameters_html = match &r.parameters {
        Some(params) if !params.is_empty() => {
            let rows: String = params
                .iter()
                .map(|(name, value)| {
                    format!(
                        "<tr><td>{}</td><td>{}</td></tr>",
                        escape_html(name),
                        parameter_value_html(value)
                    )
                })
                .collect();
            format!(
                "<div class=\"labeled-block parameters-block\"><span class=\"label\">Parameters</span><table class=\"parameters-table\"><thead><tr><th>Name</th><th>Value</th></tr></thead><tbody>{rows}</tbody></table></div>"
            )
        }
        _ => String::new(),
    };

    let satisfies_html = if satisfies_ids.is_empty() {
        String::new()
    } else {
        let items: String = satisfies_ids
            .iter()
            .map(|id| {
                format!(
                    "<li><a href=\"#{}\">{}</a></li>",
                    escape_html(id),
                    escape_html(id)
                )
            })
            .collect();
        format!(
            "<div class=\"labeled-block\"><span class=\"label\">Satisfies</span><ul class=\"satisfies-list\">{items}</ul></div>"
        )
    };

    let linked_from_html = match linked_from_ids {
        Some(ids) if !ids.is_empty() => {
            let items: String = ids
                .iter()
                .map(|id| {
                    format!(
                        "<li><a href=\"#{}\">{}</a></li>",
                        escape_html(id),
                        escape_html(id)
                    )
                })
                .collect();
            format!(
                "<div class=\"labeled-block\"><span class=\"label\">Linked from</span><ul class=\"linked-from-list\">{items}</ul></div>"
            )
        }
        _ => String::new(),
    };

    let other_links_html = if other_link_parts.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"labeled-block links-block\"><span class=\"label\">Links</span><p class=\"link-inline\">{}</p></div>",
            other_link_parts.join(" | ")
        )
    };

    let rationale_html: String = below_desc_attrs
        .iter()
        .map(|(k, v)| {
            let str_val = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let html = if k.as_str() == "rationale" {
                resolve_and_render_text(&str_val, &r.id, by_id, true)
            } else {
                resolve_and_render_text(&str_val, &r.id, by_id, false)
            };
            format!(
                "<div class=\"labeled-block\"><span class=\"label\">{}</span><div class=\"rationale\">{html}</div></div>",
                escape_html(&capitalize_label(k))
            )
        })
        .collect();

    let satisfied_by_html = combined_trace_section_html(
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
    let verified_by_html = combined_trace_section_html(
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

    let links_at_bottom = [
        satisfied_by_html,
        verified_by_html,
        satisfies_html,
        linked_from_html,
        other_links_html,
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n      ");

    let title_html = resolve_and_render_text(&r.title, &r.id, by_id, false);
    let require_html = resolve_and_render_text(&r.require, &r.id, by_id, false);
    let refinement_html = render_refinement_html(&r.refinement, &r.id, by_id, &[]);

    let refinement_block = if refinement_html.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"labeled-block\"><span class=\"label\">Refinement</span><div class=\"refinement\">{refinement_html}</div></div>"
        )
    };

    format!(
        r#"
    <section id="{id}" class="requirement-detail">
      <h2>{id_esc} – {title_html}</h2>
      {meta_html}
      {parameters_html}
      <div class="labeled-block"><span class="label">Require</span><div class="require">{require_html}</div></div>
      {refinement_block}
      {rationale_html}
      {links_at_bottom}
      <p class="source"><span class="label">Source file</span> {source}</p>
    </section>"#,
        id = escape_html(&r.id),
        id_esc = escape_html(&r.id),
        source = source_file_html(&r.source_path.display().to_string(), artifact_links),
    )
}

const GITHUB_LINK_ATTRS: &str = " target=\"_blank\" rel=\"noopener noreferrer\"";

fn github_file_href(
    path: &str,
    line_start: Option<u32>,
    line_end: Option<u32>,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> Option<String> {
    let github = artifact_links?.github.as_ref()?;
    Some(github_blob_url_for_path(path, github, line_start, line_end))
}

fn github_file_anchor(href: &str, inner: &str) -> String {
    format!(
        "<a href=\"{}\"{GITHUB_LINK_ATTRS}>{inner}</a>",
        escape_html(href)
    )
}

/// GRD-HTML-008: Source YAML path is a GitHub blob link when origin is GitHub.
#[shallgraph::implements("GRD-HTML-008")]
fn source_file_html(
    source_path: &str,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let escaped = escape_html(source_path);
    match github_file_href(source_path, None, None, artifact_links) {
        Some(href) => github_file_anchor(&href, &escaped),
        None => escaped,
    }
}

fn artifact_refs_list_html(
    refs: &[ArtifactRef],
    by_id: &HashMap<String, &RequirementWithSource>,
    requirement_id: &str,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let http_attrs = if artifact_links.is_some() {
        GITHUB_LINK_ATTRS
    } else {
        ""
    };
    let mut items = Vec::new();
    for ref_item in refs {
        let artifact = ref_item.artifact.trim();
        if artifact.is_empty() {
            continue;
        }
        let lower = artifact.to_ascii_lowercase();
        let is_url = lower.starts_with("http://") || lower.starts_with("https://");
        let artifact_html = if is_url {
            format!(
                "<a href=\"{}\"{http_attrs}>{}</a>",
                escape_html(artifact),
                escape_html(artifact)
            )
        } else if let Some(href) = github_file_href(artifact, None, None, artifact_links) {
            github_file_anchor(&href, &escape_html(artifact))
        } else {
            format!("<code>{}</code>", escape_html(artifact))
        };
        if let Some(desc) = ref_item
            .description
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let desc_html = resolve_and_render_text(desc, requirement_id, by_id, false);
            items.push(format!(
                "<li>{artifact_html} <span class=\"artifact-desc\">— {desc_html}</span></li>"
            ));
        } else {
            items.push(format!("<li>{artifact_html}</li>"));
        }
    }
    items.join("")
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

fn source_link_item_html(
    link: &SourceLink,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let line_start = link.linespace.first().copied();
    let line_end = link.linespace.last().copied();
    let path_html = match github_file_href(link.path.as_str(), line_start, line_end, artifact_links)
    {
        Some(href) => {
            github_file_anchor(&href, &format!("<code>{}</code>", escape_html(&link.path)))
        }
        None => format!("<code>{}</code>", escape_html(&link.path)),
    };
    format!(
        "<li class=\"source-link\">{path_html} <span class=\"source-link-item\">{}</span> <span class=\"source-link-lines\">{}</span></li>",
        escape_html(&link.item),
        escape_html(&format_linespace(&link.linespace)),
    )
}

fn source_links_list_html<'a, I>(
    links: I,
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String
where
    I: IntoIterator<Item = &'a SourceLink>,
{
    links
        .into_iter()
        .map(|link| source_link_item_html(link, artifact_links))
        .collect()
}

fn origin_group_html(origin: &str, list_class: &str, items: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    format!(
        "<div class=\"link-origin\"><span class=\"label\">{}</span><ul class=\"{list_class}\">{items}</ul></div>",
        escape_html(origin)
    )
}

/// GRD-HTML-007: One Satisfied by / Verified by heading with origin groups (By comment, Rust).
#[shallgraph::implements("GRD-HTML-007")]
fn combined_trace_section_html<'a, I>(
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
        .map(|refs| artifact_refs_list_html(refs, by_id, requirement_id, artifact_links))
        .unwrap_or_default();
    let source_list = source_links_list_html(source_links, artifact_links);
    let groups = [
        origin_group_html("By comment", "artifact-refs-list", &yaml_list),
        origin_group_html("Rust", "source-links-list", &source_list),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("");
    if groups.is_empty() {
        return String::new();
    }
    format!(
        "<div class=\"labeled-block artifact-refs-block source-links-block\"><span class=\"label\">{}</span>{groups}</div>",
        escape_html(heading)
    )
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

/// GRD-VSC-003: Single-requirement HTML shares base structure/styling with the full report.
/// GRD-HTML-004: Refinement and rationale are rendered as Markdown (same as the full report).
pub fn generate_single_requirement_html(
    requirement: &RequirementWithSource,
    all_requirements: Option<&[RequirementWithSource]>,
) -> String {
    generate_single_requirement_html_with_source_links(requirement, all_requirements, &[], None)
}

/// GRD-VSC-003: Single-requirement HTML; GRD-HTML-007: optional source-link records.
/// GRD-UI-009: optional GitHub context turns project-relative artifact paths into blob links.
#[shallgraph::implements("GRD-VSC-003")]
pub fn generate_single_requirement_html_with_source_links(
    requirement: &RequirementWithSource,
    all_requirements: Option<&[RequirementWithSource]>,
    source_links: &[SourceLink],
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let fallback;
    let list: &[RequirementWithSource] = match all_requirements {
        Some(reqs) if !reqs.is_empty() => reqs,
        _ => {
            fallback = [requirement.clone()];
            &fallback
        }
    };
    let by_id: HashMap<String, &RequirementWithSource> =
        list.iter().map(|r| (r.id.clone(), r)).collect();
    let linked_from = linked_from_map(list);
    let req_links = source_links_for_requirement(source_links, &requirement.id);
    let detail = requirement_detail_html(
        requirement,
        linked_from.get(&requirement.id).map(|v| v.as_slice()),
        &by_id,
        &req_links,
        artifact_links,
    );

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Requirement {id}</title>
  <style>
    body {{ font-family: system-ui, sans-serif; max-width: 60rem; margin: 0 auto; padding: 1rem; }}
    .requirement-detail {{ margin: 0; padding: 0; border: none; }}
    .meta, .source {{ color: #666; font-size: 0.9rem; }}
    .label {{ font-weight: 600; color: #444; }}
    .labeled-block {{ margin-top: 0.75rem; }}
    .labeled-block .label {{ display: block; margin-bottom: 0.25rem; font-size: 0.9rem; }}
    .require, .refinement p, .rationale p {{ margin: 0.4em 0; }}
    .refinement p:first-child, .rationale p:first-child {{ margin-top: 0; }}
    .refinement p:last-child, .rationale p:last-child {{ margin-bottom: 0; }}
    {REFINEMENT_OUTLINE_CSS}
    .rationale {{ margin-top: 0; }}
    .satisfies-list, .linked-from-list, .artifact-refs-list, .source-links-list {{ margin: 0.25rem 0 0 1.25rem; padding: 0; }}
    .source-link-item, .source-link-lines {{ color: #666; }}
    .link-origin {{ margin-top: 0.35rem; margin-left: 0.25rem; }}
    .link-origin > .label {{ font-size: 0.8rem; color: #555; }}
    .param-value {{ background: #e8f4f8; padding: 0.1em 0.3em; border-radius: 3px; font-weight: 500; }}
    .parameters-table {{ margin: 0.25rem 0 0 0; border-collapse: collapse; width: 100%; max-width: 30rem; }}
    .parameters-table th, .parameters-table td {{ padding: 0.25rem 0.5rem; text-align: left; border: 1px solid #ddd; }}
    .parameters-table th {{ font-weight: 600; color: #444; background: #f8f8f8; }}
  </style>
</head>
<body>
{detail}
</body>
</html>"#,
        id = escape_html(&requirement.id),
        REFINEMENT_OUTLINE_CSS = REFINEMENT_OUTLINE_CSS,
    )
}

/// GRD-HTML-001: HTML report represents the full set of information in the requirements file.
/// GRD-SYS-010: Invoked via the active profile.
pub fn generate_full_html(requirements: &[RequirementWithSource]) -> String {
    generate_full_html_with_source_links(requirements, &[])
}

/// GRD-HTML-007: Present source-link records on each requirement (Satisfied by / Verified by).
pub fn generate_full_html_with_source_links(
    requirements: &[RequirementWithSource],
    source_links: &[SourceLink],
) -> String {
    generate_full_html_with_artifact_links(requirements, source_links, None)
}

/// GRD-HTML-007 / GRD-HTML-008: Full report with optional source-link records and artifact/IDE link context.
#[shallgraph::implements("GRD-HTML-007", "GRD-HTML-008")]
pub fn generate_full_html_with_artifact_links(
    requirements: &[RequirementWithSource],
    source_links: &[SourceLink],
    artifact_links: Option<&ArtifactLinkRenderOptions>,
) -> String {
    let by_id: HashMap<String, &RequirementWithSource> =
        requirements.iter().map(|r| (r.id.clone(), r)).collect();
    let index_html = render_hierarchical_index(requirements, &by_id);
    let linked_from = linked_from_map(requirements);
    let details: String = requirements
        .iter()
        .map(|r| {
            let req_links = source_links_for_requirement(source_links, &r.id);
            requirement_detail_html(
                r,
                linked_from.get(&r.id).map(|v| v.as_slice()),
                &by_id,
                &req_links,
                artifact_links,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Requirements report</title>
  <style>
    body {{ font-family: system-ui, sans-serif; max-width: 60rem; margin: 0 auto; padding: 1rem; }}
    h1 {{ margin-top: 0; }}
    .requirement-detail {{ margin: 2rem 0; padding-bottom: 2rem; border-bottom: 1px solid #eee; }}
    .meta, .source {{ color: #666; font-size: 0.9rem; }}
    .label {{ font-weight: 600; color: #444; }}
    .labeled-block {{ margin-top: 0.75rem; }}
    .labeled-block .label {{ display: block; margin-bottom: 0.25rem; font-size: 0.9rem; }}
    .require, .refinement p, .rationale p {{ margin: 0.4em 0; }}
    .refinement p:first-child, .rationale p:first-child {{ margin-top: 0; }}
    .refinement p:last-child, .rationale p:last-child {{ margin-bottom: 0; }}
    {REFINEMENT_OUTLINE_CSS}
    .rationale {{ margin-top: 0; }}
    .satisfies-list, .linked-from-list, .artifact-refs-list, .source-links-list {{ margin: 0.25rem 0 0 1.25rem; padding: 0; }}
    .source-link-item, .source-link-lines {{ color: #666; }}
    .link-origin {{ margin-top: 0.35rem; margin-left: 0.25rem; }}
    .link-origin > .label {{ font-size: 0.8rem; color: #555; }}
    .index-category {{ font-weight: 600; color: #333; }}
    .param-value {{ background: #e8f4f8; padding: 0.1em 0.3em; border-radius: 3px; font-weight: 500; }}
    .parameters-table {{ margin: 0.25rem 0 0 0; border-collapse: collapse; width: 100%; max-width: 30rem; }}
    .parameters-table th, .parameters-table td {{ padding: 0.25rem 0.5rem; text-align: left; border: 1px solid #ddd; }}
    .parameters-table th {{ font-weight: 600; color: #444; background: #f8f8f8; }}
  </style>
</head>
<body>
  <h1>Requirements</h1>
  <p>Total: {count}</p>
  <h2>Index</h2>
{index_html}
  <h2>Details</h2>
{details}
</body>
</html>"#,
        count = requirements.len(),
        REFINEMENT_OUTLINE_CSS = REFINEMENT_OUTLINE_CSS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_links::{ArtifactLinkRenderOptions, GithubArtifactLinkContext};
    use crate::types::{
        ArtifactRef, Claim, Link, ParameterValue, Refinement, Requirement, RequirementWithSource,
        SourceLink, SourceLinkKind,
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

    #[test]
    fn full_report_index_and_details() {
        let html = generate_full_html(&[req("GRD-A-001", "First"), req("GRD-A-002", "Second")]);
        assert!(html.contains("<h1>Requirements</h1>"));
        assert!(html.contains("Total: 2"));
        assert!(html.contains("<h2>Index</h2>"));
        assert!(html.contains("<h2>Details</h2>"));
        assert!(html.contains("href=\"#GRD-A-001\""));
        assert!(html.contains("id=\"GRD-A-001\""));
        assert!(html.contains("First"));
        assert!(html.contains("class=\"requirement-detail\""));
        assert!(html.contains("class=\"require\""));
    }

    #[test]
    fn includes_attributes_and_links() {
        let mut r = req("GRD-HTML-001", "HTML report");
        r.attributes = Some(IndexMap::from([
            ("status".into(), serde_json::json!("active")),
            (
                "rationale".into(),
                serde_json::json!("HTML output is easily consumed and distributed."),
            ),
        ]));
        let html = generate_full_html(&[r]);
        assert!(html.contains("Status"));
        assert!(html.contains("active"));
        assert!(html.contains("Rationale"));
        assert!(html.contains("HTML output is easily consumed"));
    }

    #[test]
    fn parameters_table() {
        let mut r = req("GRD-TBL-001", "Table requirement");
        r.parameters = Some(IndexMap::from([
            ("alpha".into(), ParameterValue::String("one".into())),
            ("beta".into(), ParameterValue::String("two".into())),
        ]));
        let html = generate_full_html(&[r]);
        assert!(html.contains("<table class=\"parameters-table\">"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<th>Value</th>"));
        assert!(html.contains("<tr><td>alpha</td><td>one</td></tr>"));
        assert!(html.contains("<tr><td>beta</td><td>two</td></tr>"));
    }

    #[test]
    fn no_parameters_section_when_absent() {
        let html = generate_full_html(&[req("GRD-NOP-001", "No params")]);
        assert!(!html.contains("parameters-block"));
    }

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
        let html = generate_full_html(&[r]);
        assert!(html.contains("Satisfied by"));
        assert!(html.contains("By comment"));
        assert!(html.contains("<code>packages/core/src/foo.ts</code>"));
        assert!(html.contains("Implements the feature."));
        assert!(html.contains("href=\"https://example.com/evidence\""));
        assert!(html.contains("Verified by"));
        assert!(!html.contains("Rust"));
        assert!(!html.contains("Implemented by"));
    }

    #[shallgraph::verifies("GRD-UI-009")]
    #[test]
    fn github_blob_links_and_external_artifacts_open_in_new_window() {
        let mut r = req("GRD-UI9-001", "Traceability");
        r.satisfied_by = Some(vec![
            ArtifactRef {
                artifact: "packages/core/src/foo.ts".into(),
                description: Some("Implementation.".into()),
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
        let artifact_links = ArtifactLinkRenderOptions {
            github: Some(GithubArtifactLinkContext {
                owner: "acme".into(),
                repo: "widgets".into(),
                commit_sha: "abcdef1234567890".into(),
                project_root_rel: "apps/reqs".into(),
                ..Default::default()
            }),
        };
        let html = generate_single_requirement_html_with_source_links(
            &r,
            Some(&[r.clone()]),
            &[],
            Some(&artifact_links),
        );
        assert!(html.contains("target=\"_blank\" rel=\"noopener noreferrer\""));
        assert!(html.contains(
            "href=\"https://github.com/acme/widgets/blob/abcdef1234567890/apps/reqs/packages/core/src/foo.ts\""
        ));
        assert!(html.contains("href=\"https://example.com/evidence\""));
        assert!(!html.contains("<code>packages/core/src/foo.ts</code>"));
    }

    #[shallgraph::verifies("GRD-UI-009")]
    #[test]
    fn file_paths_remain_code_when_artifact_links_has_no_github_context() {
        let mut r = req("GRD-UI9-002", "Local paths");
        r.satisfied_by = Some(vec![ArtifactRef {
            artifact: "src/local.ts".into(),
            description: None,
        }]);
        let artifact_links = ArtifactLinkRenderOptions::default();
        let html = generate_single_requirement_html_with_source_links(
            &r,
            Some(&[r.clone()]),
            &[],
            Some(&artifact_links),
        );
        assert!(html.contains("<code>src/local.ts</code>"));
        assert!(!html.contains("target=\"_blank\""));
    }

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
        let html = generate_full_html(&[a, b, c]);
        let start = html.find("id=\"GRD-A\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(detail.contains("Linked from"));
        assert!(detail.contains("GRD-B"));
        assert!(detail.contains("GRD-C"));
    }

    #[test]
    fn hierarchical_index() {
        let mut a = req("GRD-HTML-001", "HTML report");
        a.category_path = Some(vec!["html-report".into()]);
        let mut b = req("GRD-HTML-002", "Linked from");
        b.category_path = Some(vec!["html-report".into()]);
        let mut c = req("GRD-SYS-001", "Core");
        c.category_path = Some(vec!["sys".into()]);
        let html = generate_full_html(&[a, b, c]);
        let idx =
            &html[html.find("<h2>Index</h2>").unwrap()..html.find("<h2>Details</h2>").unwrap()];
        assert!(idx.contains("class=\"index-category\""));
        assert!(idx.contains("html-report"));
        assert!(idx.contains("sys"));
        assert!(idx.contains("href=\"#GRD-HTML-001\""));
    }

    #[test]
    fn markdown_refinement_and_rationale() {
        let mut r = req("GRD-MD-001", "Title");
        r.refinement = "Plain and **bold** and *italic* text.".into();
        let html = generate_full_html(&[r]);
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));

        let mut r = req("GRD-MD-002", "Title");
        r.refinement = "Desc".into();
        r.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("Reason with `code` and **emphasis**."),
        )]));
        let html = generate_full_html(&[r]);
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<strong>emphasis</strong>"));

        let mut r = req("GRD-MD-003", "Title");
        r.refinement = "Text with <script>alert(1)</script> here".into();
        r.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("Rationale with <b>tag</b>."),
        )]));
        let html = generate_full_html(&[r]);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<b>tag</b>"));
        assert!(html.contains("&lt;b&gt;tag&lt;/b&gt;"));
    }

    #[shallgraph::verifies("GRD-HTML-009")]
    #[test]
    fn html_claim_outline_keeps_string_refinements_unnumbered() {
        let mut prose = req("GRD-HTML-PROSE", "Prose");
        prose.refinement = "* still text".into();
        let html = generate_full_html(&[prose]);
        assert!(html.contains("* still text") || html.contains("still text"));
        assert!(!html.contains("class=\"claim-cite\""));

        let mut listed = req("GRD-HTML-LIST", "List");
        listed.refinement = Refinement::Claims(vec![
            Claim {
                text: "with a list view".into(),
                refinement: Refinement::default(),
            },
            Claim {
                text: "in the working directory".into(),
                refinement: Refinement::Claims(vec![Claim {
                    text: "The directory shall be created.".into(),
                    refinement: Refinement::default(),
                }]),
            },
        ]);
        let html = generate_full_html(&[listed]);
        assert!(html.contains("class=\"claim-cite\">(a)</span>"));
        assert!(html.contains("class=\"claim-cite\">(b)</span>"));
        assert!(html.contains("class=\"claim-cite\">(1)</span>"));
        assert!(html.contains("id=\"GRD-HTML-LIST(b)(1)\""));
        assert!(html.contains("with a list view"));
        assert!(html.contains("The directory shall be created."));
    }

    #[test]
    fn markdown_matches_typescript_markdown_it() {
        let cases = [
            (
                "a < b",
                "<p>a &lt; b</p>",
            ),
            (
                "foo <script>alert(1)</script> bar",
                "<p>foo &lt;script&gt;alert(1)&lt;/script&gt; bar</p>",
            ),
            (
                "C++ <algorithm> header",
                "<p>C++ &lt;algorithm&gt; header</p>",
            ),
            (
                "`code with <tags>`",
                "<p><code>code with &lt;tags&gt;</code></p>",
            ),
            (
                "the \"Editor\" view",
                "<p>the &quot;Editor&quot; view</p>",
            ),
            (
                "foo & bar",
                "<p>foo &amp; bar</p>",
            ),
            (
                "already-escaped &amp; bar",
                "<p>already-escaped &amp; bar</p>",
            ),
            (
                "See <http://example.com>",
                "<p>See <a href=\"http://example.com\">http://example.com</a></p>",
            ),
            (
                "~~old~~",
                "<p><s>old</s></p>",
            ),
            (
                "| Name | Value |\n| --- | --- |\n| x | 1 |",
                "<table>\n<thead>\n<tr>\n<th>Name</th>\n<th>Value</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>x</td>\n<td>1</td>\n</tr>\n</tbody>\n</table>",
            ),
        ];
        for (src, expected) in cases {
            assert_eq!(markdown_to_html(src), expected, "markdown source: {src:?}");
        }
    }

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
        let html = generate_full_html(&[r]);
        let os = html.find("<tr><td>native_binary_os</td>").expect("os row");
        let arch = html
            .find("<tr><td>native_binary_arch</td>")
            .expect("arch row");
        assert!(os < arch, "parameter rows should keep insertion order");
    }

    #[test]
    fn auto_link_known_ids() {
        let mut holder = req("GRD-REF-001", "Reference holder");
        holder.refinement = "See GRD-HTML-001 for base behavior.".into();
        holder.attributes = Some(IndexMap::from([(
            "rationale".into(),
            serde_json::json!("Also depends on GRD-HTML-002."),
        )]));
        let html = generate_full_html(&[
            req("GRD-HTML-001", "Target 1"),
            req("GRD-HTML-002", "Target 2"),
            holder,
        ]);
        let start = html.find("id=\"GRD-REF-001\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(detail.contains("href=\"#GRD-HTML-001\""));
        assert!(detail.contains("href=\"#GRD-HTML-002\""));
    }

    #[test]
    fn param_values_in_html() {
        let mut r = req("GRD-P-001", "Limit is {{ :limit }}");
        r.refinement = "The maximum count is {{ :limit }} items.".into();
        r.parameters = Some(IndexMap::from([(
            "limit".into(),
            ParameterValue::Integer(42),
        )]));
        let html = generate_full_html(&[r]);
        assert!(html.contains("class=\"param-value\""));
        assert!(html.contains("data-source-req=\"GRD-P-001\""));
        assert!(html.contains("data-param=\"limit\""));
        assert!(html.contains("42"));
    }

    #[shallgraph::verifies("GRD-VSC-003")]
    #[test]
    fn single_requirement_html_renders_one_requirement() {
        let r = req("GRD-VSC-003", "Requirement editor preview");
        let html = generate_single_requirement_html(&r, None);
        assert!(html.contains("id=\"GRD-VSC-003\""));
        assert!(html.contains("Requirement editor preview"));
        assert!(html.contains("class=\"requirement-detail\""));
        assert!(html.contains("class=\"meta\""));
        assert!(html.contains("class=\"source\""));
        assert!(html.contains("class=\"require\""));
        assert!(html.contains("<title>Requirement GRD-VSC-003</title>"));
        assert!(!html.contains("<h1>Requirements</h1>"));
        assert!(!html.contains("<h2>Index</h2>"));
        assert!(!html.contains("data-shallgraph-field"));
    }

    #[shallgraph::verifies("GRD-VSC-003")]
    #[test]
    fn single_requirement_html_shares_structure_and_styling_with_full_report() {
        let mut r = req("GRD-X-001", "Title");
        r.refinement = "Body".into();
        r.attributes = Some(IndexMap::from([(
            "status".into(),
            serde_json::json!("active"),
        )]));
        r.links = Some(vec![Link {
            satisfies: Some("GRD-X-000".into()),
            extra: BTreeMap::new(),
        }]);
        let single = generate_single_requirement_html(&r, None);
        let full = generate_full_html(&[r.clone()]);
        assert!(single.contains("<html lang=\"en\">"));
        assert!(single.contains("requirement-detail"));
        assert!(single.contains("meta"));
        assert!(single.contains("source"));
        assert!(single.contains("refinement"));
        assert!(single.contains("font-family: system-ui, sans-serif"));
        assert!(single.contains("max-width: 60rem"));
        assert!(full.contains("font-family: system-ui, sans-serif"));
        assert!(full.contains("max-width: 60rem"));
        assert!(single.contains("<title>Requirement GRD-X-001</title>"));
        let start = single.find("id=\"GRD-X-001\"").unwrap();
        let end = single[start..].find("</section>").unwrap() + start;
        let detail = &single[start..end];
        assert!(full.contains(detail));
    }

    #[shallgraph::verifies("GRD-VSC-003")]
    #[test]
    fn single_requirement_html_escapes_fields() {
        let mut r = req("ID", "Title <script>");
        r.refinement = "Desc & \"quoted\"".into();
        r.source_path = PathBuf::from("/p/ID.req.yml");
        let html = generate_single_requirement_html(&r, None);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&quot;"));
    }

    #[shallgraph::verifies("GRD-VSC-003")]
    #[test]
    fn single_requirement_html_resolves_params_from_all_requirements() {
        let mut r_a = req("GRD-A", "A");
        r_a.parameters = Some(IndexMap::from([(
            "x".into(),
            ParameterValue::String("10".into()),
        )]));
        let mut r_b = req("GRD-B", "B");
        r_b.refinement = "Value: {{ GRD-A:x }}".into();
        let html = generate_single_requirement_html(&r_b, Some(&[r_a.clone(), r_b.clone()]));
        assert!(html.contains("10"));
        assert!(html.contains("class=\"param-value\""));
        assert!(html.contains("data-source-req=\"GRD-A\""));
        assert!(html.contains("data-param=\"x\""));
    }

    #[shallgraph::verifies("GRD-VSC-003")]
    #[test]
    fn single_requirement_html_includes_reverse_links_from_all_requirements() {
        let a = req("GRD-A", "Target");
        let mut b = req("GRD-B", "Linker");
        b.links = Some(vec![Link {
            satisfies: Some("GRD-A".into()),
            extra: BTreeMap::new(),
        }]);
        let html = generate_single_requirement_html(&a, Some(&[a.clone(), b]));
        assert!(html.contains("Linked from"));
        assert!(html.contains("GRD-B"));
    }

    #[shallgraph::verifies("GRD-VSC-003", "GRD-HTML-007")]
    #[test]
    fn single_requirement_html_presents_source_links() {
        let r = req("GRD-HTML-007", "Source links");
        let implements = SourceLink::new(
            "GRD-HTML-007",
            SourceLinkKind::Implements,
            "src/html.rs",
            "function",
            vec![10, 11, 12],
        )
        .unwrap();
        let html =
            generate_single_requirement_html_with_source_links(&r, None, &[implements], None);
        assert!(html.contains("Satisfied by"));
        assert!(html.contains("Rust"));
        assert!(!html.contains("By comment"));
        assert!(html.contains("<code>src/html.rs</code>"));
        assert!(html.contains("L10–L12"));
        assert!(!html.contains("Verified by"));
        assert!(!html.contains("Implemented by"));
    }

    #[shallgraph::verifies("GRD-HTML-007")]
    #[test]
    fn presents_source_links_by_kind() {
        let r = req("GRD-HTML-007", "Source links");
        let implements = SourceLink::new(
            "GRD-HTML-007",
            SourceLinkKind::Implements,
            "src/html.rs",
            "function",
            vec![10, 11, 12],
        )
        .unwrap();
        let verifies = SourceLink::new(
            "GRD-HTML-007",
            SourceLinkKind::Verifies,
            "src/html.rs",
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
        let html = generate_full_html_with_source_links(&[r], &[implements, verifies, other]);
        let start = html.find("id=\"GRD-HTML-007\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(detail.contains("Satisfied by"));
        assert!(detail.contains("Rust"));
        assert!(detail.contains("<code>src/html.rs</code>"));
        assert!(detail.contains("function"));
        assert!(detail.contains("L10–L12"));
        assert!(detail.contains("Verified by"));
        assert!(detail.contains("test"));
        assert!(detail.contains("L80"));
        assert!(!detail.contains("src/other.rs"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("By comment"));
    }

    #[shallgraph::verifies("GRD-HTML-007")]
    #[test]
    fn omits_source_link_sections_without_records() {
        let html = generate_full_html(&[req("GRD-NONE-001", "No links")]);
        let start = html.find("id=\"GRD-NONE-001\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(!detail.contains("Satisfied by"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("Verified by"));
        assert!(!detail.contains("By comment"));
        assert!(!detail.contains("Rust"));
    }

    #[shallgraph::verifies("GRD-HTML-007")]
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
        let html = generate_full_html_with_source_links(&[r], &[verifies]);
        let start = html.find("id=\"GRD-MIX-001\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        let verified_count = detail.matches("Verified by").count();
        assert_eq!(verified_count, 1);
        assert!(detail.contains("By comment"));
        assert!(detail.contains("Rust"));
        assert!(detail.contains("<code>test/foo.test.ts</code>"));
        assert!(detail.contains("<code>src/lib.rs</code>"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("Satisfied by"));
        let comment_pos = detail.find("By comment").unwrap();
        let rust_pos = detail.find("Rust").unwrap();
        assert!(comment_pos < rust_pos);
    }

    #[shallgraph::verifies("GRD-HTML-007")]
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
        let html = generate_full_html_with_source_links(&[r], &[implements]);
        let start = html.find("id=\"GRD-MIX-002\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert_eq!(detail.matches("Satisfied by").count(), 1);
        assert!(detail.contains("By comment"));
        assert!(detail.contains("Rust"));
        assert!(detail.contains("<code>src/feature.ts</code>"));
        assert!(detail.contains("Primary implementation."));
        assert!(detail.contains("<code>src/lib.rs</code>"));
        assert!(detail.contains("L8–L10"));
        assert!(!detail.contains("Implemented by"));
        assert!(!detail.contains("Verified by"));
    }

    #[shallgraph::verifies("GRD-HTML-007")]
    #[test]
    fn format_linespace_compresses_ranges() {
        assert_eq!(format_linespace(&[3]), "L3");
        assert_eq!(format_linespace(&[3, 4, 5]), "L3–L5");
        assert_eq!(format_linespace(&[1, 3, 4, 8]), "L1, L3–L4, L8");
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

    #[shallgraph::verifies("GRD-HTML-008")]
    #[test]
    fn github_links_wrap_source_file_yaml_artifacts_and_source_links() {
        let mut r = req("GRD-HTML-008", "GitHub links");
        r.source_path = PathBuf::from("/workspace/requirements/html/GRD-HTML-008.req.yml");
        r.satisfied_by = Some(vec![
            ArtifactRef {
                artifact: "crates/shallgraph-core/src/html.rs".into(),
                description: Some("Renders the report.".into()),
            },
            ArtifactRef {
                artifact: "https://example.com/evidence".into(),
                description: None,
            },
        ]);
        let implements = SourceLink::new(
            "GRD-HTML-008",
            SourceLinkKind::Implements,
            "crates/shallgraph-core/src/html.rs",
            "function",
            vec![10, 11, 12],
        )
        .unwrap();
        let html =
            generate_full_html_with_artifact_links(&[r], &[implements], Some(&github_links()));
        let start = html.find("id=\"GRD-HTML-008\"").unwrap();
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(detail.contains(
            "href=\"https://github.com/acme/widgets/blob/deadbeef/requirements/html/GRD-HTML-008.req.yml\""
        ));
        assert!(detail.contains(
            "href=\"https://github.com/acme/widgets/blob/deadbeef/crates/shallgraph-core/src/html.rs\""
        ));
        assert!(detail.contains(
            "href=\"https://github.com/acme/widgets/blob/deadbeef/crates/shallgraph-core/src/html.rs#L10-L12\""
        ));
        assert!(detail.contains("<code>crates/shallgraph-core/src/html.rs</code>"));
        assert!(detail.contains("href=\"https://example.com/evidence\""));
        assert!(!detail.contains("cursor://"));
        assert!(!detail.contains("vscode://"));
    }

    #[shallgraph::verifies("GRD-HTML-008")]
    #[test]
    fn file_paths_stay_plain_without_github_context() {
        let mut r = req("GRD-HTML-008", "No GitHub");
        r.source_path = PathBuf::from("/workspace/requirements/html/GRD-HTML-008.req.yml");
        r.satisfied_by = Some(vec![ArtifactRef {
            artifact: "src/foo.ts".into(),
            description: None,
        }]);
        let html = generate_full_html(&[r]);
        assert!(!html.contains("github.com"));
        assert!(html.contains("<code>src/foo.ts</code>"));
        assert!(html.contains("/workspace/requirements/html/GRD-HTML-008.req.yml"));
        assert!(!html.contains("href=\"/workspace/requirements/html/GRD-HTML-008.req.yml\""));
    }
}
