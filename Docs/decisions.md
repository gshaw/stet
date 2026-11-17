# Decisions

What was decided about stet and why, newest first, one dated section each. Add new
decisions here. [design.md](design.md) describes how stet works now. The idea's history
before the repo is the Workshop note
[`Ideas/2026/stet.md`](https://github.com/gshaw/Workshop/blob/main/Ideas/2026/stet.md).

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
