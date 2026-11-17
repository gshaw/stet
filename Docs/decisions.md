# Decisions

What was decided about stet and why, newest first, one dated section each. Add new
decisions here. [design.md](design.md) describes how stet works now. The idea's history
before the repo is the Workshop note
[`Ideas/2026/stet.md`](https://github.com/gshaw/Workshop/blob/main/Ideas/2026/stet.md).

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
