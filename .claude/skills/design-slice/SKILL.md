---
name: design-slice
description: >
  Use whenever a ZX Sidekick ticket needs its spec or plan: "let's design X",
  "spec out #NN", "pick up #NN" when #NN has no approved plan yet, or a ticket
  sitting in `Spec` or `Plan` on the board. Runs the design-first workflow: the
  spec and plan are written IN the GitHub issue (spec template), decisions are
  settled with the maintainer there through a copy-paste answer block, and
  building only starts after the maintainer's explicit OK. Trigger even if the
  user doesn't say "skill".
---

You design a change **in its GitHub issue**: the issue is the single design
record, and the maintainer may never read chat. The one rule that matters
most: **never auto-proceed from plan to implementation.** The maintainer's OK
(the card dragged to `Build`, or a `go` / `approved` comment) is the build
signal, and it must be explicit.

Move the card at each pause with `.claude/scripts/board.sh state <n> "<state>"`.
`Spec` and `Plan` are yours; `Your input` and `Your sign-off` are the
maintainer's.

## Step 1: the issue

- **First, choose the route.** A card in `Spec` means "your call, go", not
  "write a spec": decide how much process it needs, and say so on the ticket
  with the reason (zx-recompiled/starquake-recompiled#26).
  - **A bug or tweak** small enough to need no plan: comment *"Routing: straight
    to Build — …"*, move it to `Build`, and hand it to `build-slice`. No spec.
  - **No design question left, but work that wants a plan**: move it to
    `Plan` and write the plan. No spec.
  - **A small decision**: a short spec with its answer block; the plan is a few
    lines in the same body once it's answered.
  - **A feature**: the full route below.

  The maintainer overrides by dragging the card back.
- **No issue yet?** Create one from `.github/ISSUE_TEMPLATE/spec.md`. Read that
  file and fill its sections; `gh issue create --body-file` bypasses templates
  entirely, so nothing fails when the structure goes missing. The body's first
  line is the attribution header:
  `> 🤖 **Issue by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`
- **A sub-issue** (filed under a parent, or split out of one) names its parent
  as `Part of #NN` and never with `Closes`, `Fixes` or `Resolves`; nor does
  its plan ask its PR to close the parent. A parent's own plan, while any
  sub-issue is open, ends in `Part of #NN` too:
  zx-recompiled/starquake-recompiled#76 closed its #1 with four sub-issues
  still open (zx-recompiled/starquake-recompiled#84).
- **The issue exists but is free-form?** Restructure its body into the
  template's sections, keeping everything true that's already there.
- **The BODY is the living spec; the COMMENTS are the history.** Opposite
  editing rules:
  - The **body** is always current: rewrite it freely. When a question is
    answered, MOVE it into *Decisions* and DELETE it from *Open questions*. A
    body that still asks a settled question is the bug.
  - The **comments** are append-only: never edit one; post a new
    `> 🤖 **Next steps**` comment each time the state changes.
- Read the code before writing: name real symbols (the crate under
  `crates/`, its module, type and function; the definition section in
  `docs/definition-format.md`), not "the core" or "the UI". Check
  `ZX-SIDEKICK.md` for what the plan already settled (the build order in §19
  among it), and `docs/definition-format.md` for what a definition already
  says. A change to the machine itself names the patch to the
  [zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx) fork it needs.
- Set `Spec` while you're writing it, so the board shows it's your turn.

## Step 2: the spec

- **Goal**: what ships, plus the one-line reason.
- **Decisions**: numbered, each with its why. Anything unsettled is a question
  TO the maintainer. Never decide design direction yourself.
- **Game material**: the copyright rules (CLAUDE.md, "Working with
  copyrighted game code") hold for the whole change. Name the game facts it
  relies on (addresses, what the bytes there mean, patches with their
  expected bytes), each recorded in `docs/re/<game>.md` as **confirmed**,
  **read** or **guess**, and say which are still to find. Nothing of a game's
  code, disassembly, graphics, maps, text or sound goes into the repository,
  the issue or a definition: the program reads those from the player's tape
  at run time. A change that seems to need any of it says so here, as an open
  question, before the work starts.
- **Open questions end with a copy-paste answer block.** One line per
  question, the options inline, and **every line carrying a `(rec)`**:

  ~~~
  ```
  # keep your pick, delete the rest
  Q1 the panel's width is: fixed at 3× (rec) / follows the window
  Q2 a training switch shows: in the menu only (rec) / in the panel too
  notes =
  ```
  ~~~

  The recommendation belongs **in the block**, not in prose under it: a bare
  menu makes the maintainer reconstruct reasoning you already did. No view?
  Write `discuss (rec)` on that line: "no recommendation" is real information.
  Before posting, re-read the prose against the block: **a question missing
  from the block is effectively not asked.**

  Mirror the block in the Next-steps comment, so it can be answered from the
  thread.
- **Answers in chat are allowed; leaving them there is not.** When the
  maintainer answers conversationally, write the answers into the BODY
  (*Decisions*, dated, with a verbatim quote wherever the wording carries
  reasoning) and post a Next-steps comment saying so **before acting on
  them**, so the record survives a session that ends unexpectedly. A fix to a
  missing or wrong block is a NEW comment, never an edit.
- **Mockup**: if the change's value is how it looks, make the mockup now
  (`mockup` skill) and embed it in the *Mockup* section. Approving the
  picture is part of the spec OK.
- **Hand off**: posting the block moves the ticket to `Your input` in the same
  step. Nothing open? Skip straight to the plan. A card in a lane carries no
  `needs:` label; drop one if it still has it.

## Step 3: settle, then plan

Fill the plan only once the decisions are settled. Set `Plan` while you write
it. A plan already in the body, ported from another project or drafted
earlier, is checked against this repository rather than rewritten: fix what
names code that isn't here, or say in the Next-steps comment what the build
will map. Tasks go in landing order, each one green commit
(`scripts/check.sh`); failing tests first where practical. A change to the
machine is its own task: the fork PR first, then the `rev` bump here once it
merges. Say which tests need the player's tape, since those skip in CI. The
last task updates `README.md` / `CLAUDE.md` / `ZX-SIDEKICK.md` /
`docs/definition-format.md` if anything they say changed.

## Step 4: STOP

Set `Your sign-off`, post a Next-steps comment (*Next: drag the card to `Build`
or comment `go`*), and **end there**. The build belongs to `build-slice` and
starts only on the maintainer's go.
