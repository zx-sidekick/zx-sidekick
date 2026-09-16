# zx-sidekick-starquake

Starquake (Stephen Crow / Bubble Bus, 1985), the plain game, running as the
**original program** in an emulated ZX Spectrum the player never sees. **The
repository contains no part of the game and no ROM**: the player supplies their
own tape at runtime, and the few ROM routines the game calls are answered by
the program itself. `GOAL.md` holds the aim and the hard legal rules; `PLAN.md`
the steps and what comes later.

The processor is `rustzx-z80` from our fork
([zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx)), pinned to a
commit, inside our own 48K bus (`crates/zx-spectrum`). The ROM-free machine is
`crates/sidekick`, and `tools/sk-check` checks it against a real ROM supplied
locally.

## Commands

- `scripts/check.sh` is the pre-PR gate: fmt, build, tests, clippy, docs, the
  libraries without the frontend, the dependency policy and `THIRD-PARTY.md`,
  the Fuse corpus, and, with `SK_ASSETS` pointing at a folder holding
  `starquake.tap` and `48.rom`, the checks against the game. **Gate on the
  exit code, never on grepped output.**
- `cargo run --release -p sk-check -- entry "$SK_ASSETS"`,
  `… -- rom "$SK_ASSETS" 6000`, `… -- keys "$SK_ASSETS"`,
  `… -- facts "$SK_ASSETS"` and `… -- map "$SK_ASSETS" 60` run the checks
  against the game on their own (`keys` and `map` need only the tape;
  `facts` walks into the teleporter booths only with the ROM).
- `cargo run --release -p sk-lab --bin planet -- "$SK_ASSETS"` draws the whole
  planet with every room's openings into `assets/planet.png`; the other
  `sk-lab` tools (`tools/sk-lab/README.md`) probe rooms and run the level 5
  search. They look, `sk-check` proves; none runs in CI or the gate.
- `cargo test -p zx-spectrum --test fuse -- --nocapture` checks the processor
  in our bus against the Fuse Z80 corpus (needs `assets/tests.in` and
  `assets/tests.expected`).
- `cargo run --release -p zx-sidekick-starquake -- "$SK_ASSETS/starquake.tap"`
  plays it. Add `--headless <frames> <dir> [level]` for screenshots of real play at a guidance level.
- `cargo about generate --all-features about.hbs -o THIRD-PARTY.md`
  regenerates the attributions after a dependency change.
- `cargo llvm-cov --workspace --summary-only` measures test coverage (needs
  `cargo install cargo-llvm-cov` and `rustup component add llvm-tools`). The
  line counts include the tests' own code; what is left uncovered needs a
  window, a sound card, a gamepad, or the player's tape (`sk-check`).
- The tool shell is zsh: never name a variable `status`, and run anything
  loop-shaped as a `bash` script.

## Invariants

- **No game or ROM data is ever committed.** `assets/` is ignored except its
  README, and CI has a job that fails if anything slips through. This is what
  makes the project legal to publish; nothing is worth breaking it for.
- **No translated game logic** (`GOAL.md`, rule 2). What the code knows about
  Starquake is facts: the tape's checksum, where it starts, memory addresses.
  The game runs as its own program; none of it is reimplemented. Reused code
  is our own generic work, recorded in `REUSED.md`.
- **The checks against the game are the contract.** `sk-check entry` must find
  the loader returning where the game is started, `sk-check rom` must find
  every compared ROM call answered as the real ROM does (20,326 of 20,326 when
  the fork was pinned), `sk-check keys` must find the joystick reaching the
  game and the pause key taken from it in all five control methods, and
  `sk-check facts` must find the entry points the guidance panel follows and
  End this game ending a game. A change that moves any of them is a deliberate, called-out
  decision, never a check adjusted to make it pass.
- **The processor is not self-certified.** It is checked in our bus against
  the Fuse corpus: 1,329 of 1,335 cases exact, the 6 undocumented-flag cases
  listed by name, and bus activity 1,335 of 1,335. The gate and CI require
  exactly that; a fork update that changes it is updated on purpose.
- **Say so when something is not checked.** What works but has not been
  checked by hand is marked that way in `README.md`, rather than left to be
  discovered.

## How work lands

**The ticket is canonical.** Every conversation about a piece of work happens
in its GitHub issue. Chat is optional and the maintainer may not read it: an
answer given in chat is written back into the issue body before acting on it.

- **Claude reviews its own diff before handing a PR over**: the whole branch
  against `main`, for what the gates cannot see (leftovers from earlier
  iterations, behaviour against the ticket, input and state edge cases).
  Defects are fixed straight away and listed in the PR; judgement calls are a
  review comment on their line, left for the maintainer to answer `fix`,
  `skip` or `ticket` (`build-slice`).
- **Everything lands via a pull request** with an issue behind it, including
  chores and docs. One issue, one deliverable; a ticket that needs several PRs
  in different states is split into sub-issues. A PR says `Closes #NN` only
  when it completes every task in the ticket's plan, maintainer steps included;
  otherwise `Part of #NN`, and the ticket stays open. **A parent issue with a
  sub-issue still open is never closed by a PR**, whatever its own plan says,
  and a sub-issue's ticket or PR never mentions closing its parent: they say
  `Part of #NN` (starquake-recompiled#76 closed its #1 with four sub-issues
  still open, starquake-recompiled#84).
- **The board is the handoff baton**: the Status field of the "ZX Sidekick
  Starquake" org Project (https://github.com/orgs/zx-sidekick/projects/1).
  Read and move it with `.claude/scripts/board.sh`.

  ```
  Backlog · Your input · Spec · Plan · Your sign-off · Build · Your review · Done
  ```

  **If a state says "your", it is the maintainer's gate** and work stops;
  `Spec`, `Plan` and `Build` are Claude's and proceed without re-asking.
  `Your input` (questions) can interrupt any stage. `Your sign-off` comes
  BEFORE a build (approve the spec, plan or mockup); `Your review` comes AFTER
  it (the PR is open, awaiting `ready to merge`). Cards move both ways and
  stages can be skipped: a bug or tweak goes straight to `Build`.

- **Approval** of a spec or plan is the maintainer dragging the card on, or a
  `go` / `approved` comment. Never proceed from plan to build without it.
- **A card dragged to `Spec` means "your call"**: first decide whether it needs
  a spec at all, and say so on the ticket. With no design question left it goes
  on to `Plan`; a bug or tweak needing no plan goes on to `Build`.
- **A card dragged to `Build` is the go**, even when its plan is missing or
  names another project's code: the plan is written into the body before the
  first commit, and built, with no sign-off round (#27).
- **The `Backlog` column's order is the priority.** Nothing leaves `Backlog`
  without the maintainer; "pick up the next one" means its top card.
- **Merging needs the `ready to merge` label** on the PR, re-read from the API
  at the moment of merging. Claude never adds it and never merges without it.
- **A position in the flow is a Status; a property of a ticket is a label**:
  `ready to merge`, `hold` (skip entirely), and, **only while a ticket waits in
  `Backlog`**, its route: `needs: spec` (a design question to settle) or
  `needs: build` (none left), with the routing reason in the body. The route
  says where the card goes when it is picked up. **A card in a lane carries no
  route label**: the lane already says where it stands, so the label comes off
  in the same step as the move out of `Backlog` (@starquake, 2026-09-15). A
  parent carries none either.
- **A ticket ported from a sibling repository goes in the lane its content puts
  it in**: open questions to `Your input`, a settled spec with a plan to
  `Your sign-off` and one without to `Plan`, a parent to `Backlog` with no
  label.
- **The body is the living spec; the comments are append-only history.** When
  a question is answered it moves into _Decisions_ and is deleted from _Open
  questions_. Every state change gets a NEW `> 🤖 **Next steps**` comment;
  never edit an old one.
- **Questions go in a copy-paste answer block**: a fenced block headed
  `# keep your pick, delete the rest`, one line per question, every line
  carrying a `(rec)`, ending with `notes =`. Posting one moves the ticket to
  `Your input` in the same step.
- **Visual work gets a mockup approved before the real UI is built**
  (`mockup` skill): a real headless screenshot for anything the Spectrum
  draws, a sketch only for the window's own screens.
- **Nothing becomes public without asking**: the repository, the board, a
  release or a tag.

### Attribution: issues, comments and PR descriptions yes, commits no

`gh` acts as @starquake, so an unmarked Claude comment reads as the
maintainer's own answer — and the board monitor tells them apart by exactly
that prefix. So **every issue, comment and pull request description Claude
posts** opens with one of these lines, posted via `--body-file`:

- `> 🤖 **Issue by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`
- `> 🤖 **Comment by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`
- `> 🤖 **Pull request by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`

The line on a pull request says what happens: Claude opens it, the maintainer
reviews and merges it, and takes accountability for what is merged
(@starquake, 2026-09-15; every earlier PR got the line the same day).
**Commit messages carry no attribution line**: the squash-merge commit on
`main` is the maintainer's own, signed with their key.

The procedure behind each step lives in the skills: `work-the-board` (and its
`/board` alias), `design-slice`, `mockup`, `build-slice`, `merge-pr`,
`issue-comment-replies`.
