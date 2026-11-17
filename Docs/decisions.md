# Decisions

What was decided about stet and why, newest first, one dated section each. Add new
decisions here. [design.md](design.md) describes how stet works now. The idea's history
before the repo is the Workshop note
[`Ideas/2026/stet.md`](https://github.com/gshaw/Workshop/blob/main/Ideas/2026/stet.md).

## 2026-11-17 · markdownlint skips what Prettier and cspell skip

The first code repos showed markdownlint reading what the other two tools skip: am's
`worker/node_modules` (about 1,400 findings, because `#node_modules` only matched at the
root) and sarduty's gitignored `deps/` and `_build/`. Ignores now become the same
any-depth globs Prettier gets, and markdownlint's config sets `gitignore: true`. The six
knowledge repos read the same files as before.

## 2026-11-17 · stet checks itself

stet is now a `kind = "code"` repo like the others: hooks through hk with stet's
generated `hk.pkl`, check-only, from the global install in dotfiles
([hk.md](https://github.com/gshaw/Workshop/blob/main/Tooling/hk.md)). stet owns its
Markdown and spelling; rustfmt and clippy are the repo's own steps. Why: an agent pushed
failing commits on 2026-11-17, and Gerry wants the same hooks everywhere.

- **The hook runs the released stet** (`"github:gshaw/stet" = "0.2.0"` in `mise.toml`),
  not the working copy, so a broken build can't block its own fix.
- **`tests/fixtures/` and `tests/snapshots/` are ignored:** the fixtures are broken on
  purpose, the snapshots are generated.
- **cspell had never read stet's code.** About 30 words went into `.cspell.yaml`; regex
  fragments and test strings with deliberate typos get an inline `cspell:ignore`.
- **CI** (`static-analysis.yml`) runs `cargo fmt --check`, clippy, `cargo test` and
  `stet check --check-only` on every PR and push to main. Without it, the
  `no-early-merges` guard let stet PRs through 10 minutes after the push, on the
  assumption that a repo with no checks has no CI.
- **CI also builds the musl release target** (`release-build`), so a broken
  cross-compile shows up on the PR, not on the tag. Linux only: the repo is public, so
  minutes are free, and `mise run publish` already builds the Mac target. A new push to
  a PR cancels its older run.

## 2026-11-17 · The same hooks in every repo, and code repos

Gerry's goal: consistent hooks on everything, knowledge repos and code repos (am, stet,
sarduty, Tides, LandNav; not MarNav; rota is archived).

- **One global hook.** `hk install --global --mise` in dotfiles' `gitconfig` replaces the
  per-clone installs. Every clone and worktree is covered, and a repo with no `hk.pkl` is
  a no-op. The local entries were removed from the repos that had them.
- **Check-only, no stash, everywhere.** The Rust repos' hand-written `hk.pkl` ran hk's
  defaults: fix and stage, and a git stash that takes untracked files. In a shared
  checkout that pulls another session's edits off disk. A failed hook costs one
  `hk fix --all` and a re-stage.
- **stet writes every `hk.pkl`;** a repo's own steps go below a marker line, like
  `.mise.toml`'s tasks. One Pkl file, so `hk check` and `hk fix` see every step. Tested
  on hk 2.12.0: a `local` property after the `steps` block that spreads it is valid.
- **The stet step takes every staged file,** not just Markdown: cspell reads code too.
- **`kind = "code"`.** stet checks a code repo's Markdown and spelling, and writes only
  `.cspell.yaml` and `hk.pkl`. The repo's `mise.toml` and CI are its own, so stet can't
  move the pins; `stet check` reports them when they're behind. Measured on copies of
  Tides, LandNav, sarduty, am and stet: 1 to 4 seconds a run, findings real (unformatted
  docs, an issue template's frontmatter, now allowed in `.github/`). am and stet never
  ran cspell and need word lists.
- **Tests stay in CI.** Builds, `mix compile --force` and test suites never go in
  pre-commit.
- **Agents can't skip the hook.** `no-hook-skips` in dotfiles is a PreToolUse hook for
  Claude Code and Codex.

## 2026-11-17 · stet owns the tools; repos pin only stet

Every repo pinned Node, Prettier, cspell and markdownlint-cli2 and carried their configs,
because CI couldn't install stet. Gerry wants a repo, knowledge or code, to depend on stet
alone and get a known-good set of linters. Now:

- **stet is released on GitHub** for macOS arm64 and Linux x64 (`.github/workflows/release.yml`,
  on a `v*` tag). Repos pin `"github:gshaw/stet" = "<version>"` and hk in `.mise.toml`; CI
  runs `stet check --check-only` like anywhere else.
- **stet pins the tools, mise installs them.** `mise exec node@… npm:prettier@… -- prettier`
  at the versions in `generate.rs`, whatever the repo pins. mise keeps one copy per version
  in its shared install folder, so twenty repos on one stet share one set. Upgrading a
  linter is a stet release, and each repo moves when its pin does.
- **The tool configs left the repos.** stet writes them to its cache on every run and
  passes them (`-c`, `--config`); Prettier's settings and every ignore go on the command
  line. `stet sync` removes the old generated copies, only ones that start with stet's
  header.
- **`.cspell.yaml` stays, and is the repo's.** It holds only the words. stet's cspell
  config imports it, and VS Code's spell checker reads it directly, so "add to workspace
  dictionary" lands in the one file stet uses. No copy to keep in sync. The editor doesn't
  see stet's ignores or its markdownlint rules; Gerry rarely uses VS Code and chose a
  simpler repo over a matching editor.
- **Same output, measured.** On copies of all six repos with planted errors (in notes, in
  `Inbox/` at the root and deeper, in `.build/`, in the journal and Goodreads paths Life
  ignores, in Arniston's `Untracked/`), the old and new `stet check --check-only` gave the
  same findings and the same file counts. Arniston's `index-check` needs `node`, which it
  got from the shared pin; it now declares it on the task.
- **cspell 10.4.0, and no `language` from stet** (0.1.1, same day). Run on copies of the
  code repos, 9.1.5 flagged sarduty's CSS data URIs, which 10.4.0 passes. stet's
  `language: en` also overrode sarduty's `en,en-GB`. 10.4.0 found the same issues as
  9.1.5 in all six knowledge repos; it only skips 17 binary spreadsheets in Finances.
  cspell defaults to `en`, so a repo sets another language in its own `.cspell.yaml`.
- **`use_versions_host = false` in each repo's `.mise.toml`** (0.1.2). mise looks up a
  `github:` release through its version service first, and that service 404s for
  gshaw/stet (it has hk), so a plain install failed. Asking GitHub directly works; CI
  passes mise-action's token, so rate limits don't bite.
- **Sibling-repo links are skipped in CI** (0.1.3). CI used to run only the three tools;
  now it runs all of `stet check`, and Life's links to `../Arniston/` failed on a runner
  that has Life alone. Those links are still checked on the Mac, where the repos sit side
  by side.

Looked at and not taken, measured on the same copies:

- **dprint's Markdown plugin** for Prettier: it changed 60 files. It turned
  `~30% withheld (~$20K)` into strikethrough, joined link text written across lines (breaks
  `proseWrap: preserve`), and realigned 660 table rows by measuring emoji differently.
- **rumdl** for markdownlint and Prettier: it matched markdownlint's findings (plus one rule
  of its own, MD076, to turn off) and, with styles set, made no changes to any repo. A good
  later swap inside stet, invisible to the repos. Not now: one change at a time.
- **typos** for cspell: it knows a list of common misspellings, so it caught 10 of 22
  realistic typos where cspell caught 22, and flagged about 100 false positives in
  Workshop. A dictionary-only checker without cspell's software and names dictionaries
  would add 247 words to Workshop's list. cspell stays.
- **One bundled binary** (`bun build --compile` of all three): 90 MB, and only markdownlint
  ran outside `node_modules`. Prettier and cspell load files at run time.

## 2026-11-17 · public, and a pre-commit hook through hk

- **stet is public.** Its history was scanned first: no secrets (one fake AWS key in a
  publish test), and the commit email was already public on other repos. Going public is
  the first step to CI installing stet instead of running the tools on copied configs.
- **The knowledge repos get a pre-commit hook, run by hk.** Twice on 2026-11-17 an agent
  pushed Workshop commits while `mise run check` failed: it piped the check into `grep`,
  so the chain saw grep's exit status. A rule in memory relies on the agent remembering;
  a hook doesn't. Gerry chose hk for every hook, in these repos and the code repos.
- **Pre-commit, not pre-push.** Arniston, Family and Finances have no remote, so a
  pre-push hook would never run there, and a bad commit on `main` is already the problem.
- **Staged files only, by filtering findings.** `stet check` read every file, untracked
  ones included, so another session's draft blocked every commit. `--files` keeps only
  findings in the named files. Filtering, not passing the files to each tool, keeps every
  tool's ignore rules exactly as they are, and the run is short enough (about 3 s).
- **hk lists the files; stet still never runs git.** No stash and no fixing in the hook:
  sessions share a working tree, and a stash would pull their edits off disk.
- **gh-signoff was looked at and dropped.** It posts a green commit status after checking
  the tree is clean and pushed; it runs no check. Its ruleset needs a remote, GitHub Pro,
  and binds no admin, and agents use Gerry's admin login.

## 2026-10-04 · inbox, init and publish

- **`stet inbox` reads the mail rules from AGENTS.md** rather than moving them into
  config. They're prose people and agents edit; stet parses the section's bullets, its
  "Not here:" paragraph and its notes, and reports a repo without one.
- **`stet init` stops at `git init`.** The GitHub repo, the config entry and the first
  commit are printed as next steps. What only a person knows is a ❌ line in the
  templates, so a new repo passes `stet check` and an agent can see what's missing.
- **`stet publish` refuses more than the brief named.** Health and Finances have
  `publish = false` as asked; so does Arniston, since its notes are the Company's and
  name third parties, which seemed the safer default (flip it in its `.stet.toml`).
  Workshop's `HomeAutomation/` is under a new `never_publish` key: AGENTS.md says it
  holds garage fob IDs and access plans that need review before anything goes public,
  and the fob note wasn't flagged, so a flag-only check would have let it through.
- **The secret scan has no override.** A false alarm is fixed by editing the note. One
  was found on real notes and fixed in the scanner: an environment variable's name
  (`SOPS_AGE_KEY`) after "secret:".
- **Private links are found from git remotes,** not a hard-coded list, so stet holds no
  repo names. A link to a private repo that isn't configured, like a code repo, is kept
  ⚠️; Gerry's read of the draft is the check for those.

## 2026-10-03 · Why stet exists, and how it starts

Five knowledge repos each carried their own copy of the Prettier, cspell and markdownlint
setup: `.mise.toml`, `.prettierrc.yaml`, `.prettierignore`, `.markdownlint-cli2.yaml`,
`.cspell.yaml` and a CI workflow. By 2026-10-03 the copies had drifted only in comments
and in each repo's ignores, and the house rules in five `AGENTS.md` files (no frontmatter,
dated superseded blocks, links that work) had nothing checking them. Gerry approved the
build on 2026-10-03.

- **Rust, built like Rota.** Same toolchain, same conventions: clap, miette, `--json` on
  every command with `"schema": 1`, `stet doctor`, `mise run publish`, a signed build. A
  second tool in the family shouldn't be a second way of building tools.
- **Shell out to Prettier, cspell and markdownlint-cli2,** through `mise exec` in each
  repo, so the versions the repo pins are the ones used. They're what works today; Rust
  equivalents would change what passes.
- **stet holds the canonical config and writes it into each repo.** CI on GitHub can't
  install a private binary, so the repos keep committed configs and CI keeps running the
  tools check-only on them. `stet sync` writes them from `.stet.toml`; `stet check` fails
  when they differ. The alternative, configs that only exist when stet runs, would have
  left CI checking nothing.
- **`.stet.toml` holds only what differs:** extra ignores with their reasons and which
  tools they apply to, where frontmatter is allowed, cspell patterns, the repo's own check
  tasks, whether it has CI, whether `publish` may read it.
- **Two files are shared by both.** `.cspell.yaml` from `words:` on is the repo's
  vocabulary and `.mise.toml` below a marker line is the repo's own tasks; stet writes
  the rest and keeps those parts as they are. A shared base word list was left out: the
  lists overlap little, and cspell's `import` would have been a second file per repo to
  explain.
- **`mise run check` stays the documented command** and becomes a call to `stet check`,
  so the AGENTS.md files and the nightly Rota task keep working. Without stet installed
  it falls back to the tools alone.
- **The tools run one after another,** Prettier first. Run in parallel, the linters read
  half-written files (Health's `.mise.toml` recorded phantom MD060 errors on 2026-08-07).
- **House rules, measured on all five repos first.** Links and anchors that don't resolve,
  frontmatter outside the allowed paths, and an undated superseded block are errors: each
  had only a handful of hits, all real. "Currently" with no date nearby is a warning: 39
  hits, most fine in context. Links into a sibling repo (`../Arniston/`) only have to
  exist: they're read side by side on the Mac.
