# .stet.toml

Each knowledge repo has a `.stet.toml` at its root. It holds only what that repo does
differently from the shared setup; everything else is built into stet. stet writes the
tool configs from it into its cache on every run. `stet sync` writes the few files a repo
keeps, and `stet check` fails when one differs.

```toml
# Top-level keys come before any [table].
ci = true        # write .github/workflows/static-analysis.yml (default true)
publish = true   # `stet publish` may copy notes from this repo (default true)
never_publish = ["HomeAutomation/"]   # paths publish refuses even so

[[ignore]]
paths = ["Journal/Diary/2*.md", "Writing/pieces/"]
tools = ["prettier", "markdownlint", "rules"]   # default: all four
reason = """
Why these are skipped. Written as a comment beside the paths in the tool configs."""

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

## The files in the repo

- `.cspell.yaml`: the repo's words, and only them. stet makes it once and never rewrites
  it; stet's cspell settings import it, and editors read it as it is. Add words there.
- `.mise.toml`: stet's down to the marker line
  `# ---- Below this line: this repo's own tasks. …`. It pins stet, at the version that
  last ran `stet sync`, and hk. Add the repo's own tasks below it; a task that needs a
  tool names it with `tools = { node = "24" }`.
- `hk.pkl`: all stet's. The pre-commit hook: `stet check --check-only --files` on the
  staged Markdown files. Turn it on once per clone with `hk install --mise`.
- `.github/workflows/static-analysis.yml` when `ci` is true: installs stet with mise and
  runs `stet check --check-only`.

Prettier, cspell and markdownlint keep no config in the repo. An older stet wrote
`.prettierrc.yaml`, `.prettierignore` and `.markdownlint-cli2.yaml`; `stet sync` removes
them.

After editing `.stet.toml`, run `stet sync`, then `mise run check`, and commit both.
