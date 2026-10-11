---
name: Spec (spec + plan)
about: A feature or change that is designed in this issue before it is built
labels: 'needs: spec'
---

<!-- The workflow (CLAUDE.md, "How work lands"): fill the SPEC, settle its
     decisions with the maintainer here, then fill the PLAN. The maintainer's
     OK on the plan (dragging the card to Build, or a `go` comment) is the
     go-ahead to build. The implementation PR says `Closes #NN`.

     THIS BODY IS THE LIVING SPEC. THE COMMENTS ARE THE HISTORY.
       - BODY: always current. When a question is answered, MOVE it into
         Decisions and DELETE it from Open questions. The body must never keep
         asking something that has been settled.
       - COMMENTS: append-only. A new `> 🤖 **Next steps**` comment each time
         the state changes; never edit an old one. Answers given in chat are
         written back into this BODY.

     Filed by Claude? The body's first line is the attribution header:
     > 🤖 **Issue by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.
     `gh issue create --body-file` bypasses this template, so read it and fill
     its sections by hand. -->

## Spec

### Goal

<!-- One paragraph: what ships, and the one-line reason it exists. -->

### Decisions

<!-- Numbered, settled with the maintainer, each with its why. -->

### Open questions

<!-- Anything still FOR the maintainer, with the options and a
     recommendation on every line. End with a block they copy, fill in and
     paste back as a comment:

     ```
     # keep your pick, delete the rest
     Q1 <topic>: option a (rec) / option b / option c
     Q2 <topic>: option a / option b (rec)
     notes =
     ```

     No view on one? Say so on its line: `discuss (rec)`. Mirror the block in
     the Next-steps comment. Delete this section once everything is settled. -->

### Design

<!-- The crates (`crates/sidekick-*`), the definition format
     (`docs/definition-format.md`, the schema), a game's definition
     (`games/<id>/`), a patch the rustzx fork needs (a PR there first, then the
     rev bumped here), and the tests, at whatever depth the change needs. Name
     real Rust symbols (crates, modules, types, functions), since the plan
     builds on them. Anything that changes the plan says what moves in
     `ZX-SIDEKICK.md`. -->

### Mockup (visual work only)

<!-- Anything whose value is how it LOOKS gets a mockup approved here before
     the real UI is built (the `mockup` skill): a headless render of the
     program's own drawing with a plain stand-in for the game's picture, or a
     sketch for UI that doesn't exist yet. Never a game's own picture: those
     stay out of the repository, issues and PRs. Embed it with this form:
     ![mockup](https://github.com/zx-sidekick/zx-sidekick/raw/<branch>/docs/mockups/<file>.png)
     The PR that merges it repoints the embed to /raw/main/. -->

### Game material

<!-- Work that needs facts about a game: which ones (addresses, meanings,
     patches), how they'll be found, and where they're written up
     (`docs/re/<game>.md`, marked confirmed / read / guess). Under CLAUDE.md's
     "Working with copyrighted game code": facts only, in our own words;
     disassembly stays in `.scratch/`; no tape, snapshot or ROM committed. -->

### Out of scope

<!-- Deferred pieces, each with the issue that tracks it. -->

## Plan

<!-- Tasks in landing order; each ends green (`scripts/check.sh`) and is one
     commit on the implementation PR. Failing tests first where practical.
     Ticked as they land: the ticks and the branch are the progress record. -->

- [ ] Task 1 —
- [ ] Task 2 —
- [ ] Docs: `README.md` / `CLAUDE.md` / `ZX-SIDEKICK.md` / `docs/` updated in the same PR if anything they say changed
