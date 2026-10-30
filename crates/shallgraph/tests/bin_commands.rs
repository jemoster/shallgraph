//! End-to-end binary tests for the essential CLI (GRD-CLI-008).

extern crate shallgraph_macros as shallgraph;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_shallgraph"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn help_lists_essential_commands() {
    let out = Command::new(bin()).arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("validate"));
    assert!(text.contains("html"));
    assert!(text.contains("schema"));
    assert!(text.contains("bootstrap"));
}

#[test]
fn help_lists_format_command() {
    let out = Command::new(bin()).arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("format"));
}

#[test]
fn help_lists_markdown_command() {
    let out = Command::new(bin()).arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("markdown"));
}

#[test]
fn validate_sample_project_basic() {
    let root = repo_root();
    let out = Command::new(bin())
        .args([
            "validate",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "validate failed: stdout={stdout} stderr={stderr}"
    );
    assert!(stdout.contains("Validated"));
}

#[test]
fn schema_json_stdout_from_sample() {
    let root = repo_root();
    let out = Command::new(bin())
        .args([
            "schema",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["type"], "object");
    assert!(parsed["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "tags"));
    let tags = &parsed["properties"]["tags"];
    assert_eq!(tags["required"], serde_json::json!(["priority"]));
    assert_eq!(
        tags["properties"]["priority"]["enum"],
        serde_json::json!(["low", "medium", "high"])
    );
    assert_eq!(
        tags["properties"]["component"]["enum"],
        serde_json::json!(["frontend", "backend", "platform", null])
    );
    let concern = &tags["properties"]["concern"];
    assert_eq!(concern["type"], "array");
    assert_eq!(concern["maxItems"], 3);
    assert!(concern.get("minItems").is_none());
    assert_eq!(
        concern["items"]["enum"],
        serde_json::json!(["privacy", "latency", "scale", "security", null])
    );
}

#[test]
fn html_writes_index() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(tmp.join("index.html")).unwrap();
    assert!(html.contains("<h1>Requirements</h1>"));
    assert!(html.contains("SYS-001") || html.contains("FR-"));
}

#[test]
fn html_dot_output_prints_normalized_path() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-dot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .current_dir(&tmp)
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            ".",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "html failed: stdout={stdout} stderr={stderr}"
    );
    let expected = tmp.join("index.html");
    assert!(
        stdout.contains(&format!("Wrote {}", expected.display())),
        "expected normalized path in stdout, got {stdout}"
    );
    assert!(
        !stdout.contains("/./"),
        "output path should not contain /./, got {stdout}"
    );
}

#[shallgraph::verifies("GRD-CLI-010")]
#[test]
fn markdown_writes_index() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-md-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "markdown",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let md = fs::read_to_string(tmp.join("index.md")).unwrap();
    assert!(md.contains("# Requirements"));
    assert!(md.contains("SYS-001") || md.contains("FR-"));
}

#[test]
fn markdown_dot_output_prints_normalized_path() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-md-dot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .current_dir(&tmp)
        .args([
            "markdown",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            ".",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "markdown failed: stdout={stdout} stderr={stderr}"
    );
    let expected = tmp.join("index.md");
    assert!(
        stdout.contains(&format!("Wrote {}", expected.display())),
        "expected normalized path in stdout, got {stdout}"
    );
    assert!(
        !stdout.contains("/./"),
        "output path should not contain /./, got {stdout}"
    );
}

#[test]
fn validate_sample_project_rust() {
    let root = repo_root();
    let out = Command::new(bin())
        .args([
            "validate",
            "--project-dir",
            root.join("sample_projects/rust").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "validate rust sample failed: stdout={stdout} stderr={stderr}"
    );
    assert!(stdout.contains("Validated"));
}

#[test]
fn html_sample_project_rust_includes_source_links() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-rust-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/rust").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(tmp.join("index.html")).unwrap();
    for id in ["TEMP-001", "TEMP-002"] {
        let start = html
            .find(&format!("id=\"{id}\""))
            .unwrap_or_else(|| panic!("missing detail section for {id}"));
        let end = html[start..].find("</section>").unwrap() + start;
        let detail = &html[start..end];
        assert!(detail.contains("Satisfied by"), "{id} missing Satisfied by");
        assert!(detail.contains("By comment"), "{id} missing By comment");
        assert!(detail.contains("Rust"), "{id} missing Rust origin");
        assert!(detail.contains("Verified by"), "{id} missing Verified by");
        assert!(
            detail.contains("<code>src/lib.rs</code>"),
            "{id} missing src/lib.rs"
        );
        assert!(
            detail.contains("source-link-item\">function"),
            "{id} missing function source link"
        );
        assert!(
            detail.contains("source-link-item\">test"),
            "{id} missing test source link"
        );
        assert!(
            !detail.contains("Implemented by"),
            "{id} still has Implemented by"
        );
    }
}

#[test]
fn markdown_sample_project_rust_includes_source_links() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-md-rust-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "markdown",
            "--project-dir",
            root.join("sample_projects/rust").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let md = fs::read_to_string(tmp.join("index.md")).unwrap();
    for id in ["TEMP-001", "TEMP-002"] {
        let start = md
            .find(&format!("<a id=\"{id}\"></a>"))
            .unwrap_or_else(|| panic!("missing detail section for {id}"));
        let end = md[start..]
            .find("\n<a id=\"")
            .map(|i| start + i)
            .unwrap_or(md.len());
        let detail = &md[start..end];
        assert!(
            detail.contains("**Satisfied by**"),
            "{id} missing Satisfied by"
        );
        assert!(detail.contains("*By comment*"), "{id} missing By comment");
        assert!(detail.contains("*Rust*"), "{id} missing Rust origin");
        assert!(
            detail.contains("**Verified by**"),
            "{id} missing Verified by"
        );
        assert!(detail.contains("`src/lib.rs`"), "{id} missing src/lib.rs");
        assert!(
            detail.contains("function"),
            "{id} missing function source link"
        );
        assert!(detail.contains("test"), "{id} missing test source link");
        assert!(
            !detail.contains("Implemented by"),
            "{id} still has Implemented by"
        );
    }
}

#[test]
#[shallgraph::verifies("GRD-CLI-011")]
fn version_flags_print_semantic_version() {
    let version = env!("CARGO_PKG_VERSION");
    let expected = format!("{version}\n");
    let core = version.split_once('-').map(|(c, _)| c).unwrap_or(version);
    let core = core.split_once('+').map(|(c, _)| c).unwrap_or(core);
    let mut parts = core.split('.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    let patch = parts.next().unwrap_or("");
    assert!(
        parts.next().is_none()
            && !major.is_empty()
            && major.chars().all(|ch| ch.is_ascii_digit())
            && !minor.is_empty()
            && minor.chars().all(|ch| ch.is_ascii_digit())
            && !patch.is_empty()
            && patch.chars().all(|ch| ch.is_ascii_digit()),
        "package version is not a semantic version: {version}"
    );

    for args in [
        vec!["-v"],
        vec!["--version"],
        vec!["-v", "validate"],
        vec!["validate", "-v"],
        vec!["--version", "html"],
        vec!["html", "--version"],
    ] {
        let out = Command::new(bin()).args(&args).output().unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "{args:?} failed: stdout={stdout} stderr={stderr}"
        );
        assert_eq!(stdout, expected, "{args:?}");
        assert!(stderr.is_empty(), "{args:?} stderr={stderr}");
    }
}

#[test]
#[shallgraph::verifies("GRD-CLI-011")]
fn help_lists_version_flags() {
    let out = Command::new(bin()).arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("-v, --version"));
    assert!(text.contains("Print the semantic version"));
}

#[shallgraph::verifies("GRD-CLI-012")]
#[test]
fn html_help_lists_combined_pages_and_base_url() {
    let out = Command::new(bin())
        .args(["html", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("--combined"));
    assert!(text.contains("--pages"));
    assert!(text.contains("--base-url"));
    assert!(text.contains("--output"));
    assert!(!text.contains("--single-page"));
}

#[shallgraph::verifies("GRD-CLI-012")]
#[test]
fn html_combined_flag_writes_the_single_report() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!(
        "shallgraph-bin-html-combined-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--combined",
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    let index = fs::read_to_string(tmp.join("index.html")).unwrap();
    assert!(index.contains("<h2>Details</h2>"));
    assert!(index.contains("href=\"#SYS-001\""));
    assert!(!tmp.join("SYS-001.html").exists());
}

#[shallgraph::verifies("GRD-CLI-012")]
#[test]
fn html_combined_and_pages_together_writes_nothing() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-both-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--combined",
            "--pages",
            "all",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!tmp.join("index.html").exists());
}

#[shallgraph::verifies("GRD-CLI-012", "GRD-HTML-010")]
#[test]
fn html_pages_all_writes_index_and_requirement_pages() {
    let root = repo_root();
    let tmp =
        std::env::temp_dir().join(format!("shallgraph-bin-html-pages-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--pages",
            "all",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "html pages failed: stdout={stdout} stderr={stderr}"
    );
    let index = fs::read_to_string(tmp.join("index.html")).unwrap();
    assert!(index.contains("<h2>Index</h2>"));
    assert!(index.contains("href=\"SYS-001.html\""));
    assert!(!index.contains("class=\"requirement-detail\""));
    let page = fs::read_to_string(tmp.join("SYS-003.html")).unwrap();
    assert!(page.contains("href=\"index.html\""));
    assert!(page.contains("href=\"SYS-001.html\""));
    assert!(page.contains("href=\"#SYS-003(a)\""));
    assert!(page.contains("class=\"requirement-detail\""));
    let html_files = fs::read_dir(&tmp)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .count();
    let total: usize = index
        .split("Total: ")
        .nth(1)
        .and_then(|rest| rest.split('<').next())
        .and_then(|value| value.trim().parse().ok())
        .unwrap();
    assert_eq!(html_files, total + 1);
}

#[shallgraph::verifies("GRD-CLI-012", "GRD-HTML-010")]
#[test]
fn html_pages_one_id_writes_only_that_file() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-one-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--pages",
            "SYS-001",
            "--base-url",
            "https://mysite.com/requirements/",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!tmp.join("index.html").exists());
    assert!(!tmp.join("SYS-002.html").exists());
    let page = fs::read_to_string(tmp.join("SYS-001.html")).unwrap();
    assert!(page.contains("href=\"https://mysite.com/requirements/SYS-002.html\""));
    assert!(page.contains("href=\"https://mysite.com/requirements/index.html\""));
}

#[shallgraph::verifies("GRD-CLI-012")]
#[test]
fn html_base_url_without_pages_writes_nothing() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!("shallgraph-bin-html-base-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--base-url",
            "https://mysite.com/requirements",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!tmp.join("index.html").exists());
    let html_files = fs::read_dir(&tmp)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .count();
    assert_eq!(html_files, 0);
}

#[shallgraph::verifies("GRD-CLI-012")]
#[test]
fn html_unknown_pages_id_writes_nothing() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!(
        "shallgraph-bin-html-missing-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let out = Command::new(bin())
        .args([
            "html",
            "--project-dir",
            root.join("sample_projects/basic").to_str().unwrap(),
            "--output",
            tmp.to_str().unwrap(),
            "--pages",
            "ALL",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ALL"));
    assert_eq!(
        fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .count(),
        0
    );
}

#[test]
fn validate_this_repository() {
    let root = repo_root();
    let out = Command::new(bin())
        .args(["validate", "--project-dir", root.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "validate repo failed: stdout={stdout} stderr={stderr}"
    );
}
