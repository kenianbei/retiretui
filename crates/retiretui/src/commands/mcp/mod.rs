mod files;
mod markets;
mod optimize;
mod queries;
mod store;

use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use retiretui_engine::market::History;
use retiretui_engine::params::TaxTables;
use retiretui_engine::plan::Plan;
use retiretui_engine::project::validate_plan;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::stdio;
use rmcp::{ServerHandler, ServiceExt, tool_handler};

/// Arguments of the `mcp` subcommand.
#[derive(Args)]
pub struct McpArgs {
    /// Directory the server may read and write plan files in; defaults to
    /// the working directory. Tool paths cannot escape it.
    #[arg(long, default_value = ".", hide_default_value = true)]
    pub dir: PathBuf,
    /// Extra directory of tax parameter TOML files (repeatable).
    #[arg(long)]
    pub tax_dir: Vec<PathBuf>,
}

pub fn run(args: &McpArgs) -> anyhow::Result<()> {
    let root = args
        .dir
        .canonicalize()
        .with_context(|| format!("failed to open {}", args.dir.display()))?;
    let tables = super::load_tables(&args.tax_dir)?;
    let history = super::markets::load_history(None)?;
    let server = PlanServer::new(store::PlanStore::new(root), tables, history);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        server.serve(stdio()).await?.waiting().await?;
        Ok(())
    })
}

/// The MCP tool surface: a sandboxed plan store and the loaded tax tables.
pub struct PlanServer {
    store: store::PlanStore,
    tables: TaxTables,
    history: History,
    router: ToolRouter<Self>,
}

impl PlanServer {
    fn new(store: store::PlanStore, tables: TaxTables, history: History) -> Self {
        Self {
            store,
            tables,
            history,
            router: Self::tool_router()
                + Self::query_router()
                + Self::optimize_router()
                + Self::markets_router(),
        }
    }

    /// Loads and fully validates a plan, refusing an invalid one with its
    /// issue listing.
    fn load_valid_plan(&self, path: &str) -> Result<Plan, String> {
        let plan = self.store.load_plan(path)?;
        let issues = validate_plan(&plan, &self.tables);
        if issues.is_empty() {
            return Ok(plan);
        }
        Err(format!(
            "{path} is invalid:\n{}",
            super::issue_listing(&issues)
        ))
    }
}

const INSTRUCTIONS: &str = "Deterministic U.S. retirement projections over \
plan TOML files in the served directory. Call describe_schema before \
composing a plan; write_plan validates and refuses invalid documents; \
project_plan reports nominal dollars with a per-year deflator.";

#[tool_handler(router = self.router)]
#[expect(
    clippy::unused_async_trait_impl,
    reason = "the async fns are rmcp's tool_handler expansion"
)]
impl ServerHandler for PlanServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("retiretui", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}
