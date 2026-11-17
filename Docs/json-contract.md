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
        { "path": ".mise.toml", "action": "written" },
        { "path": "hk.pkl", "action": "unchanged" },
        { "path": ".prettierrc.yaml", "action": "removed" }
      ]
    }
  ]
}
```

`action` is `written`, `unchanged`, or `removed` for a tool config an older stet generated
into the repo. Exit 0.

## `stet doctor --json`

```json
{
  "schema": 1,
  "ok": true,
  "checks": [
    { "id": "config", "status": "ok", "message": "~/.config/stet/config.toml: 5 repos" },
    { "id": "mise", "status": "ok", "message": "/Users/gerry/.local/bin/mise" },
    { "id": "node", "status": "ok", "message": "v24.4.1" },
    { "id": "prettier", "status": "ok", "message": "3.8.3" },
    { "id": "cspell", "status": "ok", "message": "9.1.5" },
    { "id": "markdownlint-cli2", "status": "ok", "message": "v0.22.1 (markdownlint v0.40.0)" },
    { "id": "repo:Workshop", "status": "ok", "message": "~/Documents/Workshop: configs in sync" },
    { "id": "publish", "status": "ok", "message": "articles go to ~/Developer/gshaw.ca/content/articles" }
  ]
}
```

- `status`: `ok`, `warning` or `error`. `ok` is false, and the exit code 1, when any check
  is an error. A missing config file or `[publish]` is only a warning.
- The tool versions come from running each one through `mise exec` at the versions stet
  pins.
- One `repo:<name>` check per configured repo: the folder exists, `.stet.toml` reads, and
  the generated configs match it.

## `stet inbox --json`

What's waiting in each repo's `Inbox/`, and the repo's "Mail that belongs here" section
from its `AGENTS.md`. The repos are the ones named, or every repo in the config file.
Exit 0 whatever is waiting.

```json
{
  "schema": 1,
  "waiting": 1,
  "repos": [
    {
      "name": "Health",
      "path": "/Users/gerry/Documents/Health",
      "inbox": "/Users/gerry/Documents/Health/Inbox",
      "items": [
        {
          "name": "lab results.pdf",
          "path": "/Users/gerry/Documents/Health/Inbox/lab results.pdf",
          "kind": "file",
          "bytes": 182340,
          "modified": "2026-10-03T22:14:05Z"
        }
      ],
      "mail": {
        "belongs": [
          "Gerry's and Jesse's medical records, lab and imaging results, referrals, prescriptions, appointments, and clinic or pharmacy receipts.",
          "Fitness: training, gym, and coaching mail, and nutrition."
        ],
        "notes": [
          "A question email about an item from this repo names only the sender, subject and date. No results and no conditions."
        ]
      }
    }
  ]
}
```

- `items`: everything in `Inbox/` except its own `README.md`, `.gitignore`, `_processed/`
  and dotfiles, sorted by name. `kind` is `file` or `folder`; a folder's `bytes` is
  everything inside it. A repo with no `Inbox/` folder has no items.
- `mail.belongs`: the section's top-level bullets, each on one line, Markdown kept.
- `mail.notHere`: the paragraph that starts "Not here:", without the label. Absent when
  there's none.
- `mail.notes`: any other paragraph after the bullets, like how to word a question email.
  The paragraph before the bullets describes the nightly run and is left out.
- `mail` is absent, and `problem` says why, when `AGENTS.md` is missing or has no
  "Mail that belongs here" heading. `problem` also reports an `Inbox/` stet couldn't read.

## `stet init <dir> --json`

```json
{
  "schema": 1,
  "path": "/Users/gerry/Documents/Music",
  "name": "Music",
  "created": [".cspell.yaml", ".git/", ".gitignore", "AGENTS.md", "CLAUDE.md", "Inbox/", "README.md"],
  "next": ["Fill in the ❌ lines in README.md and AGENTS.md.", "Create the private repo on GitHub and add it as origin."]
}
```

- `created`: every file and folder written, relative to `path`, sorted. `.git/` appears
  when stet ran `git init`; a folder that already had an empty `.git` keeps it.
- `next`: what's left to do by hand, in order.
- Exit 1, with nothing written, when `dir` holds anything but `.git` and `.DS_Store`.

## `stet publish <note> --json`

Writes the note into the site's articles folder (`[publish]` in the config file) as a
Hugo draft, and prints:

```json
{
  "schema": 1,
  "source": "/Users/gerry/Documents/Workshop/Web/web-app-recovery.md",
  "target": "/Users/gerry/Developer/gshaw.ca/content/articles/web-app-recovery.md",
  "title": "Web app backup and recovery",
  "slug": "web-app-recovery",
  "links": [{ "line": 136, "url": "../Tooling/healthchecks.md", "action": "unlinked" }]
}
```

- `links`: every link changed, by line in the source note. `unlinked`: the link became its
  text. `removed`: an image, autolink or reference definition pointing into a private repo
  was dropped.
- A refusal exits 1 with an error document whose `reasons` list every problem, by line,
  never quoting the secret:

```json
{
  "schema": 1,
  "error": "refused to publish ~/Documents/Workshop/Ideas/2026/stet.md",
  "reasons": ["line 39: flagged ⚠️ publish-review"]
}
```

- An existing article of the same name is an error unless `--force`.
