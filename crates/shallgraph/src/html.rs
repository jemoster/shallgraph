//! GRD-CLI-002: CLI HTML report.
//! GRD-HTML-008: When `origin` is GitHub, file paths become blob links at HEAD.

use shallgraph_core::{
    collect_rust_source_links, discover_project_root_candidates, github_project_root_rel,
    load_active_profile, load_requirements, normalize_path, parse_github_origin_url,
    ArtifactLinkRenderOptions, GithubArtifactLinkContext, ROOT_MARKER_HINT,
};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

enum PageSelection {
    Combined,
    All,
    One(String),
}

fn page_file_names_ok<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    for id in ids {
        let name = format!("{id}.html");
        let path = Path::new(&name);
        let mut components = path.components();
        let single_file = matches!(
            (components.next(), components.next()),
            (Some(std::path::Component::Normal(part)), None) if part.to_str() == Some(name.as_str())
        );
        if !single_file {
            return Err(format!(
                "Requirement id cannot be used as an HTML file name: {id}. No HTML file was written."
            ));
        }
    }
    Ok(())
}

/// GRD-CLI-012: Optional per-requirement pages and the publish prefix for links between them.
pub struct HtmlReportOptions {
    pub pages: Option<String>,
    pub base_url: Option<String>,
}

impl Default for HtmlReportOptions {
    fn default() -> Self {
        Self {
            pages: None,
            base_url: None,
        }
    }
}

pub fn run_html(project_dir: &Path, output_dir: &Path) -> io::Result<bool> {
    run_html_with_options(
        project_dir,
        output_dir,
        &HtmlReportOptions::default(),
        discover_github_link_context,
    )
}

/// GRD-HTML-008: Injected GitHub context for tests; production uses origin/HEAD discovery.
pub fn run_html_with_github<F>(
    project_dir: &Path,
    output_dir: &Path,
    discover: F,
) -> io::Result<bool>
where
    F: Fn(&Path) -> Option<GithubArtifactLinkContext>,
{
    run_html_with_options(
        project_dir,
        output_dir,
        &HtmlReportOptions::default(),
        discover,
    )
}

/// GRD-CLI-012: Write the combined report, every requirement page, or one requirement page.
/// GRD-HTML-008: File paths still use the discovered GitHub blob context.
#[shallgraph::implements("GRD-CLI-012", "GRD-HTML-008")]
pub fn run_html_with_options<F>(
    project_dir: &Path,
    output_dir: &Path,
    options: &HtmlReportOptions,
    discover: F,
) -> io::Result<bool>
where
    F: Fn(&Path) -> Option<GithubArtifactLinkContext>,
{
    if options.base_url.is_some() && options.pages.is_none() {
        writeln!(
            io::stderr(),
            "--base-url requires --pages. No report was written."
        )?;
        return Ok(false);
    }

    let candidates = match discover_project_root_candidates(project_dir) {
        Ok(c) => c,
        Err(err) => {
            writeln!(io::stderr(), "{err}")?;
            return Ok(false);
        }
    };
    if candidates.is_empty() {
        writeln!(
            io::stderr(),
            "No project root found (missing {ROOT_MARKER_HINT}). Run from a directory that contains {ROOT_MARKER_HINT} or use --project-dir."
        )?;
        return Ok(false);
    }

    let root = &candidates[0];
    let profile = match load_active_profile(root) {
        Ok(p) => p,
        Err(err) => {
            writeln!(io::stderr(), "{err}")?;
            return Ok(false);
        }
    };
    let result = match load_requirements(project_dir, Some(root)) {
        Ok(r) => r,
        Err(err) => {
            writeln!(io::stderr(), "{err}")?;
            return Ok(false);
        }
    };

    if !result.errors.is_empty() {
        for err in &result.errors {
            writeln!(io::stderr(), "{}: {}", err.path, err.message)?;
        }
        writeln!(
            io::stderr(),
            "Validation failed; fix errors before generating HTML."
        )?;
        return Ok(false);
    }

    let known_ids: HashSet<String> = result.requirements.iter().map(|r| r.id.clone()).collect();
    let page_selection = match options.pages.as_deref() {
        None => PageSelection::Combined,
        Some("all") => PageSelection::All,
        Some(id) => {
            if !known_ids.contains(id) {
                writeln!(
                    io::stderr(),
                    "Unknown requirement id: {id}. No HTML file was written."
                )?;
                return Ok(false);
            }
            PageSelection::One(id.to_string())
        }
    };
    let file_ids: Vec<&str> = match &page_selection {
        PageSelection::Combined => Vec::new(),
        PageSelection::All => result
            .requirements
            .iter()
            .map(|requirement| requirement.id.as_str())
            .collect(),
        PageSelection::One(id) => vec![id.as_str()],
    };
    if let Err(message) = page_file_names_ok(file_ids) {
        writeln!(io::stderr(), "{message}")?;
        return Ok(false);
    }

    let cwd = std::env::current_dir()?;
    let out_dir = if output_dir.is_absolute() {
        output_dir.to_path_buf()
    } else {
        cwd.join(output_dir)
    };
    let out_dir = normalize_path(&out_dir);
    fs::create_dir_all(&out_dir)?;
    let html_path = normalize_path(&out_dir.join("index.html"));
    // GRD-HTML-007: HTML consumes collected source-link records; this CLI command is the composition point.
    let source_links = match collect_rust_source_links(root, &known_ids) {
        Ok(links) => links,
        Err(err) => {
            writeln!(io::stderr(), "{err}")?;
            Vec::new()
        }
    };
    let github = discover(root);
    let artifact_links = github.as_ref().map(|ctx| ArtifactLinkRenderOptions {
        github: Some(ctx.clone()),
    });
    let base_url = options.base_url.as_deref();
    match &page_selection {
        PageSelection::Combined => {
            let html = profile.generate_full_html(
                &result.requirements,
                &source_links,
                artifact_links.as_ref(),
            );
            fs::write(&html_path, html)?;
            writeln!(
                io::stdout(),
                "Wrote {} ({} requirements).",
                html_path.display(),
                result.requirements.len()
            )?;
        }
        PageSelection::All => {
            let index =
                profile.generate_requirement_index_page_html(&result.requirements, base_url);
            fs::write(&html_path, index)?;
            for requirement in &result.requirements {
                let page = profile.generate_requirement_page_html(
                    requirement,
                    &result.requirements,
                    &source_links,
                    artifact_links.as_ref(),
                    base_url,
                );
                let page_path = normalize_path(&out_dir.join(format!("{}.html", requirement.id)));
                fs::write(&page_path, page)?;
            }
            writeln!(
                io::stdout(),
                "Wrote {} (index and {} requirement pages).",
                out_dir.display(),
                result.requirements.len()
            )?;
        }
        PageSelection::One(id) => {
            let Some(requirement) = result
                .requirements
                .iter()
                .find(|requirement| requirement.id == *id)
            else {
                writeln!(
                    io::stderr(),
                    "Unknown requirement id: {id}. No HTML file was written."
                )?;
                return Ok(false);
            };
            let page = profile.generate_requirement_page_html(
                requirement,
                &result.requirements,
                &source_links,
                artifact_links.as_ref(),
                base_url,
            );
            let page_path = normalize_path(&out_dir.join(format!("{id}.html")));
            fs::write(&page_path, page)?;
            writeln!(io::stdout(), "Wrote {}.", page_path.display())?;
        }
    }
    if let Some(ctx) = github.as_ref() {
        writeln!(
            io::stdout(),
            "GitHub file links: https://{}/{}/{}/blob/{}/…",
            ctx.github_host(),
            ctx.owner,
            ctx.repo,
            ctx.commit_sha
        )?;
    }
    Ok(true)
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// GRD-HTML-008: Resolve GitHub blob context from `origin` and `HEAD`.
#[shallgraph::implements("GRD-HTML-008")]
pub fn discover_github_link_context(project_root: &Path) -> Option<GithubArtifactLinkContext> {
    let toplevel = git_stdout(project_root, &["rev-parse", "--show-toplevel"])?;
    let origin = git_stdout(project_root, &["remote", "get-url", "origin"])?;
    let head = git_stdout(project_root, &["rev-parse", "HEAD"])?;
    let parsed = parse_github_origin_url(&origin)?;
    let project_abs = project_root
        .canonicalize()
        .unwrap_or_else(|_| {
            if project_root.is_absolute() {
                project_root.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(project_root))
                    .unwrap_or_else(|_| project_root.to_path_buf())
            }
        })
        .to_string_lossy()
        .replace('\\', "/");
    let top_abs = PathBuf::from(&toplevel)
        .canonicalize()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| toplevel.replace('\\', "/"));
    Some(GithubArtifactLinkContext {
        host: parsed.host,
        owner: parsed.owner,
        repo: parsed.repo,
        commit_sha: head,
        project_root_rel: github_project_root_rel(&top_abs, &project_abs),
        project_root: project_abs,
    })
}
