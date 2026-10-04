# stet: keep the knowledge repos in shape

`stet` is a small Rust CLI for Gerry's Markdown knowledge repos (Workshop, Life, Health,
Finances, Arniston). It does the mechanical parts of the pattern in Workshop's
`Meta/README.md`; judgment stays with the agents.

_Stet_ is the proofreader's mark for "let it stand", written in the margin to keep text
that was crossed out. That's the superseded-block rule: old text stays, marked and dated.
stet is one of the three Bunter tools, beside [Rota](https://github.com/gshaw/Rota) and `am`.

## Developing

`mise run test`, `mise run lint`, `mise run fmt`, and `mise run snapshots` to review changed
snapshots. hk runs rustfmt and clippy before every commit. `mise run publish` is the whole
release: no tags, no releases. Agents working on this repo start with
[AGENTS.md](AGENTS.md).
