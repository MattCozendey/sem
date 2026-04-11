use serde::Deserialize;

// ── Tool parameter structs ──

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EntitiesParams {
    #[schemars(description = "Path to the file (relative to repo root or absolute)")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DiffParams {
    #[schemars(description = "Base ref to compare from (branch, tag, or commit hash, e.g. 'main')")]
    pub base_ref: String,
    #[schemars(description = "Target ref to compare to. Defaults to HEAD.")]
    pub target_ref: Option<String>,
    #[schemars(description = "Optional: diff only this file")]
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlameParams {
    #[schemars(description = "Path to the file (relative to repo root or absolute)")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ImpactAnalysisParams {
    #[schemars(description = "Path to the file containing the entity")]
    pub file_path: String,
    #[schemars(description = "Name of the entity to analyze impact for")]
    pub entity_name: String,
    #[schemars(description = "Analysis mode: 'all' (default, shows deps + dependents + transitive impact + tests), 'deps' (direct dependencies only), 'dependents' (direct dependents only), 'tests' (affected test entities only)")]
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LogParams {
    #[schemars(description = "Name of the entity to trace history for")]
    pub entity_name: String,
    #[schemars(description = "Path to the file containing the entity. If omitted, auto-detects.")]
    pub file_path: Option<String>,
    #[schemars(description = "Maximum number of commits to analyze. Defaults to 50.")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GrepParams {
    #[schemars(description = "Entity name pattern. If omitted, provide at least one filter.")]
    pub pattern: Option<String>,
    #[serde(default)]
    #[schemars(description = "Also search inside entity content. Requires pattern.")]
    pub content: bool,
    #[serde(default)]
    #[schemars(description = "Match text filters literally, including case and identifier separators.")]
    pub case_sensitive: bool,
    #[serde(default, alias = "types", alias = "type")]
    #[schemars(description = "Only include these entity types, such as function or class.")]
    pub entity_types: Vec<String>,
    #[schemars(description = "Only include entities whose file path contains this substring.")]
    pub path: Option<String>,
    #[serde(default)]
    #[schemars(description = "Only include entities that look like tests.")]
    pub tests: bool,
    #[serde(default)]
    #[schemars(description = "Only include entities that directly depend on a matching entity name.")]
    pub depends_on: Vec<String>,
    #[schemars(description = "Restrict depends_on matches to one reference kind: calls, imports, or type.")]
    pub ref_kind: Option<String>,
    #[schemars(description = "Only include entities with at least this many direct dependencies.")]
    pub min_dependencies: Option<usize>,
    #[schemars(description = "Only include entities with at least this many direct dependents.")]
    pub min_dependents: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Only include files with these extensions, such as .py or .rs.")]
    pub file_exts: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ContextParams {
    #[schemars(description = "Path to the file containing the entity")]
    pub file_path: String,
    #[schemars(description = "Name of the target entity")]
    pub entity_name: String,
    #[schemars(description = "Maximum token budget. Defaults to 8000.")]
    pub token_budget: Option<usize>,
}
