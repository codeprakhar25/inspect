//! Terminal rendering for inspect output.

use colored::Colorize;

use crate::inspector::{CommandIdentity, CommandInspection, CommandKind, InvocationAnalysis};
use crate::sources::ollama;
use crate::sources::{RiskLevel, RiskNote};

const SEP: &str = "──────────────────────────────────────";

/// Render a command lookup (Mode 1).
pub fn render_lookup(inspection: &CommandInspection) {
    if !inspection.found() && !inspection.has_docs() {
        render_not_found(inspection);
        return;
    }

    let sep = SEP.dimmed();
    let cmd = &inspection.command;

    println!("{sep}");

    // Header: command — summary
    match &inspection.summary {
        Some(summary) => {
            println!(" {} — {}", cmd.cyan().bold(), summary.white());
        }
        None => {
            println!(" {}", cmd.cyan().bold());
        }
    }

    println!();
    println!(
        " {}: {}",
        "IDENTITY".white().bold(),
        render_identity(&inspection.identity)
    );

    // Usage
    if let Some(usage) = &inspection.usage {
        println!();
        println!(" {}: {}", "USAGE".white().bold(), usage.white());
    }

    // Examples
    if !inspection.examples.is_empty() {
        println!();
        println!(" {}:", "EXAMPLES".white().bold());
        for example in inspection.examples.iter().take(5) {
            match &example.description {
                Some(description) => {
                    println!("  {}", example.command.green());
                    println!("      {}", truncate(description, 76).dimmed());
                }
                None => println!("  {}", example.command.green()),
            }
        }
    }

    // Flags
    if !inspection.flags.is_empty() {
        println!();
        println!(" {}:", "IMPORTANT FLAGS".white().bold());

        // Show at most 10 flags to avoid overwhelming output.
        for flag in inspection.flags.iter().take(10) {
            let name_col = format!("  {}", flag.names).yellow().to_string();
            if flag.description.is_empty() {
                println!("{name_col}");
            } else {
                let desc = truncate(&flag.description, 72);
                println!("{name_col:<30}  {}", desc.white());
            }
        }
        if inspection.flags.len() > 10 {
            println!(
                "  {} (and {} more — run `man {cmd}` for full list)",
                "…".dimmed(),
                inspection.flags.len() - 10
            );
        }
    }

    // Risks
    if !inspection.risks.is_empty() {
        println!();
        println!(" {}:", "RISKS".white().bold());
        for risk in inspection.risks.iter().take(4) {
            render_risk(risk);
        }
    }

    if !inspection.sources.is_empty() {
        println!();
        println!(
            " {}: {}",
            "SOURCES".white().bold(),
            inspection.sources.join(", ").dimmed()
        );
    }

    render_warnings(&inspection.warnings);

    println!("{sep}");
}

/// Render an invocation analysis (Mode 2).
pub fn render_invocation(analysis: &InvocationAnalysis) {
    let sep = SEP.dimmed();

    println!("{sep}");
    println!(
        " {}: {}",
        "Command".white().bold(),
        analysis.invocation.cyan().bold()
    );
    println!();

    if analysis.resolved_flags.is_empty() {
        println!("  {}", "(no flags detected)".dimmed());
    } else {
        for rf in &analysis.resolved_flags {
            let flag_col = format!("  {}", rf.typed).yellow().to_string();
            if rf.known {
                println!("{flag_col:<14}  {}", rf.description.white());
            } else {
                println!("{flag_col:<14}  {}", "(unknown flag)".dimmed());
            }
        }
    }

    println!();
    println!(
        " {}: {}",
        "Net effect".white().bold(),
        analysis.net_effect.white()
    );

    // Optionally show risks from the underlying docs.
    if let Some(docs) = &analysis.docs {
        if !docs.risks.is_empty() {
            println!();
            println!(" {}:", "RISKS".white().bold());
            for risk in docs.risks.iter().take(3) {
                render_risk(risk);
            }
        }
    }

    println!("{sep}");
}

/// Render the JSON output for a lookup.
pub fn render_json(inspection: &CommandInspection) {
    match serde_json::to_string_pretty(inspection) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("inspect: failed to serialize JSON: {error}");
            std::process::exit(1);
        }
    }
}

/// Render the JSON output for an invocation analysis.
pub fn render_json_invocation(analysis: &InvocationAnalysis) {
    match serde_json::to_string_pretty(analysis) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("inspect: failed to serialize JSON: {error}");
            std::process::exit(1);
        }
    }
}

fn render_identity(identity: &CommandIdentity) -> String {
    match identity.kind {
        CommandKind::External => identity
            .path
            .as_deref()
            .unwrap_or("external executable")
            .to_string(),
        CommandKind::Builtin => "shell builtin".to_string(),
        CommandKind::BuiltinAndExternal => {
            let path = identity.path.as_deref().unwrap_or("external executable");
            format!("shell builtin + {path}")
        }
        CommandKind::NotFound => "not found".to_string(),
    }
}

fn render_risk(risk: &RiskNote) {
    let label = match risk.level {
        RiskLevel::Info => "info".blue(),
        RiskLevel::Caution => "caution".yellow(),
        RiskLevel::Danger => "danger".red().bold(),
    };
    println!("  {label:<10} {}", truncate(&risk.message, 84).white());
}

fn render_warnings(warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    println!();
    println!(" {}:", "WARNINGS".white().bold());
    for warning in warnings.iter().take(4) {
        println!("  {}", warning.yellow());
    }
}

/// Truncate a string to `max` chars, appending "…" if cut.
fn truncate(s: &str, max: usize) -> String {
    // Collapse internal whitespace runs first (man pages have wrapped lines joined).
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let cut: String = collapsed.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

/// Render the results of `--find <intent>` (reverse lookup).
pub fn render_find(intent: &str, suggestions: &[ollama::CommandSuggestion]) {
    let sep = SEP.dimmed();
    println!("{sep}");
    println!(" Commands for: \"{}\"", intent.white());
    println!();
    for s in suggestions {
        println!("  {}", s.command.cyan().bold());
        println!("    {}", s.description.white());
        if let Some(example) = &s.example {
            println!("    {}", example.green());
        }
        println!();
    }
    println!("{sep}");
}

/// Render the results of `--explore <cmd>` (capability groups).
pub fn render_explore(cmd: &str, summary: Option<&str>, use_cases: &[ollama::UseCase]) {
    let sep = SEP.dimmed();
    println!("{sep}");
    match summary {
        Some(s) => println!(" {} — {}", cmd.cyan().bold(), s.white()),
        None => println!(" {}", cmd.cyan().bold()),
    }
    println!();
    for uc in use_cases {
        println!(" {}", uc.title.to_uppercase().white().bold());
        println!("   {}", uc.description.dimmed());
        for example in &uc.examples {
            println!("   {}", format!("$ {example}").green());
        }
        println!();
    }
    println!("{sep}");
}

/// Print a friendly error when a command's docs cannot be found.
pub fn render_not_found(inspection: &CommandInspection) {
    eprintln!(
        "{} command not found: '{}'",
        "inspect:".yellow().bold(),
        inspection.command.cyan()
    );
    eprintln!("         check spelling or PATH");
    for warning in &inspection.warnings {
        eprintln!("         warning: {warning}");
    }
}
