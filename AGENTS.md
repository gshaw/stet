# AGENTS.md

Start with [README.md](README.md). [Docs/design.md](Docs/design.md) is how stet works.
[Docs/decisions.md](Docs/decisions.md) is what was decided and why: add new decisions
there, dated, newest first. The idea's history before the repo is in Workshop at
`~/Documents/Workshop/Ideas/2026/stet.md`; don't add to it from here.

## The one rule: the CLI is the engine

- **`stet` is the product and its only door.** An agent, a rota task or a script gets
  every piece of data from `stet … --json` and changes the repos' configs only through
  `stet sync`. If a client needs something the CLI can't give it, add a command or a field.
- **`src/contract.rs` is data shapes only.** serde structs and enums for the `--json`
  output. No file access, no process calls.
- **Every `--json` document carries `"schema": 1`.** Changes are additive only. Bump the
  number on a breaking change: a key removed or renamed, or a meaning changed.
- **`Docs/json-contract.md` documents every command's JSON and exit codes.** Update it in
  the same commit as `contract.rs`, and accept the new snapshots with `mise run snapshots`.
- **`stet help <topic>` is the reference** for `.stet.toml` and the house rules, built
  from `Docs/help/*.md`. Change a topic in the same commit as the behaviour it describes.

## The engine

- **`src/generate.rs` is the one copy of the shared config.** The tool versions and
  configs live here and reach the tools through stet's cache, never the repos. A repo
  keeps only `.mise.toml` (pinning stet), `hk.pkl`, the CI workflow and its own
  `.cspell.yaml` words. A change here ships in a release; then `stet sync --all` moves
  each repo's pin, and each repo is committed.
- **stet never uses git, never commits, never pushes.** `sync` and `init` write files;
  `publish` writes one file into the site repo. `check` changes files only through
  Prettier, and not at all with `--check-only`. The pre-commit hook gets the staged files
  from hk (`--files`); `doctor` reads git config as text to see the hook is on.
- **Never touch `Inbox/` or the verbatim paths** a repo's `.stet.toml` ignores. No tool
  and no rule reads them.
- **Tests never touch the real repos or gshaw.ca.** They copy `tests/fixtures` into a
  temp folder, set `HOME` and `STET_CONFIG_DIR` there, and point `STET_PRETTIER`,
  `STET_CSPELL`, `STET_MARKDOWNLINT` and `STET_MISE` at the fakes in `tests/fixtures/bin`,
  so no test needs Node. `STET_TODAY` pins the date.
- **A new rule must pass on all five repos before it ships.** Run
  `stet check --all --rules-only` and look at every hit: fix what's really broken in the
  content, make a noisy rule a warning, never mass-edit prose to satisfy it.

## Style

- Rust 2024, pinned in `mise.toml`. clap derive, serde, miette, pulldown-cmark, the
  `ignore` crate for walking and gitignore-style patterns.
- Library code takes an `Env`, never the process environment, so every test controls it.
- Human output is short and plain. `--json` output is stable and complete.
- Run `mise run publish` before calling a change done: it checks, tests, builds, signs
  and installs to `~/.local/bin/stet`. hk runs rustfmt and clippy on every commit.
