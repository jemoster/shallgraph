//! shallgraph – requirement discovery and validation (GRD-CLI-008).

use clap::{Parser, Subcommand, ValueEnum};
use shallgraph_cli::{
    run_bootstrap, run_format, run_html, run_markdown, run_schema, run_validate, BootstrapOptions,
    SchemaOutputFormat,
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
Commands:\n  \
  validate   Check requirement files for schema, duplicate IDs, and broken links\n  \
  format     Rewrite requirement YAML files to canonical formatting\n  \
  html       Generate an HTML report of all requirements\n  \
  markdown   Generate a Markdown report of all requirements\n  \
  schema     Print the requirement schema for the current project (JSON Schema or YAML)\n  \
  bootstrap  Initialize a directory with shallgraph.yaml and a requirements folder"
)]
struct Cli {
    /// Project directory (default: current directory)
    #[arg(long, global = true, value_name = "DIR")]
    project_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Check requirement files for schema, duplicate IDs, and broken links
    Validate,
    /// Rewrite requirement YAML files to canonical formatting
    Format,
    /// Generate an HTML report of all requirements
    Html {
        /// Output directory for index.html (default: .)
        #[arg(short = 'o', long = "output", default_value = ".", value_name = "DIR")]
        output: PathBuf,
    },
    /// Generate a Markdown report of all requirements
    Markdown {
        /// Output directory for index.md (default: .)
        #[arg(short = 'o', long = "output", default_value = ".", value_name = "DIR")]
        output: PathBuf,
    },
    /// Print the requirement schema for the current project
    Schema {
        /// Output format: json-schema (default) or yaml
        #[arg(long, value_enum, default_value_t = SchemaFormatArg::JsonSchema)]
        format: SchemaFormatArg,
        /// Write to this file instead of stdout
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Initialize a directory with shallgraph.yaml and a requirements folder
    Bootstrap {
        /// Overwrite existing shallgraph.yaml; do not fail if requirements folder exists
        #[arg(long)]
        force: bool,
        /// Add .cursor rules for requirements (without prompting)
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

fn main() -> ExitCode {
    let cli = Cli::parse();
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
        Command::Html { output } => match run_html(&project_dir, &output) {
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
