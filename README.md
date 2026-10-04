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
```

Each repo's `.stet.toml` holds only what it does differently; `stet help config` has every
key, and `stet help rules` the house rules. Every command takes `--json`; the documents
and exit codes are in [Docs/json-contract.md](Docs/json-contract.md). How stet works is
in [Docs/design.md](Docs/design.md), and why in [Docs/decisions.md](Docs/decisions.md).

## Developing

`mise run test`, `mise run lint`, `mise run fmt`, and `mise run snapshots` to review changed
snapshots. hk runs rustfmt and clippy before every commit. `mise run publish` is the whole
release: no tags, no releases. Agents working on this repo start with
[AGENTS.md](AGENTS.md).
