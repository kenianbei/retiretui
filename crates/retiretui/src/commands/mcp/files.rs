use retiretui_engine::plan::{Issue, Scenario};
use retiretui_engine::project::validate_plan;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::PlanServer;
use super::store::PlanEntry;

#[derive(Deserialize, JsonSchema)]
pub struct PlanPathArgs {
    /// Plan file path, relative to the served directory.
    pub path: String,
}

#[derive(Serialize, JsonSchema)]
pub struct PlanList {
    /// Every `*.toml` file under the served directory.
    pub plans: Vec<PlanEntry>,
}

#[derive(Serialize, JsonSchema)]
pub struct PlanText {
    /// The raw file text, exactly as stored.
    pub text: String,
}

#[derive(Serialize, JsonSchema)]
pub struct IssueEntry {
    /// TOML-style path of the offending item.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

#[derive(Serialize, JsonSchema)]
pub struct IssueList {
    /// Every problem found; empty means the plan is valid.
    pub issues: Vec<IssueEntry>,
}

#[derive(Deserialize, JsonSchema)]
pub struct WritePlanArgs {
    /// Destination path, relative to the served directory; must end in
    /// `.toml`. Creating a new file is allowed.
    pub path: String,
    /// The complete plan document as TOML text.
    pub toml: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ImportEarningsArgs {
    /// Plan file path, relative to the served directory; a plan, not a
    /// scenario.
    pub path: String,
    /// The person the statement is for.
    pub person: String,
    /// The Social Security statement's XML text, as downloaded from
    /// ssa.gov.
    pub statement: String,
}

#[derive(Serialize, JsonSchema)]
pub struct WriteReply {
    /// Issues that blocked the write; empty means the plan was written.
    pub issues: Vec<IssueEntry>,
    /// Whether the stored canonical form differs from the submitted text;
    /// false when nothing was written.
    pub canonicalized: bool,
}

#[tool_router(vis = "pub(super)")]
impl PlanServer {
    /// List the plan files (`*.toml`) under the served directory.
    #[tool]
    fn list_plans(&self) -> Result<Json<PlanList>, String> {
        Ok(Json(PlanList {
            plans: self.store.list()?,
        }))
    }

    /// Read a plan file's raw TOML text.
    #[tool]
    fn read_plan(
        &self,
        Parameters(PlanPathArgs { path }): Parameters<PlanPathArgs>,
    ) -> Result<Json<PlanText>, String> {
        Ok(Json(PlanText {
            text: self.store.read(&path)?,
        }))
    }

    /// Validate a plan file: schema, semantic rules, and horizon-wide
    /// contribution limits. An empty issue list means the plan is valid.
    #[tool]
    fn validate_plan(
        &self,
        Parameters(PlanPathArgs { path }): Parameters<PlanPathArgs>,
    ) -> Result<Json<IssueList>, String> {
        let plan = self.store.load_plan(&path)?;
        Ok(Json(IssueList {
            issues: issue_entries(validate_plan(&plan, &self.tables)),
        }))
    }

    /// Write a complete plan or scenario document. The document is parsed
    /// and fully validated first - a scenario is resolved against its base
    /// chain in the served directory; on any issue nothing is written and
    /// the issues are returned. On success the document is stored in
    /// canonical TOML (comments and layout are not preserved), written
    /// atomically.
    #[tool]
    fn write_plan(
        &self,
        Parameters(WritePlanArgs { path, toml }): Parameters<WritePlanArgs>,
    ) -> Result<Json<WriteReply>, String> {
        self.store_document(&path, &toml).map(Json)
    }

    /// Record a Social Security statement's earnings on a person, writing
    /// the plan as `write_plan` does. Refused when the statement is for
    /// someone born on another date than the person.
    #[tool]
    fn import_earnings(
        &self,
        Parameters(ImportEarningsArgs {
            path,
            person,
            statement,
        }): Parameters<ImportEarningsArgs>,
    ) -> Result<Json<WriteReply>, String> {
        let text = self.store.read(&path)?;
        let plan = crate::commands::adopt_statement(&text, &person, &statement)
            .map_err(|reason| format!("{path}: {reason}"))?;
        let toml = plan.to_toml_string().map_err(|err| err.to_string())?;
        self.store_document(&path, &toml).map(Json)
    }
}

impl PlanServer {
    /// The one write gate: the document (plan or scenario) is resolved and
    /// fully validated, then stored canonically; on issues nothing is
    /// written.
    pub(super) fn store_document(&self, path: &str, toml: &str) -> Result<WriteReply, String> {
        let plan = self.store.resolve_document_at(path, toml)?;
        let issues = validate_plan(&plan, &self.tables);
        if !issues.is_empty() {
            return Ok(WriteReply {
                issues: issue_entries(issues),
                canonicalized: false,
            });
        }
        let scenario = Scenario::from_toml_str(toml).map_err(|err| format!("{path}: {err}"))?;
        let canonical = if let Some(scenario) = scenario {
            scenario.to_toml_string()
        } else {
            plan.to_toml_string()
        }
        .map_err(|err| err.to_string())?;
        self.store.write(path, &canonical)?;
        Ok(WriteReply {
            issues: Vec::new(),
            canonicalized: canonical != toml,
        })
    }
}

fn issue_entries(issues: Vec<Issue>) -> Vec<IssueEntry> {
    issues
        .into_iter()
        .map(|issue| IssueEntry {
            path: issue.path,
            message: issue.message,
        })
        .collect()
}
