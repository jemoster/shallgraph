//! shallgraph – requirement discovery and validation (GRD-CLI-008).

extern crate shallgraph_macros as shallgraph;

use clap::{Parser, Subcommand, ValueEnum};
use shallgraph_cli::{
    run_bootstrap, run_format, run_html_with_options, run_markdown, run_schema, run_validate,
    BootstrapOptions, HtmlReportOptions, SchemaOutputFormat,
};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "shallgraph",
    about = "ShallGraph – requirement discovery and validation",
    long_about = "ShallGraph – requirement discovery and validation\n\n\
Usage: shallgraph <command> [options]\n\n\
Pass -v or --version to print the semantic version and exit.\n\n\
Commands:\n  \
  validate   Validate requirement files\n  \
  format     Rewrite requirement YAML files to canonical formatting\n  \
  html       Generate an HTML report\n  \
  markdown   Generate a Markdown report\n  \
  schema     Print the requirement schema\n  \
  bootstrap  Initialize a shallgraph project"
)]
struct Cli {
    /// Print the semantic version and exit
    #[arg(short = 'v', long = "version", global = true)]
    version: bool,

    /// Project directory (default: current directory)
    #[arg(long, global = true, value_name = "DIR")]
    project_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Validate requirement files
    Validate,
    /// Rewrite requirement YAML files to canonical formatting
    Format,
    /// Generate an HTML report
    Html {
        /// Output directory (default: .)
        #[arg(short = 'o', long = "output", default_value = ".", value_name = "DIR")]
        output: PathBuf,
        /// Write the combined HTML report
        #[arg(long, conflicts_with = "pages")]
        combined: bool,
        /// Write one HTML file per requirement
        #[arg(long, value_name = "all|ID", conflicts_with = "combined")]
        pages: Option<String>,
        /// Base URL for links between pages
        #[arg(long, value_name = "URL", requires = "pages")]
        base_url: Option<String>,
    },
    /// Generate a Markdown report
    Markdown {
        /// Output directory (default: .)
        #[arg(short = 'o', long = "output", default_value = ".", value_name = "DIR")]
        output: PathBuf,
    },
    /// Print the requirement schema
    Schema {
        /// Output format
        #[arg(long, value_enum, default_value_t = SchemaFormatArg::JsonSchema)]
        format: SchemaFormatArg,
        /// Write to this file instead of stdout
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Initialize a shallgraph project
    Bootstrap {
        /// Overwrite existing project files
        #[arg(long)]
        force: bool,
        /// Add .cursor rules for requirements
        #[arg(long)]
        cursor_rules: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SchemaFormatArg {
    #[value(name = "json-schema")]
    JsonSchema,
    Yaml,
}

impl From<SchemaFormatArg> for SchemaOutputFormat {
    fn from(value: SchemaFormatArg) -> Self {
        match value {
            SchemaFormatArg::JsonSchema => SchemaOutputFormat::JsonSchema,
            SchemaFormatArg::Yaml => SchemaOutputFormat::Yaml,
        }
    }
}

fn ask_cursor_rules() -> bool {
    print!("Add .cursor rules for requirements? (y/N) ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    let normalized = answer.trim().to_lowercase();
    normalized == "y" || normalized == "yes"
}

/// GRD-CLI-011: print the shared package semantic version.
#[shallgraph::implements("GRD-CLI-011")]
fn print_semantic_version() {
    println!("{}", env!("CARGO_PKG_VERSION"));
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.version {
        print_semantic_version();
        return ExitCode::SUCCESS;
    }

    let project_dir = cli
        .project_dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let ok = match cli.command.unwrap_or(Command::Validate) {
        Command::Validate => match run_validate(&project_dir) {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{err}");
                false
            }
        },
        Command::Format => match run_format(&project_dir) {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{err}");
                false
            }
        },
        Command::Html {
            output,
            combined: _,
            pages,
            base_url,
        } => match run_html_with_options(
            &project_dir,
            &output,
            &HtmlReportOptions { pages, base_url },
            shallgraph_cli::discover_github_link_context,
        ) {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{err}");
                false
            }
        },
        Command::Markdown { output } => match run_markdown(&project_dir, &output) {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("{err}");
                false
            }
        },
        Command::Schema { format, output } => {
            match run_schema(&project_dir, format.into(), output.as_deref()) {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("{err}");
                    false
                }
            }
        }
        Command::Bootstrap {
            force,
            cursor_rules,
        } => {
            let mut cursor = cursor_rules;
            if !cursor && io::stdin().is_terminal() {
                cursor = ask_cursor_rules();
            }
            let result = run_bootstrap(
                &project_dir,
                BootstrapOptions {
                    force,
                    cursor_rules: cursor,
                },
            );
            if !result.success {
                if let Some(err) = result.error {
                    eprintln!("{err}");
                }
                false
            } else {
                let created: Vec<String> = result
                    .created
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect();
                println!("Created: {}", created.join(", "));
                true
            }
        }
    };

    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
