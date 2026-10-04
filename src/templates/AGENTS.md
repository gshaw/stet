# Working in this repo (for agents)

`{name}` is a Markdown knowledge base: plain files in git, maintained by agents, read by a
person skimming later. ❌ Say what it covers and who owns it.

## The Inbox — "run inbox"

`Inbox/` is untracked scratch space. The owner drops captures in and says **"run
inbox"**; an agent works through it item by item. Each item is **absorbed** into the
canonical domain file (dated, source cited, original deleted), **filed** beside the
Markdown that explains it (for records worth keeping as files), or **discarded** (say
what and why; never delete anything ambiguous without asking). Batch the questions; one
commit per batch. `Inbox/` is empty when the work is done.

## Mail that belongs here

The nightly inbox run reads this section to decide where each forwarded email goes
(`stet inbox` turns it into data). Anything that fits no repo, or more than one, gets
asked about rather than guessed.

- ❌ What forwarded mail belongs here, one bullet per kind.

Not here: ❌ what belongs in another repo instead.

## Sensitivity

- **Never record** — passwords, API keys, tokens, 2FA seeds, government identifiers, and
  other people's personal data beyond a first name without asking first.
- ❌ Anything else this repo must never hold.

**Git history is permanent.** If something from the never-list gets committed, the fix is
to rotate the secret, not to quietly redact.

## Universal rules

- **One canonical home per fact.** Other files link to it rather than restating it.
- **Never create empty scaffolding.** A domain or file is created when it has real
  content.
- **Let topics start small.** A new area begins as a single file and splits out only
  when it gets hard to navigate.
- **Newer decisions supersede older notes.** Keep the old text under a dated
  "superseded" block rather than silently deleting it.
- **Date things.** Notes carry the date they were true; a claim about the present with
  no date is a future trap.
- **Cite sources** — the chat, article, document or commit a note came from.
- **Mark anything unverified with ⚠️.** Status glyphs: ✅ done · ❌ todo · ⚠️ needs
  confirming.
- **Plain Markdown**, no YAML front matter. Links inline as `[label](url)`.

## Adding a new domain

1. Create the directory only when there is real content to put in it.
2. Add a `README.md` entry point once the domain has more than a couple of files.
3. Add an `AGENTS.md` only if the domain needs rules that differ from this file.
4. Add the domain to the root `README.md`.

## Markdown formatting

Run `mise run check` before committing. It runs `stet check`, which formats (Prettier),
spell-checks (cspell), lints (markdownlint) and applies the house rules. The tool configs
are generated from `.stet.toml`: change that and run `stet sync`, never the generated
files. New legitimate words go in `.cspell.yaml`'s `words:`; fix genuine typos instead.
**Work commits straight to `main`**; CI runs the check-only variant on every push. Prose
is never rewrapped (`proseWrap: preserve`).
