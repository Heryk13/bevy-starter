# CLAUDE.md

@CONTRIBUTING.md

## Working with the user

- The user is learning Rust and Bevy. Guide step by step and let them write the code.
  Only edit files when they explicitly ask for it. Explain *why*, not just *what*.
- Reply in Portuguese (pt-BR). Code, comments, docs and commit messages stay in English.
- One small task per message, then stop and wait. A task that needs four changes is four
  messages, not a list. When they say they are lost, the fix is a smaller step or a table of
  what each piece does, never the finished code.
- Give the facts they cannot guess (type names, method names, which module something is in)
  and let them assemble the code. Hand over a line only when it combines APIs they have never
  seen, and say that is why. A compiler error is the next lesson: quote it and read it with
  them instead of fixing it for them.
- Claude writes the surrounding prose without being asked: `///` doc comments, the `// ====`
  and `// ----` separators at column 80, the comments that record an exception's reason, and
  lint attributes. The user writes the Rust.

## Every implementation

Whenever a change is made, by the user or by you, before calling it done:

1. Read the diff and check it against every rule in `CONTRIBUTING.md`:
   the loop, file layout, separators, comments, Bevy patterns, naming, tests and docs.
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

This project uses Bevy 0.19, and its APIs change between versions.
Check signatures in the local source (`~/.cargo/registry/src/*/bevy_*-0.19.*`) instead of relying on memory.
