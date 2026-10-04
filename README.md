# stet: keep the knowledge repos in shape

`stet` is a small Rust CLI for Gerry's Markdown knowledge repos (Workshop, Life, Health,
Finances, Arniston). It does the mechanical parts of the pattern in Workshop's
`Meta/README.md`; judgment stays with the agents.

_Stet_ is the proofreader's mark for "let it stand", written in the margin to keep text
that was crossed out. That's the superseded-block rule: old text stays, marked and dated.
stet is one of the three Bunter tools, beside [Rota](https://github.com/gshaw/Rota) and `am`.

## Install

```sh
mise run publish
```

That checks, tests, builds, signs and installs `~/.local/bin/stet`. It signs with the
certificate in `ROTA_SIGN_IDENTITY`, Rota's, set once per Mac with `mise set -g`;
without it, stet installs unsigned with a warning.

## Checking a repo

```sh
mise run check            # in a knowledge repo: runs stet check
stet check                # the same: format, spell-check, lint, house rules
stet check --check-only   # change nothing, as CI does
stet check --warnings     # list the warnings too, not just the count
stet check --all          # every repo in ~/.config/stet/config.toml
stet sync                 # rewrite the tool configs from .stet.toml
stet doctor               # mise, Node and the tools run; every repo is in sync
```

Each repo's `.stet.toml` holds only what it does differently; `stet help config` has every
key, and `stet help rules` the house rules. Every command takes `--json`; the documents
and exit codes are in [Docs/json-contract.md](Docs/json-contract.md). How stet works is
in [Docs/design.md](Docs/design.md), and why in [Docs/decisions.md](Docs/decisions.md).

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

## Developing

`mise run test`, `mise run lint`, `mise run fmt`, and `mise run snapshots` to review changed
snapshots. hk runs rustfmt and clippy before every commit. `mise run publish` is the whole
release: no tags, no releases. Agents working on this repo start with
[AGENTS.md](AGENTS.md).
