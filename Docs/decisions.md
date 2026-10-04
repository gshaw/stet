# Decisions

What was decided about stet and why, newest first, one dated section each. Add new
decisions here. [design.md](design.md) describes how stet works now. The idea's history
before the repo is the Workshop note
[`Ideas/2026/stet.md`](https://github.com/gshaw/Workshop/blob/main/Ideas/2026/stet.md).

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
