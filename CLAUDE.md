# ZX Sidekick

ZX Sidekick runs original ZX Spectrum games, from the player's own copy, in an emulated machine, and adds training switches, gamepad controls and guidance around them. What it does for a game comes from that game's **definition** in `games/`. The plan is `ZX-SIDEKICK.md`; the definition format is `docs/definition-format.md`.

## Working with copyrighted game code

Games and the Spectrum ROM belong to their rights holders. These rules keep the project on the right side of that. They apply to every session, every game, and every file, issue, PR and comment.

1. **A legally distributed copy, never shared.** Work from the supported dump named by SHA-1 in the game's definition. For Starquake, that's the copy World of Spectrum distributes. Keep it in `assets/` (ignored by git). Never commit, upload, attach or share it. Players download their own.
2. **Disassembly is for finding facts and understanding how the game works, nothing else.** It is never used to recreate, port or translate the game, and never published. It lives only in `.scratch/` (ignored by git). None of it goes into a commit, a doc, an issue, a PR, a comment, or a summary in chat.
3. **Publish facts only, in our own words.** Addresses, what the bytes there mean, and short patch bytes with the bytes they replace (`expect`). Write each fact in `docs/re/<game>.md`, marked **confirmed** (seen running), **read** (understood from the disassembly) or **guess**. No listings, no code excerpts. No extracted maps, text, graphics or sound either: the program reads those from the player's tape at run time.
   **Screenshots are the exception** (@starquake): a picture of the program running a game, as a player sees it, may be committed and shown in issues and PRs, to show what was built. Graphics, fonts, maps or sprite sheets extracted as files still may not. **Mockups never use a game's assets**: they're drawn with stand-ins (`docs/mockups/starquake/mockups.html`); the product uses the real ones, read from the player's `.tap` or `.tzx`.
4. **Use whatever is quickest and most accurate.** Our own disassembly, tracing, memory search, watchpoints: no method is preferred. Read a public disassembly only if it's published with the rights holder's permission, or holds annotations only (for example a SkoolKit control file applied to our own copy). Never download an unlicensed full listing.
5. **Never circumvent copy protection.** A game with a protected loader (Speedlock and the like) loads in the emulator as it would on a Spectrum. Nobody decrypts or strips it to find facts.
6. **No money on any game.** No donations, tiers or paid builds tied to a game. Ask the rights holder before anything commercial; asking for permission is always the better option.

The repository never holds tapes, snapshots or ROM files. `scripts/check.sh guard` (in CI's `lint` job) fails on any `.tap`, `.tzx`, `.z80`, `.sna`, `.szx` or `.rom`. The ROM the emulator needs comes inside the `rustzx-core` dependency, under Amstrad's permission (`THIRD-PARTY.md`).

## Hard rules

- **Never `kill` a process you didn't start.** Ask first.
- **Never `git stash`.** Stashes are shared across worktrees, so a pop in one can bring in another branch's work. Commit to a branch or leave the change in place.
- **Nothing becomes public without asking**: the board, a repository's visibility, a release or a tag.

## The engine

The machine is `rustzx-core` and `rustzx-z80` from the [zx-sidekick fork of RustZX](https://github.com/zx-sidekick/rustzx), a git dependency pinned to one commit in `Cargo.toml`. Changes to the machine are made there, as numbered patches recorded in its `ZX-SIDEKICK.md`, each by pull request, under that repository's own checks. Then bump the `rev` here, in a PR of this repository's own. "Working on the engine" in `ZX-SIDEKICK.md` says how to build against a local checkout.

## Commands

```bash
scripts/check.sh          # the gate: repo guard, fmt, clippy, tests; before every PR
scripts/check.sh test     # one step: guard, fmt, clippy or test
cargo test -p sidekick-core
```

## How work lands

**The ticket is canonical.** Every conversation about a piece of work happens in its GitHub issue. Chat is optional and the maintainer may not read it: an answer given in chat is written back into the issue body before acting on it.

- **Claude reviews its own diff before handing a PR over**: the whole branch against `main`, for what the gates cannot see (leftovers from earlier iterations, behaviour against the ticket, input and state edge cases, and the copyright rules above). Defects are fixed straight away and listed in the PR; judgement calls are a review comment on their line, left for the maintainer to answer `fix`, `skip` or `ticket` (`build-slice`).
- **Everything lands via a pull request** with an issue behind it, including chores and docs. One issue, one deliverable; a ticket that needs several PRs in different states is split into sub-issues. A PR says `Closes #NN` only when it completes every task in the ticket's plan, maintainer steps included; otherwise `Part of #NN`, and the ticket stays open. **A parent issue with a sub-issue still open is never closed by a PR**, whatever its own plan says, and a sub-issue's ticket or PR never mentions closing its parent: they say `Part of #NN`.
- **Dependabot's pull requests are the one exception**: they have no issue behind them and no card (`.github/dependabot.yml`), and they merge themselves: `.github/workflows/dependabot-auto-merge.yml` squash-merges each once the required checks pass, then dispatches CI on `main`. Claude never merges one by hand and never adds `ready to merge` to one. It keeps them mergeable: one left behind `main` is rebased locally and force-pushed. The rustzx fork is ignored there; its `rev` moves by hand.
- **The gate is `scripts/check.sh`** on every commit. The ruleset lives as code in `.github/rulesets/main.json`: squash only, signed commits, linear history, every review thread resolved, the branch up to date with `main`, and the required checks `lint` and `build (linux)`, `build (macos)`, `build (windows)`.
- **Branches and commits**: a branch is `<issue>-<slug>` (`3-definition-loader`). A commit is a plain one-line capitalized subject with no body and no trailer, signed with the local SSH key, staging explicit paths. Never change `user.email`: the signature only verifies under the configured identity. A branch behind `main` is rebased locally and force-pushed: never merge `main` into a branch and never `gh pr update-branch`, since a server-side update makes unsigned commits, which the ruleset blocks.
- **The board is the handoff baton**: the Status field of the "ZX Sidekick" organisation Project (https://github.com/orgs/zx-sidekick/projects/1). Read and move it with `.claude/scripts/board.sh`.

  ```
  Backlog · Your input · Spec · Plan · Your sign-off · Build · Your review · Done
  ```

  **If a state says "your", it is the maintainer's gate** and work stops; `Spec`, `Plan` and `Build` are Claude's and proceed without re-asking. `Your input` (questions) can interrupt any stage. `Your sign-off` comes BEFORE a build (approve the spec, plan or mockup); `Your review` comes AFTER it (the PR is open, awaiting `ready to merge`). Cards move both ways and stages can be skipped: a bug or tweak goes straight to `Build`.

- **Approval** of a spec or plan is the maintainer dragging the card on, or a `go` / `approved` comment. Never proceed from plan to build without it.
- **A card dragged to `Spec` means "your call"**: first decide whether it needs a spec at all, and say so on the ticket. With no design question left it goes on to `Plan`; a bug or tweak needing no plan goes on to `Build`.
- **A card dragged to `Build` is the go**, even when its plan is missing: the plan is written into the body before the first commit, and built, with no sign-off round.
- **The `Backlog` column's order is the priority.** Nothing leaves `Backlog` without the maintainer; "pick up the next one" means its top card.
- **Merging needs the `ready to merge` label** on the PR, re-read from the API at the moment of merging. Claude never merges without it, and adds it only on the maintainer's yes to a hands-on test of that PR (`test-prs`), or their choice to accept one with nothing to see on its tests.
- **A position in the flow is a Status; a property of a ticket is a label**: `ready to merge`, `hold` (skip entirely), `dependencies` (Dependabot's), and, **only while a ticket waits in `Backlog`**, its route: `needs: spec` (a design question to settle) or `needs: build` (none left), with the routing reason in the body. The route says where the card goes when it is picked up. **A card in a lane carries no route label**: the label comes off in the same step as the move out of `Backlog`. A parent carries none either.
- **A ticket ported from a sibling repository goes in the lane its content puts it in**: open questions to `Your input`, a settled spec with a plan to `Your sign-off` and one without to `Plan`, a parent to `Backlog` with no label.
- **The body is the living spec; the comments are append-only history.** When a question is answered it moves into _Decisions_ and is deleted from _Open questions_. Every state change gets a NEW `> 🤖 **Next steps**` comment; never edit an old one.
- **Questions go in a copy-paste answer block**: a fenced block headed `# keep your pick, delete the rest`, one line per question, every line carrying a `(rec)`, ending with `notes =`. Posting one moves the ticket to `Your input` in the same step.
- **Visual work gets a mockup approved before the real UI is built** (`mockup` skill): a headless render of the program's drawing over stand-ins, or a sketch for UI that doesn't exist yet; never a game's own picture, graphics or font. Mockups are committed under `docs/mockups/`; the reference design is `docs/mockups/starquake/`, drawn by its `mockups.html` (`scripts/render-mockups.sh`).

### Attribution: issues and comments yes, pull requests and commits no

`gh` acts as @starquake, so an unmarked Claude comment reads as the maintainer's own answer, and the board monitor tells them apart by exactly that prefix. So **every issue and comment Claude posts** opens with one of these lines, posted via `--body-file`:

- `> 🤖 **Issue by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`
- `> 🤖 **Comment by Claude** (AI pair-programmer working with @starquake) — posted through @starquake's account.`

**Pull request descriptions and commit messages carry no attribution line and no trailer**: the maintainer reviews and merges each PR and takes accountability for it, and the squash-merge commit on `main` is the maintainer's own, signed by GitHub.

The procedure behind each step lives in the skills: `work-the-board` (and its `/board` alias), `design-slice`, `mockup`, `build-slice`, `test-prs`, `merge-pr`, `issue-comment-replies`.

## CI required checks

`main` is protected by the ruleset in `.github/rulesets/main.json`, which is the source of truth. Its required checks are the jobs of `.github/workflows/ci.yml`: `lint` (repo guard, fmt, clippy on a pinned toolchain) and `build (linux)`, `build (macos)` and `build (windows)` (SDL3 built from source, the tests run, the binary kept as an artifact; Linux with Wayland and X11). When you add, rename or remove a required job, change the ruleset file in the same PR and apply it:

```bash
RID=$(gh api repos/zx-sidekick/zx-sidekick/rulesets --jq '.[] | select(.name=="main").id')
gh api -X PUT repos/zx-sidekick/zx-sidekick/rulesets/$RID --input .github/rulesets/main.json
```

A required context that no job produces blocks every PR.

## Rust

- **The strictest useful lints** for every crate (`[workspace.lints]` in `Cargo.toml`): no `unsafe`, `missing_docs`, clippy's pedantic, nursery and cargo groups, and the restriction lints that suit this code (`unwrap_used` outside tests, `print_stdout`, `todo`, `dbg_macro` and others). CI denies warnings. A lint allowed has its reason beside it, in `Cargo.toml` or on the attribute (`#[expect(…, reason = "…")]`).
- **The latest versions of dependencies.** A new dependency comes in at its latest release; Dependabot keeps them there. One held back has its reason in `Cargo.toml` or `.github/dependabot.yml`.
- `sidekick-core` and `sidekick-def` have no frontend dependencies, so they build and test anywhere, headless.
- Every public item has a doc comment, kept to what a reader needs.
- Errors are returned, never ignored; `unwrap` and `expect` only in tests, or with the invariant that makes them safe in the message.
- Game-specific facts never appear in Rust code: they live in a definition. The program knows Spectrum hardware, not games.

## Testing

Every change comes with tests. Write a test instead of a one-off check script: a scripted run verifies once and is thrown away; a test catches the regression forever. Screenshots are for looking, never for assuring behaviour.

- **Unit tests** in a `#[cfg(test)] mod tests` beside the code; **integration tests** in the crate's `tests/`, through its public interface.
- **Tests run in parallel, so they stay fast.** Each builds its own machine and shares no state with another; nothing serialises them. Split a slow test rather than letting one dominate the run.
- **No game in CI**: tests that need a tape read it from `assets/` and skip, saying so, when it's missing. CI runs everything else; the ROM tests run everywhere, since the ROM comes with `rustzx-core`.
- **Golden images** of the program's own drawing are committed, over a plain stand-in for the game's picture: CI has no tape.
- **A game's addresses come from its definition, in tests too**: a test looks a value's address up by name rather than writing the game's numbers into Rust.
- **A game's script is tested on stand-ins** in CI: its real definition and script on a bare 48K, the game held in a state by parking the processor at that state's address, stand-in values written where the definition says (`crates/sidekick/tests/starquake_script.rs`). The real-tape tests confirm the facts; these keep the script honest without a tape.
- **A flaky test is a bug to file**, not to rerun past.

## Comments

Default to none. Well-named identifiers and small functions say *what*; a comment earns its place only for a non-obvious *why*: a hidden constraint, an invariant, a workaround. Keep it short. Don't restate the code and don't reference the current task or caller (`// added for #12`); issue links that carry the reason are fine (`// see #12`).

## Writing

**Code uses US spelling** (color, behavior, center, gray): identifiers, comments, log messages and UI strings. Docs prose is free.

Release notes, README, UI strings, commit messages and PR descriptions are plain and factual: say what is there, with neutral verbs, and no selling.
