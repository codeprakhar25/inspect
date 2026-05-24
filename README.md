<div align="center">

# 🔍 inspect

**Understand any shell command without leaving your terminal.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Platform: Linux](https://img.shields.io/badge/Platform-Linux-blue?logo=linux)](https://github.com/codeprakhar25/inspect)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/codeprakhar25/inspect/pulls)

[![asciicast](https://asciinema.org/a/acFWWlMKeCvpdGjN.svg)](https://asciinema.org/a/acFWWlMKeCvpdGjN)

</div>

---

You're mid-workflow. You half-remember what `-rn` does in `cp`. You could Google it, wade through man pages, or just... ask.

```
$ inspect cp -rn src/ dest/
──────────────────────────────────────
 Command: cp -rn src/ dest/

  -r  copy directories recursively
  -n  will NOT overwrite existing files in dest/

 Net effect: safe incremental copy. New files added, nothing clobbered.
──────────────────────────────────────
```

No browser. No context switch. Back to work in 3 seconds.

---

## Features

- **Invocation explain** — paste any command, get a plain-English breakdown of every flag
- **Command lookup** — full docs: summary, usage, flags, examples, risk warnings
- **Interactive Q&A** — ask follow-up questions after lookup, powered by local LLM (Ollama)
- **Shell widget** — press `Ctrl+]` on any half-typed command to explain it inline
- **Git-aware** — understands `git commit`, `git push --force-with-lease`, etc. as subcommands
- **Works offline** — curated knowledge base + man pages + `--help`, no network needed
- **LLM enrichment** — local Ollama integration for obscure commands and cached responses

---

## Demo

```
$ inspect git push --force-with-lease origin main
──────────────────────────────────────
 Command: git push --force-with-lease origin main

  --force-with-lease  force push only if nobody else has pushed (safer than --force)
  origin              the remote to push to
  main                the branch to push

 Net effect: rewrites remote history safely — aborts if someone else pushed since your last fetch.

 RISKS:
  caution    Still rewrites history. Coordinate with your team first.
──────────────────────────────────────
```

```
$ inspect tar
──────────────────────────────────────
 tar — store and extract files from an archive

 USAGE: tar [OPTION...] [FILE]...

 EXAMPLES:
  tar -czf archive.tar.gz dir/     create compressed archive
  tar -xzf archive.tar.gz          extract compressed archive
  tar -tzf archive.tar.gz          list contents without extracting

 IMPORTANT FLAGS:
  -c, --create     create a new archive
  -x, --extract    extract files from an archive
  -z, --gzip       filter through gzip
  -f, --file       use archive file
  -v, --verbose    list files processed
  ...
──────────────────────────────────────

  ask anything (enter to exit): what's the difference between -z and -j?
  -z uses gzip compression (.tar.gz), -j uses bzip2 (.tar.bz2). bzip2 compresses
  better but is slower. For most uses, gzip is fine.
```

---

## Install

### From source (requires [Rust](https://rustup.rs))

```bash
git clone https://github.com/codeprakhar25/inspect.git
cd inspect
cargo build --release

# Put on PATH (pick one)
cp target/release/inspect ~/.local/bin/
sudo cp target/release/inspect /usr/local/bin/
```

---

## Shell widget — Ctrl+]

The best part. Press **Ctrl+]** on any command you're typing to explain it inline — without losing your prompt.

```bash
inspect --install-shell bash   # or zsh
source ~/.bashrc               # reload
```

Type something, press Ctrl+]:

```
$ rsync -avz --progress src/ dest/     ← press Ctrl+]

  -a  archive mode (preserves permissions, timestamps, symlinks)
  -v  verbose
  -z  compress during transfer
  --progress  show per-file progress bar

$ rsync -avz --progress src/ dest/     ← cursor back, unchanged
```

---

## LLM enrichment (optional)

Install [Ollama](https://ollama.com), pull a model, and `inspect` will enrich docs and power the interactive Q&A prompt:

```bash
ollama pull llama3.2

inspect --llm some-obscure-tool
inspect --llm --llm-model qwen2.5 curl
```

Responses cache at `~/.cache/inspect-cli/ollama/` — repeated lookups are instant.

Without Ollama, `inspect` works fine using man pages, `--help`, and the built-in curated database.

**Environment variables:**

| Variable | Default |
|---|---|
| `INSPECT_OLLAMA_MODEL` | `llama3.2` |
| `INSPECT_OLLAMA_URL` | `http://127.0.0.1:11434` |

---

## All options

```
inspect [OPTIONS] <command>
inspect [OPTIONS] <command> [flags...] [args...]

MODES:
    inspect cp                     lookup: show docs for 'cp'
    inspect cp -rn src/ dest/      explain: resolve each flag in the invocation
    inspect git commit -m "fix"    explain git subcommands

OPTIONS:
    --json                emit machine-readable JSON
    --allow-help          run <command> --help even for unknown commands
    --no-help             skip all <command> --help execution
    --llm                 enrich output via local Ollama instance
    --llm-model MODEL     Ollama model (default: env INSPECT_OLLAMA_MODEL or llama3.2)
    --llm-url URL         Ollama base URL (default: env INSPECT_OLLAMA_URL or http://127.0.0.1:11434)
    -v, --verbose         show source/fallback warnings
    --install-shell SHELL append shell widget to ~/.bashrc or ~/.zshrc (bash or zsh)
    -h, --help            show this help
    -V, --version         print version
```

---

## How docs are assembled

Sources are tried in priority order:

| Priority | Source | Notes |
|---|---|---|
| 1 | Curated knowledge base | Hand-written entries for 40+ common commands |
| 2 | `man` pages | Parsed and cleaned — strips groff, encoding artifacts |
| 3 | `--help` output | Flag and usage extraction |
| 4 | Ollama LLM | For obscure commands; cached locally |

---

## Contributing

Contributions welcome — especially curated entries. A curated entry gives cleaner output than any man page and takes ~10 minutes to write.

Entries live in [`src/curated.rs`](src/curated.rs). Existing entries are the best format reference.

```bash
cargo test      # run tests
cargo clippy    # lint
cargo fmt       # format
```

Open an issue first for anything beyond a curated entry addition or bug fix.

---

## License

MIT — see [LICENSE](LICENSE).
