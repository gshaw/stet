# The house rules

`stet check` runs these after Prettier, cspell and markdownlint. They come from the
knowledge repos' AGENTS.md: rules no linter knows. They skip `Inbox/` and every path a
repo's `.stet.toml` ignores for `rules`.

## Errors: `stet check` exits 1

- **`config-sync`**: a generated file differs from what `.stet.toml` makes, or a tool
  config an older stet generated is still there. Run `stet sync` and commit the result.
- **`frontmatter`**: a Markdown file starts with a YAML frontmatter block outside the
  allowed paths. These repos are plain Markdown. `AgentSkills/` and `.claude/` are
  allowed everywhere, since skills need frontmatter; a repo adds more under
  `[frontmatter] allow`.
- **`link`**: a relative link or image whose file doesn't exist, or exists with a
  different case. The Mac doesn't care about case and GitHub does, so `Notes.md` for
  `notes.md` works in iA Writer and breaks on GitHub. A link into a sibling repo
  (`../Arniston/`) only has to exist, and is skipped in CI (`CI` set), where the sibling
  isn't checked out.
- **`anchor`**: a link's `#anchor` names no heading in its target. Anchors are GitHub's:
  lowercase, punctuation dropped, spaces to hyphens, `-1` and `-2` for repeats.
- **`superseded-date`**: a block that starts "Superseded" (after any ✅, ❌, ⚠️, `>` or
  `**`) and carries no date. The house style is a dated block on top of the old text:
  `**Superseded 2026-09-28:** …`. A date in the block, in its heading, or in the block
  just before it counts. A table cell or a heading that says only "Superseded" is a
  label and doesn't count.

## Warnings: reported, never fail

- **`undated-now`**: "currently", "right now", "at the moment", "at present", "these
  days", "nowadays", "presently" or "as of now" with no date in the same block, its
  heading, or the block before. "Currently" without a date is a future trap. Many are
  fine in context, so this only warns. `stet check --warnings` lists them.

A date is any of `2026-10-03`, `2026-10`, `October 3rd, 2026`, `3 Oct`, `Aug 2026`, or a
year from 1990 on.
