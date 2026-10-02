//! GRD-CLI-010: CLI Markdown report.
//! GRD-MD-008: When `origin` is GitHub, file paths become blob links at HEAD.

use crate::html::discover_github_link_context;
use shallgraph_core::{
    collect_rust_source_links, discover_project_root_candidates, load_active_profile,
    load_requirements, normalize_path, ArtifactLinkRenderOptions, GithubArtifactLinkContext,
    ROOT_MARKER_HINT,
};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn run_markdown(project_dir: &Path, output_dir: &Path) -> io::Result<bool> {
    run_markdown_with_github(project_dir, output_dir, discover_github_link_context)
}

/// GRD-MD-008: Injected GitHub context for tests; production uses origin/HEAD discovery.
#[shallgraph::implements("GRD-CLI-010", "GRD-MD-008")]
pub fn run_markdown_with_github<F>(
    project_dir: &Path,
    output_dir: &Path,
    discover: F,
) -> io::Result<bool>
where
    F: Fn(&Path) -> Option<GithubArtifactLinkContext>,
{
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
            "Validation failed; fix errors before generating Markdown."
        )?;
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
    let md_path = normalize_path(&out_dir.join("index.md"));
    let known_ids: HashSet<String> = result.requirements.iter().map(|r| r.id.clone()).collect();
    // GRD-MD-007: Markdown consumes collected source-link records; this CLI command is the composition point.
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
    let markdown = profile.generate_full_markdown(
        &result.requirements,
        &source_links,
        artifact_links.as_ref(),
    );
    fs::write(&md_path, markdown)?;
    writeln!(
        io::stdout(),
        "Wrote {} ({} requirements).",
        md_path.display(),
        result.requirements.len()
    )?;
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
