//! `inspect` - local command inspector for Linux/shell.
//!
//! # Usage
//!
//! ```text
//! inspect cp                       # lookup mode
//! inspect cp -rn src/ dest/        # explain-invocation mode
//! inspect git commit -m "fix"      # git-style subcommand explain
//! inspect --json cp                # machine-readable output
//! inspect --allow-help custom-tool
//! inspect --install-shell bash
//! inspect --llm some-obscure-cmd
//! ```

mod curated;
mod inspector;
mod render;
mod sources;

use std::io::IsTerminal;
use std::path::PathBuf;

use inspector::InspectOptions;

fn main() {
    // ── Manual arg parsing ───────────────────────────────────────────────────
    // Inspect's own flags are parsed only before the command. Once the first
    // non-option token is seen, all remaining tokens belong to the command
    // being inspected, so `inspect cargo --help` explains cargo's --help
    // instead of showing inspect's help.
    let raw: Vec<String> = std::env::args().skip(1).collect();

    let mut json = false;
    let mut allow_help = false;
    let mut no_help = false;
    let mut llm = false;
    let mut verbose = false;
    let mut llm_model: Option<String> = None;
    let mut llm_url: Option<String> = None;
    let mut install_shell: Option<String> = None;
    let mut query: Vec<String> = Vec::new();

    let mut i = 0;
    while i < raw.len() {
        if !query.is_empty() {
            query.push(raw[i].clone());
            i += 1;
            continue;
        }

        match raw[i].as_str() {
            "--json" => json = true,
            "--allow-help" => allow_help = true,
            "--no-help" => no_help = true,
            "--llm" => llm = true,
            "--verbose" | "-v" => verbose = true,
            "--llm-model" => {
                i += 1;
                if i < raw.len() {
                    llm_model = Some(raw[i].clone());
                } else {
                    eprintln!("inspect: --llm-model requires a model name");
                    std::process::exit(1);
                }
            }
            "--llm-url" => {
                i += 1;
                if i < raw.len() {
                    llm_url = Some(raw[i].clone());
                } else {
                    eprintln!("inspect: --llm-url requires a base URL");
                    std::process::exit(1);
                }
            }
            "--install-shell" => {
                i += 1;
                if i < raw.len() {
                    install_shell = Some(raw[i].clone());
                } else {
                    eprintln!("inspect: --install-shell requires an argument (bash or zsh)");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-V" => {
                println!("inspect {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--" => {
                query.extend(raw.iter().skip(i + 1).cloned());
                break;
            }
            // Everything else is part of the query (command name, flags, args).
            other => query.push(other.to_string()),
        }
        i += 1;
    }

    // ── --install-shell handled first ────────────────────────────────────────
    if let Some(shell) = &install_shell {
        install_shell_widget(shell);
        return;
    }

    colored::control::set_override(std::io::stdout().is_terminal() && !json);

    if query.is_empty() {
        eprintln!("Usage: inspect <command> [flags...] [args...]");
        std::process::exit(1);
    }

    let options = InspectOptions {
        allow_help,
        no_help,
        use_llm: llm,
        verbose,
        llm_model,
        llm_url,
    };

    // Mode selection:
    //   lookup  — single token, no leading dash, no internal space
    //   explain — multiple tokens, OR single token that starts with '-'
    let is_lookup = query.len() == 1 && !query[0].starts_with('-') && !query[0].contains(' ');

    if is_lookup {
        let command = &query[0];
        let inspection = inspector::inspect_command(command, options);

        if json {
            render::render_json(&inspection);
        } else {
            render::render_lookup(&inspection);
        }

        std::process::exit(if inspection.found() || inspection.has_docs() {
            0
        } else {
            1
        });
    } else {
        // Explain-invocation mode.
        let invocation = query.join(" ");
        let analysis = inspector::analyse_invocation(&invocation, options);

        if json {
            render::render_json_invocation(&analysis);
        } else {
            render::render_invocation(&analysis);
        }

        std::process::exit(0);
    }
}

fn print_help() {
    println!(
        "inspect {} — understand commands without leaving the terminal

USAGE:
    inspect [OPTIONS] <command>
    inspect [OPTIONS] <command> [flags...] [args...]

MODES:
    inspect cp                     lookup: show docs for 'cp'
    inspect cp -rn src/ dest/      explain: resolve each flag in the invocation
    inspect git commit -m \"fix\"    explain git subcommands

OPTIONS:
    --json                emit machine-readable JSON
    --allow-help          run <command> --help even for unknown commands
    --no-help             skip all <command> --help execution
    --llm                 enrich output via local Ollama instance
    --llm-model MODEL     Ollama model (default: env INSPECT_OLLAMA_MODEL or llama3.2)
    --llm-url URL         Ollama base URL (default: env INSPECT_OLLAMA_URL or http://127.0.0.1:11434)
    -v, --verbose         show source/fallback warnings
    --install-shell SHELL append shell widget to ~/.bashrc or ~/.zshrc
    -h, --help            show this help
    -V, --version         print version",
        env!("CARGO_PKG_VERSION")
    );
}

// ── Shell widget installation ─────────────────────────────────────────────────

fn install_shell_widget(shell: &str) {
    let shell_lower = shell.to_lowercase();
    let (widget_file, rc_file_name, widget_contents) = match shell_lower.as_str() {
        "bash" => (
            "inspect.bash",
            ".bashrc",
            include_str!("../shell/inspect.bash"),
        ),
        "zsh" => (
            "inspect.zsh",
            ".zshrc",
            include_str!("../shell/inspect.zsh"),
        ),
        other => {
            eprintln!(
                "inspect: unsupported shell '{}'. Use 'bash' or 'zsh'.",
                other
            );
            std::process::exit(1);
        }
    };

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| home.join(".local").join("share"))
        .join("inspect-cli");
    let widget_path = data_dir.join(widget_file);

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("inspect: failed to create {}: {e}", data_dir.display());
        std::process::exit(1);
    }

    if let Err(e) = std::fs::write(&widget_path, widget_contents) {
        eprintln!("inspect: failed to write {}: {e}", widget_path.display());
        std::process::exit(1);
    }

    let rc_path = home.join(rc_file_name);

    let source_line = format!(
        "# inspect shell widget\nsource \"{}\"",
        widget_path.display()
    );

    if std::fs::read_to_string(&rc_path)
        .map(|contents| contents.contains(&source_line))
        .unwrap_or(false)
    {
        println!(
            "inspect: shell widget already installed in {}",
            rc_path.display()
        );
        return;
    }

    match std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&rc_path)
    {
        Ok(mut file) => {
            use std::io::Write;
            let line = format!("\n{source_line}\n");
            if let Err(e) = file.write_all(line.as_bytes()) {
                eprintln!("inspect: failed to write to {}: {e}", rc_path.display());
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("inspect: failed to open {}: {e}", rc_path.display());
            std::process::exit(1);
        }
    }

    println!("inspect: added source line to {}", rc_path.display());
    println!(
        "         restart your shell or run: source {}",
        rc_path.display()
    );
}
