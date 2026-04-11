use std::collections::BTreeMap;
use std::path::Path;

use clap::ValueEnum;
use colored::Colorize;
use sem_core::parser::grep::{find_matches, result_json, GrepMatch, GrepQuery, GrepRefKind};
use sem_core::parser::plugins::create_default_registry;

pub struct GrepOptions {
    pub cwd: String,
    pub query: GrepQuery,
    pub json: bool,
    pub file_exts: Vec<String>,
    pub no_cache: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RefKind {
    Calls,
    Imports,
    Type,
}

impl From<RefKind> for GrepRefKind {
    fn from(value: RefKind) -> Self {
        match value {
            RefKind::Calls => Self::Calls,
            RefKind::Imports => Self::Imports,
            RefKind::Type => Self::Type,
        }
    }
}

pub fn grep_command(opts: GrepOptions) {
    let root = Path::new(&opts.cwd);
    let registry = create_default_registry();
    let ext_filter = super::graph::normalize_exts(&opts.file_exts);
    let file_paths = super::graph::find_supported_files_public(root, &registry, &ext_filter);
    let (graph, all_entities) =
        super::graph::get_or_build_graph(root, &file_paths, &registry, opts.no_cache);
    let matches = find_matches(&graph, &all_entities, &opts.query).unwrap_or_else(|err| {
        eprintln!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    });

    if opts.json {
        print_json(&matches, &opts.query);
    } else {
        print_terminal(&matches, &opts.query);
    }
}

fn print_terminal(matches: &[GrepMatch], query: &GrepQuery) {
    if matches.is_empty() {
        println!("{} {}", "grep:".green().bold(), "no matches".dimmed());
        return;
    }

    println!(
        "{} {}",
        "grep:".green().bold(),
        format!("{} matches", matches.len()).bold(),
    );

    if let Some(pattern) = query.pattern.as_deref() {
        println!("  {} {}", "pattern".dimmed(), pattern.bold());
    }
    if !query.entity_types.is_empty() {
        println!(
            "  {} {}",
            "types".dimmed(),
            query.entity_types.join(", ").bold()
        );
    }
    if let Some(path_substring) = query.path_substring.as_deref() {
        println!("  {} {}", "path".dimmed(), path_substring.bold());
    }
    if !query.depends_on.is_empty() {
        let relation = query.ref_kind.map_or("any ref", GrepRefKind::as_str);
        println!(
            "  {} {} ({})",
            "depends on".dimmed(),
            query.depends_on.join(", ").bold(),
            relation.dimmed(),
        );
    }
    println!();

    let mut by_file: BTreeMap<&str, Vec<&GrepMatch>> = BTreeMap::new();
    for matched in matches {
        by_file
            .entry(matched.file_path.as_str())
            .or_default()
            .push(matched);
    }

    for (file, entries) in by_file {
        println!("  {}", file.bold());
        for entry in entries {
            let mut suffix = format!(
                "L{}-{}, deps: {}, dependents: {}",
                entry.start_line, entry.end_line, entry.dependency_count, entry.dependent_count,
            );
            if entry.is_test {
                suffix.push_str(", test");
            }
            if !entry.matched_dependencies.is_empty() {
                suffix.push_str(&format!(", via: {}", entry.matched_dependencies.join(", ")));
            }

            println!(
                "    {} {} ({})",
                entry.entity_type.dimmed(),
                entry.name.bold(),
                suffix.dimmed(),
            );
        }
    }
}

fn print_json(matches: &[GrepMatch], query: &GrepQuery) {
    println!("{}", result_json(matches, query));
}
