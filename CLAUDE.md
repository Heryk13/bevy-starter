# CLAUDE.md

@CONTRIBUTING.md

## Before moving code

Before proposing to move or restructure code, grep `docs/flow.md` and `docs/specs/` for the
names involved: where code sits may be a recorded decision. Cite the doc that agrees, or
present both sides when one disagrees.

## Plans and specs

- A change bigger than one commit starts as a spec: `docs/specs/<feature>/design.md`, with a
  status line, the decisions, an implementation order of one commit per step, and a
  "What changed while building it" section filled in as the build departs from the plan.
- Once built, a spec is history: leave its text as written, and update only its status line.
  The living docs are `README.md`, `docs/flow.md` and `docs/template.md`.

## Every implementation

Whenever a change is made, by the user or by you, before calling it done:

1. Read the diff and check it against every rule in `CONTRIBUTING.md`:
   the loop, file layout, separators, comments, Bevy patterns, text, naming, tests and docs.
   Grep every renamed identifier in `README.md`, `docs/flow.md` and `docs/template.md`.
2. Run `just check` yourself. Never report it as passing without running it.
   Skip it for a docs-only change: there is nothing in it for `just check` to check.
3. Report a short checklist: what follows the conventions, what is missing,
   and a suggested commit message.

At the end of every step of a plan, before the commit, run that review unasked, with an
explicit pass over **every name** (constants, types, systems, helpers, parameters, tests) and
over the **order** of sections and systems. A rename leaves variables whose names now lie, and
the compiler never complains about those.

Say plainly what was and was not verified: `just check` covers compiling, lints and tests, and
it says nothing about how the game looks or sounds. Ask the user to run `just run` for that.

## Bevy version

This project uses Bevy 0.20, and its APIs change between versions.
Check signatures in the local source (`~/.cargo/registry/src/*/bevy_*-0.20.*`) instead of relying on memory.
