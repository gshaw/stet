# stet: keep the knowledge repos in shape

`stet` is a small Rust CLI for Gerry's Markdown knowledge repos (Workshop, Life, Health,
Finances, Arniston). It does the mechanical parts of the pattern in Workshop's
`Meta/README.md`; judgment stays with the agents.

_Stet_ is the proofreader's mark for "let it stand", written in the margin to keep text
that was crossed out. That's the superseded-block rule: old text stays, marked and dated.
stet is one of the three Bunter tools, beside [rota](https://github.com/gshaw/rota) and `am`.

## Install

A repo pins stet in `.mise.toml`, and mise installs it from the GitHub releases:

```toml
[tools]
"github:gshaw/stet" = "0.1.0"
```

stet brings its own Prettier, cspell and markdownlint-cli2: it runs them through
`mise exec` at the versions it pins, so mise installs each once for every repo. A repo
pins nothing else for its Markdown.

For Gerry's own build, `mise run publish` checks, tests, builds, signs and installs
`~/.local/bin/stet`. It signs with the certificate in `ROTA_SIGN_IDENTITY`, rota's, set
once per Mac with `mise set -g`; without it, stet installs unsigned with a warning.

## Checking a repo

```sh
mise run check            # in a knowledge repo: runs stet check
stet check                # the same: format, spell-check, lint, house rules
stet check --check-only   # change nothing, as CI does
stet check --warnings     # list the warnings too, not just the count
stet check --all          # every repo in ~/.config/stet/config.toml
stet check --check-only --files a.md b.md   # only findings in these, as the hook does
stet sync                 # rewrite .mise.toml, hk.pkl and CI from .stet.toml
stet doctor               # mise, Node and the tools run; every repo is in sync
```

Each repo's `.stet.toml` holds only what it does differently; `stet help config` has every
key, and `stet help rules` the house rules. Every command takes `--json`; the documents
and exit codes are in [Docs/json-contract.md](Docs/json-contract.md). How stet works is
in [Docs/design.md](Docs/design.md), and why in [Docs/decisions.md](Docs/decisions.md).

## The pre-commit hook

Every repo stet looks after, knowledge or code, has a generated `hk.pkl`: before each
commit, [hk](https://hk.jdx.dev) runs `stet check --check-only --files` on the staged
files, check-only and without a stash. Another session's untracked note can't block a
commit; an unknown word in a staged file always does. A code repo adds its own steps
(rustfmt, SwiftLint) below the marker line at the end. Turn hk on once per Mac:

```sh
hk install --global --mise
```

`stet doctor` fails a repo without it.

## Starting a repo

```sh
stet init ~/Documents/Music
```

The bootstrap from Workshop's `Meta/README.md` as one command: `git init`, `.stet.toml`
and the generated configs, `.gitignore`, a `README.md` and an `AGENTS.md` with the
universal rules and ❌ where only a person can fill in, the `CLAUDE.md` symlink, and an
untracked `Inbox/`. The new repo passes `stet check` as it stands. It prints what's left:
the GitHub repo, the config entry, the first content.

## The inbox

```sh
stet inbox          # what's waiting in every repo's Inbox/
stet inbox --mail   # and where each repo says mail goes
stet inbox --json   # both, for the nightly mail run
```

The repos come from `~/.config/stet/config.toml` (`repos = [...]`). The mail rules are
each repo's "Mail that belongs here" section in its `AGENTS.md`, read as data: the
bullets, the "Not here:" line, and any notes. The prose stays the source.

## Publishing a note

```sh
stet publish ~/Documents/Workshop/Web/web-app-recovery.md
```

Copies one note into the site repo from `[publish]` in the config file (gshaw.ca's
`content/articles/`) as a Hugo draft: front matter with the title, today's date and
`draft: true`. It refuses a note from a repo with `publish = false` (Health, Finances,
Arniston), under a repo's `never_publish` paths (Workshop's `HomeAutomation/`), flagged
⚠️ publish-review, or that looks like it holds a key, token, password, account or card
number. Links into the private repos become plain text and their images are dropped. It
prints the new file's path and never commits, pushes or deploys: Gerry reads it first.

## Developing

`mise run test`, `mise run lint`, `mise run fmt`, and `mise run snapshots` to review changed
snapshots. hk runs rustfmt and clippy before every commit. To release, bump `version` in
`Cargo.toml`, merge, and push a matching tag (`v0.2.0`): the release workflow builds the
Mac and Linux binaries and attaches them to a GitHub release. Agents working on this repo start with
[AGENTS.md](AGENTS.md).
