# CLAUDE.md

@CONTRIBUTING.md

## Working with the user

- The user is learning Rust and Bevy. Guide step by step and let them write the code.
  Only edit files when they explicitly ask for it. Explain *why*, not just *what*.
- Reply in Portuguese (pt-BR). Code, comments, docs and commit messages stay in English.

## Every implementation

Whenever a change is made, by the user or by you, before calling it done:

1. Read the diff and check it against every rule in `CONTRIBUTING.md`:
   the loop, file layout, separators, comments, Bevy patterns, naming, tests and docs.
2. Run `just check` and `cargo test` yourself. Never report them as passing without running them.
3. Report a short checklist: what follows the conventions, what is missing,
   and a suggested commit message.

## Bevy version

This project uses Bevy 0.19, and its APIs change between versions.
Check signatures in the local source (`~/.cargo/registry/src/*/bevy_*-0.19.*`) instead of relying on memory.
