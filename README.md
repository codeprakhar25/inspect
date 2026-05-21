# inspect

Understand any shell command without leaving your terminal.

```
$ inspect cp -rn src/ dest/
──────────────────────────────────────
 Command: cp -rn src/ dest/

  -r  copy directories recursively
  -n  will NOT overwrite existing files in dest/

 Net effect: safe incremental copy. New files added, nothing clobbered.
──────────────────────────────────────
```

---

## What it does

`inspect` has two modes:

**Lookup** — show docs for a command

```bash
inspect git
inspect rsync
inspect tar
```

**Explain an invocation** — break down exactly what the flags you typed do

```bash
inspect cp -rn src/ dest/
inspect git push --force-with-lease origin main
inspect tar -xzf archive.tar.gz -C /tmp
```

After lookup, an interactive prompt lets you ask follow-up questions (requires [Ollama](#llm-enrichment)):

```
  ask anything (enter to exit): when would I use -n instead of -i?
  -n skips the file entirely without prompting. Use it in scripts where you
  want idempotent copies. Use -i when you want interactive confirmation.
```

---

## Install

### From source (requires Rust)

```bash
git clone https://github.com/codeprakhar25/inspect.git
cd inspect
cargo build --release
```

Copy the binary somewhere on your PATH:

```bash
cp target/release/inspect ~/.local/bin/
# or
sudo cp target/release/inspect /usr/local/bin/
```

---

## Shell widget (Ctrl+])

The real power: press **Ctrl+]** while typing any command to explain it inline — without losing your current prompt.

```bash
# Install the widget (run once)
inspect --install-shell bash   # or zsh

# Reload your shell
source ~/.bashrc   # or ~/.zshrc
```

Now type any command at your prompt and press **Ctrl+]**:

```
$ rsync -avz --progress src/ dest/    ← press Ctrl+]

  -a  archive mode (preserves permissions, timestamps, symlinks)
  -v  verbose
  -z  compress during transfer
  --progress  show per-file progress bar

$ rsync -avz --progress src/ dest/    ← cursor returned here
```

---

## LLM enrichment

`inspect` uses a locally-running [Ollama](https://ollama.com) instance to enrich docs for obscure commands and power the interactive Q&A prompt.

```bash
# Install Ollama, then pull a model
ollama pull llama3.2

# Use LLM enrichment
inspect --llm some-obscure-tool

# Custom model or URL
inspect --llm --llm-model qwen2.5 --llm-url http://localhost:11434 curl
```

LLM responses are cached under `~/.cache/inspect-cli/ollama/` so repeated lookups are instant.

Without Ollama, `inspect` works fully for all well-known commands using man pages, `--help` output, and a built-in curated knowledge base.

---

## Usage

```
inspect [OPTIONS] <command>
inspect [OPTIONS] <command> [flags...] [args...]

OPTIONS:
    --json                emit machine-readable JSON
    --allow-help          run <command> --help even for unknown commands
    --no-help             skip all <command> --help execution
    --llm                 enrich output via local Ollama instance
    --llm-model MODEL     Ollama model (default: llama3.2)
    --llm-url URL         Ollama base URL (default: http://127.0.0.1:11434)
    -v, --verbose         show source/fallback warnings
    --install-shell SHELL append shell widget to ~/.bashrc or ~/.zshrc
    -h, --help            show this help
    -V, --version         print version
```

---

## How it works

Documentation is assembled from multiple sources in priority order:

1. **Curated knowledge base** — hand-written entries for 40+ common commands (`cp`, `git`, `rsync`, `tar`, `find`, `curl`, …)
2. **`man` pages** — parsed and cleaned (strips groff formatting, encoding artifacts)
3. **`--help` output** — flag and usage extraction via regex
4. **Ollama LLM** — enrichment and Q&A for everything else (optional, cached)

For git-style subcommands (`git commit`, `git push`), each subcommand is looked up and explained individually.

---

## Contributing

Contributions welcome. The most impactful thing you can add is curated entries for commands you use daily — they give cleaner output than raw man pages.

Curated entries live in [`src/curated.rs`](src/curated.rs). Each entry is a Rust struct with a summary, usage, flags, examples, and risk notes. Existing entries are the best reference for format.

```bash
cargo test        # run the test suite
cargo clippy      # lint
cargo fmt         # format
```

---

## License

MIT
