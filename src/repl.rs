//! Interactive streaming Q&A REPL shown after lookup output.

use std::io::Write;

use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::inspector::{CommandInspection, InspectOptions};
use crate::sources::ollama::Config as OllamaConfig;

const SEP: &str = "──────────────────────────────────────";

pub fn run(cmd: &str, inspection: &CommandInspection, options: &InspectOptions) {
    let config =
        OllamaConfig::from_options(options.llm_model.as_deref(), options.llm_url.as_deref());
    let endpoint = config.generate_endpoint();

    let system_ctx = build_context(cmd, inspection);
    let mut history: Vec<(String, String)> = Vec::new();

    let mut rl = match DefaultEditor::new() {
        Ok(e) => e,
        Err(_) => return,
    };

    println!();

    loop {
        let prompt_str = format!("  {} ", "ask anything (enter to exit):".dimmed());
        let line = match rl.readline(&prompt_str) {
            Ok(l) => l,
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(_) => break,
        };

        let question = line.trim().to_string();
        if question.is_empty() {
            break;
        }

        let _ = rl.add_history_entry(&question);

        let full_prompt = build_prompt(&system_ctx, &history, &question);

        println!();
        match stream_answer(&full_prompt, &config.model, &endpoint) {
            Ok(answer) => {
                history.push((question, answer));
            }
            Err(_) if history.is_empty() => {
                println!(
                    "  {}",
                    "no LLM configured — start Ollama to ask questions".dimmed()
                );
                break;
            }
            Err(e) => {
                println!("  {}", format!("error: {e}").red());
            }
        }
        println!();
    }

    println!("{}", SEP.dimmed());
}

fn build_context(cmd: &str, inspection: &CommandInspection) -> String {
    let mut ctx = format!("Command: `{cmd}`\n");

    if let Some(summary) = &inspection.summary {
        ctx.push_str(&format!("Summary: {summary}\n"));
    }

    if !inspection.flags.is_empty() {
        ctx.push_str("Key flags:\n");
        for flag in inspection.flags.iter().take(6) {
            ctx.push_str(&format!("  {}: {}\n", flag.names, flag.description));
        }
    }

    if !inspection.examples.is_empty() {
        ctx.push_str("Examples:\n");
        for ex in inspection.examples.iter().take(3) {
            ctx.push_str(&format!("  {}\n", ex.command));
        }
    }

    ctx
}

fn build_prompt(system_ctx: &str, history: &[(String, String)], question: &str) -> String {
    let mut prompt = format!(
        "You are a CLI reference assistant. Answer questions concisely in plain text, no markdown.\n\n{system_ctx}\n"
    );

    for (q, a) in history {
        prompt.push_str(&format!("User: {q}\nAssistant: {a}\n\n"));
    }

    prompt.push_str(&format!(
        "User: {question}\nAnswer concisely in plain text, no markdown:"
    ));
    prompt
}

fn stream_answer(prompt: &str, model: &str, endpoint: &str) -> Result<String, String> {
    use std::io::BufRead;

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": true,
        "options": { "temperature": 0.3 }
    });

    let response = ureq::post(endpoint)
        .set("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| e.to_string())?;

    let reader = std::io::BufReader::new(response.into_reader());
    let mut full = String::new();

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.is_empty() {
            continue;
        }
        let chunk: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if let Some(token) = chunk["response"].as_str() {
            print!("{token}");
            std::io::stdout().flush().ok();
            full.push_str(token);
        }
        if chunk["done"].as_bool().unwrap_or(false) {
            break;
        }
    }

    println!();
    Ok(full)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspector::{CommandIdentity, CommandInspection, CommandKind};
    use crate::sources::{Example, Flag};

    fn make_inspection(cmd: &str) -> CommandInspection {
        CommandInspection {
            command: cmd.to_string(),
            doc_command: cmd.to_string(),
            identity: CommandIdentity {
                kind: CommandKind::External,
                path: Some("/usr/bin/cp".to_string()),
                builtin_shell: None,
            },
            summary: Some("copy files and directories".to_string()),
            usage: None,
            flags: vec![
                Flag {
                    names: "-r, --recursive".to_string(),
                    description: "copy directories recursively".to_string(),
                },
                Flag {
                    names: "-n, --no-clobber".to_string(),
                    description: "do not overwrite".to_string(),
                },
            ],
            examples: vec![Example {
                command: "cp file.txt dest/".to_string(),
                description: None,
            }],
            risks: vec![],
            sources: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn build_context_includes_summary_and_flags() {
        let inspection = make_inspection("cp");
        let ctx = build_context("cp", &inspection);
        assert!(ctx.contains("Command: `cp`"));
        assert!(ctx.contains("copy files and directories"));
        assert!(ctx.contains("-r, --recursive"));
        assert!(ctx.contains("copy directories recursively"));
    }

    #[test]
    fn build_context_includes_examples() {
        let inspection = make_inspection("cp");
        let ctx = build_context("cp", &inspection);
        assert!(ctx.contains("cp file.txt dest/"));
    }

    #[test]
    fn build_prompt_no_history() {
        let ctx = "Command: `cp`\nSummary: copy files\n".to_string();
        let prompt = build_prompt(&ctx, &[], "what does -n do?");
        assert!(prompt.contains("Command: `cp`"));
        assert!(prompt.contains("User: what does -n do?"));
        assert!(!prompt.contains("Assistant:"));
    }

    #[test]
    fn build_prompt_with_history() {
        let ctx = "Command: `cp`\n".to_string();
        let history = vec![(
            "what is -r?".to_string(),
            "It copies directories recursively.".to_string(),
        )];
        let prompt = build_prompt(&ctx, &history, "what about -n?");
        assert!(prompt.contains("User: what is -r?"));
        assert!(prompt.contains("Assistant: It copies directories recursively."));
        assert!(prompt.contains("User: what about -n?"));
    }
}
