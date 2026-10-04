# The `stet --json` contract

Every `--json` document `stet` prints, and every exit code. Agents and scripts read this.
The Rust types are in [`src/contract.rs`](../src/contract.rs), and `tests/snapshots` holds
a real example of most documents. Change the types, this file and the snapshots in the
same commit.

## Rules

- Every document is one JSON object with `"schema": 1`. The number changes only on a
  breaking change: a key removed or renamed, or a meaning changed. New keys can appear
  without a bump, so ignore keys you don't know.
- Keys are camelCase. Dates are ISO 8601 in UTC, to the second: `2026-10-04T14:00:00Z`.
- A key whose value is unknown is left out, not `null`.
- Paths inside a repo are relative to its root. Repo paths are absolute.
- Human output (without `--json`) is for people and may change at any time.

## Exit codes

| Code | Meaning                                                                       |
| ---- | ----------------------------------------------------------------------------- |
| 0    | Success. For `check`, no errors (warnings don't count).                       |
| 1    | Failure: `check` found an error, or a command couldn't do its job.            |
| 64   | Usage error: an unknown command or flag, or a missing argument.               |

When a command fails before it can build its document and `--json` was given, it prints an
error document instead, on stdout:

```json
{ "schema": 1, "error": "no .stet.toml in /Users/gerry/Desktop or any folder above it" }
```

## Choosing repos

`check` and `sync` take repo paths, or any path inside a repo (the nearest folder at or
above it with a `.stet.toml`). With none, the current folder's repo. `--all` means every
repo in `~/.config/stet/config.toml`.

## `stet check --json`

```json
{
  "schema": 1,
  "ok": false,
  "errors": 1,
  "warnings": 1,
  "repos": [
    {
      "name": "Workshop",
      "path": "/Users/gerry/Documents/Workshop",
      "ok": false,
      "errors": 1,
      "warnings": 1,
      "formatted": ["Ideas/2026/stet.md"],
      "tools": [
        { "name": "prettier", "ok": true },
        { "name": "cspell", "ok": true, "files": 160 },
        { "name": "markdownlint", "ok": true, "files": 149 }
      ],
      "findings": [
        {
          "file": "Web/web-app-recovery.md",
          "line": 41,
          "tool": "stet",
          "rule": "superseded-date",
          "severity": "error",
          "message": "a superseded block needs the date it was superseded"
        },
        {
          "file": "AGENTS.md",
          "line": 70,
          "tool": "stet",
          "rule": "undated-now",
          "severity": "warning",
          "message": "\"currently\" with no date nearby"
        }
      ]
    }
  ]
}
```

- `formatted`: files Prettier rewrote. Always empty with `--check-only`.
- `tools`: each tool that ran, in order; absent with `--rules-only`. `ok` is false when it
  reported anything. `files` is how many files it read, when it says. A repo's own check
  task appears as `mise:<task>`.
- `findings`: errors first, then warnings, each by file and line. `file`, `line` and
  `column` are absent when unknown, like a tool that wouldn't start.
- `tool`: `stet`, `prettier`, `cspell`, `markdownlint` or `mise`.
- `rule`:
  - `stet`: `config-sync`, `frontmatter`, `link`, `anchor`, `superseded-date`,
    `undated-now` (`stet help rules`)
  - `prettier`: `format` (not formatted, with `--check-only`) or `error`
  - `cspell`: `unknown-word`
  - `markdownlint`: the rule, like `MD022/blanks-around-headings`
  - `mise`: the task's name
  - any tool: `run`, when it wouldn't start or failed in a way stet couldn't read
- `severity`: `error` or `warning`. Only `undated-now` is a warning.

## `stet sync --json`

```json
{
  "schema": 1,
  "repos": [
    {
      "name": "Life",
      "path": "/Users/gerry/Documents/Life",
      "files": [
        { "path": ".prettierrc.yaml", "action": "unchanged" },
        { "path": ".markdownlint-cli2.yaml", "action": "written" }
      ]
    }
  ]
}
```

`action` is `written` or `unchanged`. Exit 0.
