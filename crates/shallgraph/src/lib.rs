//! GRD-CLI-008: Essential command-line interface (bootstrap, validate, html, schema).

extern crate shallgraph_macros as shallgraph;

pub mod bootstrap;
pub mod format;
pub mod html;
pub mod markdown;
pub mod schema;
pub mod validate;

pub use bootstrap::{run_bootstrap, BootstrapOptions, BootstrapResult};
pub use format::run_format;
pub use html::{discover_github_link_context, run_html, run_html_with_github};
pub use markdown::{run_markdown, run_markdown_with_github};
pub use schema::{run_schema, SchemaOutputFormat};
pub use validate::run_validate;
