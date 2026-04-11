use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use serde_json::json;

use crate::model::entity::SemanticEntity;
use crate::parser::graph::{EntityGraph, EntityInfo, EntityRef, RefType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrepRefKind {
    Calls,
    Imports,
    Type,
}

impl GrepRefKind {
    pub fn matches(self, ref_type: &RefType) -> bool {
        match self {
            Self::Calls => *ref_type == RefType::Calls,
            Self::Imports => *ref_type == RefType::Imports,
            Self::Type => *ref_type == RefType::TypeRef,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Type => "type",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ParseGrepRefKindError;

impl fmt::Display for ParseGrepRefKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ref_kind must be one of: calls, imports, type")
    }
}

impl std::error::Error for ParseGrepRefKindError {}

impl FromStr for GrepRefKind {
    type Err = ParseGrepRefKindError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "calls" => Ok(Self::Calls),
            "imports" => Ok(Self::Imports),
            "type" => Ok(Self::Type),
            _ => Err(ParseGrepRefKindError),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct GrepQuery {
    pub pattern: Option<String>,
    pub content: bool,
    pub case_sensitive: bool,
    pub entity_types: Vec<String>,
    pub path_substring: Option<String>,
    pub tests: bool,
    pub depends_on: Vec<String>,
    pub ref_kind: Option<GrepRefKind>,
    pub min_dependencies: Option<usize>,
    pub min_dependents: Option<usize>,
}

impl GrepQuery {
    pub fn validate(&self) -> Result<(), GrepValidationError> {
        if !self.has_selector() {
            return Err(GrepValidationError::MissingSelector);
        }
        if self.content && self.pattern.is_none() {
            return Err(GrepValidationError::ContentRequiresPattern);
        }
        if self.ref_kind.is_some() && self.depends_on.is_empty() {
            return Err(GrepValidationError::RefKindRequiresDependsOn);
        }
        Ok(())
    }

    fn has_selector(&self) -> bool {
        self.pattern.is_some()
            || !self.entity_types.is_empty()
            || self.path_substring.is_some()
            || self.tests
            || !self.depends_on.is_empty()
            || self.min_dependencies.is_some()
            || self.min_dependents.is_some()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum GrepValidationError {
    MissingSelector,
    ContentRequiresPattern,
    RefKindRequiresDependsOn,
}

impl fmt::Display for GrepValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSelector => f.write_str(
                "provide a pattern or at least one filter (`--type`, `--path`, `--tests`, `--depends-on`, `--min-dependencies`, `--min-dependents`)",
            ),
            Self::ContentRequiresPattern => f.write_str("`--content` requires a pattern"),
            Self::RefKindRequiresDependsOn => {
                f.write_str("`--ref-kind` requires at least one `--depends-on` filter")
            }
        }
    }
}

impl std::error::Error for GrepValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrepMatch {
    pub name: String,
    pub entity_type: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_id: Option<String>,
    pub dependency_count: usize,
    pub dependent_count: usize,
    pub is_test: bool,
    pub matched_dependencies: Vec<String>,
}

pub fn find_matches(
    graph: &EntityGraph,
    all_entities: &[SemanticEntity],
    query: &GrepQuery,
) -> Result<Vec<GrepMatch>, GrepValidationError> {
    query.validate()?;

    let entities_by_id: HashMap<&str, &SemanticEntity> = all_entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect();
    let test_ids = if query.tests {
        graph.filter_test_entities(all_entities)
    } else {
        HashSet::new()
    };
    let outgoing_edges = build_outgoing_edges(&graph.edges);
    let mut matches = Vec::new();

    for entity in graph.entities.values() {
        let source = match entities_by_id.get(entity.id.as_str()) {
            Some(source) => *source,
            None => continue,
        };

        if !matches_pattern(
            entity,
            &source.content,
            query.pattern.as_deref(),
            query.content,
            query.case_sensitive,
        ) {
            continue;
        }
        if !matches_type(&entity.entity_type, &query.entity_types, query.case_sensitive) {
            continue;
        }
        if let Some(path_substring) = query.path_substring.as_deref() {
            if !matches_text(&entity.file_path, path_substring, query.case_sensitive) {
                continue;
            }
        }
        if query.tests && !test_ids.contains(&entity.id) {
            continue;
        }

        let dependency_count = graph
            .dependencies
            .get(entity.id.as_str())
            .map_or(0, Vec::len);
        let dependent_count = graph.dependents.get(entity.id.as_str()).map_or(0, Vec::len);

        if let Some(min_dependencies) = query.min_dependencies {
            if dependency_count < min_dependencies {
                continue;
            }
        }
        if let Some(min_dependents) = query.min_dependents {
            if dependent_count < min_dependents {
                continue;
            }
        }

        let matched_dependencies = if query.depends_on.is_empty() {
            Vec::new()
        } else {
            find_matching_dependencies(
                graph,
                entity,
                &outgoing_edges,
                &query.depends_on,
                query.ref_kind,
                query.case_sensitive,
            )
        };

        if !query.depends_on.is_empty() && matched_dependencies.is_empty() {
            continue;
        }

        matches.push(GrepMatch {
            name: entity.name.clone(),
            entity_type: entity.entity_type.clone(),
            file_path: entity.file_path.clone(),
            start_line: entity.start_line,
            end_line: entity.end_line,
            parent_id: entity.parent_id.clone(),
            dependency_count,
            dependent_count,
            is_test: test_ids.contains(&entity.id),
            matched_dependencies,
        });
    }

    matches.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then(left.start_line.cmp(&right.start_line))
            .then(left.end_line.cmp(&right.end_line))
            .then(left.name.cmp(&right.name))
    });
    Ok(matches)
}

pub fn result_json(matches: &[GrepMatch], query: &GrepQuery) -> serde_json::Value {
    json!({
        "summary": {
            "matches": matches.len(),
            "pattern": query.pattern.as_deref(),
            "content": query.content,
            "caseSensitive": query.case_sensitive,
            "types": &query.entity_types,
            "path": query.path_substring.as_deref(),
            "tests": query.tests,
            "dependsOn": &query.depends_on,
            "refKind": query.ref_kind.map(GrepRefKind::as_str),
            "minDependencies": query.min_dependencies,
            "minDependents": query.min_dependents,
        },
        "matches": matches.iter().map(|entry| json!({
            "name": &entry.name,
            "type": &entry.entity_type,
            "file": &entry.file_path,
            "lines": [entry.start_line, entry.end_line],
            "parentId": &entry.parent_id,
            "dependencyCount": entry.dependency_count,
            "dependentCount": entry.dependent_count,
            "isTest": entry.is_test,
            "matchedDependencies": &entry.matched_dependencies,
        })).collect::<Vec<_>>(),
    })
}

fn build_outgoing_edges<'a>(edges: &'a [EntityRef]) -> HashMap<&'a str, Vec<&'a EntityRef>> {
    let mut outgoing: HashMap<&str, Vec<&EntityRef>> = HashMap::new();
    for edge in edges {
        outgoing
            .entry(edge.from_entity.as_str())
            .or_default()
            .push(edge);
    }
    outgoing
}

fn find_matching_dependencies(
    graph: &EntityGraph,
    entity: &EntityInfo,
    outgoing_edges: &HashMap<&str, Vec<&EntityRef>>,
    depends_on: &[String],
    ref_kind: Option<GrepRefKind>,
    case_sensitive: bool,
) -> Vec<String> {
    let mut matched: HashSet<String> = HashSet::new();

    if let Some(edges) = outgoing_edges.get(entity.id.as_str()) {
        for edge in edges {
            if let Some(kind) = ref_kind {
                if !kind.matches(&edge.ref_type) {
                    continue;
                }
            }

            let target = match graph.entities.get(edge.to_entity.as_str()) {
                Some(target) => target,
                None => continue,
            };

            if depends_on
                .iter()
                .any(|pattern| matches_text(&target.name, pattern, case_sensitive))
            {
                matched.insert(target.name.clone());
            }
        }
    }

    let mut matched: Vec<_> = matched.into_iter().collect();
    matched.sort();
    matched
}

fn matches_pattern(
    entity: &EntityInfo,
    content: &str,
    pattern: Option<&str>,
    search_content: bool,
    case_sensitive: bool,
) -> bool {
    let Some(pattern) = pattern else {
        return true;
    };

    matches_text(&entity.name, pattern, case_sensitive)
        || (search_content && matches_text(content, pattern, case_sensitive))
}

fn matches_type(value: &str, filters: &[String], case_sensitive: bool) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| {
            if case_sensitive {
                value == filter
            } else {
                value.eq_ignore_ascii_case(filter)
            }
        })
}

fn matches_text(haystack: &str, needle: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        return haystack.contains(needle);
    }

    let needle_lower = needle.to_lowercase();
    if haystack.to_lowercase().contains(&needle_lower) {
        return true;
    }

    let normalized_needle = normalize_identifier_text(needle);
    !normalized_needle.is_empty()
        && normalize_identifier_text(haystack).contains(&normalized_needle)
}

fn normalize_identifier_text(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '_' | '-') {
            continue;
        }
        normalized.extend(ch.to_lowercase());
    }
    normalized
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::model::entity::SemanticEntity;
    use crate::parser::graph::{EntityGraph, EntityInfo, EntityRef, RefType};

    use super::{find_matches, matches_text, GrepQuery, GrepRefKind};

    fn entity_info(
        id: &str,
        name: &str,
        entity_type: &str,
        file_path: &str,
        start_line: usize,
    ) -> EntityInfo {
        EntityInfo {
            id: id.to_string(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            file_path: file_path.to_string(),
            parent_id: None,
            start_line,
            end_line: start_line + 2,
        }
    }

    fn semantic_entity(info: &EntityInfo, content: &str) -> SemanticEntity {
        SemanticEntity {
            id: info.id.clone(),
            file_path: info.file_path.clone(),
            entity_type: info.entity_type.clone(),
            name: info.name.clone(),
            parent_id: info.parent_id.clone(),
            content: content.to_string(),
            content_hash: format!("hash-{}", info.id),
            structural_hash: None,
            start_line: info.start_line,
            end_line: info.end_line,
            metadata: None,
        }
    }

    fn test_query() -> GrepQuery {
        GrepQuery {
            pattern: None,
            content: false,
            case_sensitive: false,
            entity_types: Vec::new(),
            path_substring: None,
            tests: false,
            depends_on: Vec::new(),
            ref_kind: None,
            min_dependencies: None,
            min_dependents: None,
        }
    }

    fn fixture_graph() -> (EntityGraph, Vec<SemanticEntity>) {
        let exec = entity_info(
            "src/process.ts::function::exec",
            "exec",
            "function",
            "src/process.ts",
            1,
        );
        let login = entity_info(
            "src/auth.ts::function::login",
            "login",
            "function",
            "src/auth.ts",
            10,
        );
        let run_shell = entity_info(
            "src/shell.ts::function::runShell",
            "runShell",
            "function",
            "src/shell.ts",
            20,
        );
        let auth_controller = entity_info(
            "src/auth_controller.ts::class::AuthController",
            "AuthController",
            "class",
            "src/auth_controller.ts",
            5,
        );
        let login_test = entity_info(
            "tests/auth_test.rs::function::test_login",
            "test_login",
            "function",
            "tests/auth_test.rs",
            3,
        );

        let graph = EntityGraph::from_parts(
            HashMap::from([
                (exec.id.clone(), exec.clone()),
                (login.id.clone(), login.clone()),
                (run_shell.id.clone(), run_shell.clone()),
                (auth_controller.id.clone(), auth_controller.clone()),
                (login_test.id.clone(), login_test.clone()),
            ]),
            vec![
                EntityRef {
                    from_entity: run_shell.id.clone(),
                    to_entity: exec.id.clone(),
                    ref_type: RefType::Calls,
                },
                EntityRef {
                    from_entity: auth_controller.id.clone(),
                    to_entity: login.id.clone(),
                    ref_type: RefType::TypeRef,
                },
                EntityRef {
                    from_entity: login_test.id.clone(),
                    to_entity: login.id.clone(),
                    ref_type: RefType::Calls,
                },
            ],
        );

        let entities = vec![
            semantic_entity(&exec, "export function exec(command) { return command; }"),
            semantic_entity(
                &login,
                "pub fn login(user: &str) -> bool { !user.is_empty() }",
            ),
            semantic_entity(&run_shell, "function runShell() { return exec(\"ls\"); }"),
            semantic_entity(
                &auth_controller,
                "class AuthController extends login { handle() { return true; } }",
            ),
            semantic_entity(
                &login_test,
                "#[test]\nfn test_login() {\n    assert!(login(\"alice\"));\n}",
            ),
        ];

        (graph, entities)
    }

    #[test]
    fn grep_matches_name_by_default_and_content_when_requested() {
        let (graph, entities) = fixture_graph();

        let mut query = test_query();
        query.pattern = Some("exec".to_string());

        let name_only = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(name_only.len(), 1);
        assert_eq!(name_only[0].name, "exec");

        query.content = true;
        let with_content = find_matches(&graph, &entities, &query).expect("valid grep query");
        let names: Vec<_> = with_content
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["exec", "runShell"]);
    }

    #[test]
    fn grep_matches_common_identifier_styles_by_default() {
        let (graph, entities) = fixture_graph();
        let mut query = test_query();

        query.pattern = Some("auth-controller".to_string());
        let kebab_to_pascal = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(kebab_to_pascal.len(), 1);
        assert_eq!(kebab_to_pascal[0].name, "AuthController");

        query.pattern = Some("RUN_SHELL".to_string());
        let screaming_to_camel = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(screaming_to_camel.len(), 1);
        assert_eq!(screaming_to_camel[0].name, "runShell");
    }

    #[test]
    fn grep_case_sensitive_requires_literal_text() {
        let (graph, entities) = fixture_graph();
        let mut query = test_query();
        query.case_sensitive = true;
        query.pattern = Some("RUN_SHELL".to_string());

        let matches = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert!(matches.is_empty());

        query.pattern = Some("runShell".to_string());
        let matches = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "runShell");
    }

    #[test]
    fn smart_text_matching_is_not_sparse() {
        assert!(matches_text("authenticate_user", "authenticateUser", false));
        assert!(matches_text("AUTHENTICATE-USER", "authenticate_user", false));
        assert!(!matches_text("authenticate user", "authenticateUser", false));
        assert!(!matches_text("handle_mouse_event", "hm", false));
        assert!(!matches_text("authenticate_user", "authenticateUser", true));
    }

    #[test]
    fn grep_filters_by_dependency_name_and_reference_kind() {
        let (graph, entities) = fixture_graph();
        let mut query = test_query();
        query.depends_on = vec!["login".to_string()];

        let any_ref = find_matches(&graph, &entities, &query).expect("valid grep query");
        let names: Vec<_> = any_ref.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["AuthController", "test_login"]);

        query.ref_kind = Some(GrepRefKind::Calls);
        let calls_only = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(calls_only.len(), 1);
        assert_eq!(calls_only[0].name, "test_login");
        assert_eq!(calls_only[0].matched_dependencies, vec!["login"]);
    }

    #[test]
    fn grep_combines_test_and_graph_threshold_filters() {
        let (graph, entities) = fixture_graph();
        let mut query = test_query();
        query.tests = true;
        query.depends_on = vec!["login".to_string()];
        query.min_dependencies = Some(1);

        let matches = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "test_login");
        assert!(matches[0].is_test);

        query.tests = false;
        query.depends_on.clear();
        query.min_dependencies = None;
        query.min_dependents = Some(2);

        let high_fan_in = find_matches(&graph, &entities, &query).expect("valid grep query");
        assert_eq!(high_fan_in.len(), 1);
        assert_eq!(high_fan_in[0].name, "login");
    }

    #[test]
    fn grep_rejects_filterless_queries() {
        let (graph, entities) = fixture_graph();
        let err = find_matches(&graph, &entities, &test_query())
            .expect_err("filterless grep should fail");
        assert_eq!(err.to_string(), "provide a pattern or at least one filter (`--type`, `--path`, `--tests`, `--depends-on`, `--min-dependencies`, `--min-dependents`)");
    }
}
