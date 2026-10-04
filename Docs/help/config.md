# .stet.toml

Each knowledge repo has a `.stet.toml` at its root. It holds only what that repo does
differently from the shared setup; everything else is built into stet. `stet sync` writes
the tool configs from it, and `stet check` fails when a repo's copy differs.

```toml
# Top-level keys come before any [table].
ci = true        # write .github/workflows/static-analysis.yml (default true)
publish = true   # `stet publish` may copy notes from this repo (default true)
never_publish = ["HomeAutomation/"]   # paths publish refuses even so

[[ignore]]
paths = ["Journal/Diary/2*.md", "Writing/pieces/"]
tools = ["prettier", "markdownlint", "rules"]   # default: all four
reason = """
Why these are skipped. Written as a comment above the paths in every generated file."""

[frontmatter]
allow = ["Cooking/drafts/"]   # on top of AgentSkills/ and .claude/

[[cspell.ignore_regexps]]
pattern = '/\bZ[A-Z0-9_]{3,}\b/g'
reason = "Core Data's table names, quoted exactly."

[mise]
check_tasks = ["index-check"]   # the repo's own tasks that check and CI run too
```

## Paths

Patterns are gitignore-style, from the repo root. A trailing `/` means a folder and
everything in it: `Writing/pieces/`. A glob matches files: `Journal/Diary/2*.md`.

`tools` names who skips the paths: `prettier`, `markdownlint`, `cspell`, and `rules` for
stet's house rules. Leave it out to skip them everywhere.

Every repo also skips `Inbox/`, `node_modules/` and `**/.build/` everywhere, and cspell
skips binary files (PDFs, images, office documents).

## The generated files

- `.prettierrc.yaml`, `.prettierignore`, `.markdownlint-cli2.yaml`: all stet's.
- `.cspell.yaml`: stet's down to `words:`. From `words:` on, with the comment lines
  right above it, is the repo's own vocabulary. Add words there.
- `.mise.toml`: stet's down to the marker line
  `# ---- Below this line: this repo's own tasks. …`. Add the repo's own tasks below it.
- `.github/workflows/static-analysis.yml` when `ci` is true.

After editing `.stet.toml`, run `stet sync`, then `mise run check`, and commit both.
