---
name: build-slice
description: >
  Use whenever an approved ZX Sidekick ticket gets built: "build #NN", "go
  ahead and implement it", "execute the plan", "resume #NN", a ticket moved to
  `Build` on the board, or a `go` / `approved` comment on a `Your sign-off`
  ticket. Also for bugs and tweaks small enough to need no plan. Executes the
  plan task by task on ONE PR: branch `<issue>-<slug>`, failing tests first,
  `scripts/check.sh` green per signed one-line commit, CI watched after every
  push, nothing of any game's code or material committed, then the ticket
  moves to `Your review`. Merging stays gated on the `ready to merge` label.
  Trigger even if the user doesn't say "skill".
---

You execute an approved plan from a ticket, task by task, on one PR.
Precondition: the maintainer has authorised the build: approved the *Plan*
(a `go` / `approved` comment), or moved the card to `Build`; or it is a bug or
tweak small enough to need no plan. Without that, stop and route to
`design-slice`.

**A card the maintainer moved to `Build` is the go even when the plan does not
fit**: missing, or naming code this repository doesn't have (a ticket ported
from a sibling project). Write the plan into the body before the first commit,
say so in a comment, and build it; don't send it back for sign-off. What you
would still never do is invent a plan mid-build on a ticket nobody moved: that
one goes back to `Plan`.

## Setup

- Set the card to `Build` (`.claude/scripts/board.sh state <n> "Build"`).
- Branch from an up-to-date `main`, named `<issue>-<slug>` (e.g.
  `12-sync-address-frames`). In a worktree, run every `git` command inside it;
  never `git checkout` in the shared checkout while another agent may be
  working there. Never `git stash`: stashes are shared across worktrees, so a
  pop can land another branch's work.
- Re-verify the plan against the current tree before the first commit: the
  named crates, modules and functions still exist, the pinned `rustzx` `rev`
  in `Cargo.toml` is the one the plan assumed, and nothing merged since moved
  the ground. Surface drift rather than improvising.
- **Resuming?** The issue's ticked checkboxes plus the branch's commits are
  the progress record. Confirm they agree, and pick up at the first unticked
  task.

## The task loop

1. **Failing test first** where the plan says so (a `#[cfg(test)]` module
   beside the code, or an integration test under `crates/<crate>/tests/`), and
   confirm it fails for the right reason. A test that can never run (ignored,
   unreachable) is worse than none: check it actually ran. A test that needs
   the player's tape in `assets/` skips in CI, so run it locally with the
   tape, and say in the PR that you did.
2. Implement. **Keep the copyright rules** (CLAUDE.md, "Working with
   copyrighted game code"): nothing of a game's code, disassembly, graphics,
   maps, text or sound goes into the change. A definition holds facts only, in
   our own words; each fact it relies on is written in `docs/re/<game>.md`,
   marked **confirmed**, **read** or **guess**. Disassembly stays in
   `.scratch/`. A regenerated committed file (the definition JSON Schema, a
   golden image) travels with the change that moved it, since the tests fail
   on drift. Golden images show the program's own drawing over a plain
   stand-in for the game's picture, never a real game's picture.
3. **A change to the machine is made in the fork first.** `rustzx-core` and
   `rustzx-z80` come from [zx-sidekick/rustzx](https://github.com/zx-sidekick/rustzx),
   pinned by `rev`. Build against a local checkout with the ignored
   `.cargo/config.toml` override ("Working on the engine" in `ZX-SIDEKICK.md`),
   open the patch as its own PR on the fork with its tests and its entry in
   the fork's `ZX-SIDEKICK.md`, and never merge it without the maintainer's
   approval there. Once it is merged, delete the override and bump the `rev`
   here to the fork's merge commit (never a branch commit: squash-merging
   leaves it out of the fork's history).
4. **Gate on the exit code, never on grepped output:**

   ```bash
   if scripts/check.sh > "$TMPDIR/check.log" 2>&1; then echo "GATE PASS"; else echo "GATE FAIL"; tail -40 "$TMPDIR/check.log"; fi
   ```

   `scripts/check.sh` is `cargo fmt --check`, `cargo clippy --all-targets --
   -D warnings`, `cargo test` and the repo guard, and names whatever it
   skipped for want of a tape in `assets/`. The tool shell is zsh: never pipe
   the gate into `tail` and read `$?`, since that's the pipe's status.
5. One commit per task. The message is a plain one-line capitalised subject
   saying what: no body, no attribution trailer. Commits are signed (local SSH
   signing; the ruleset refuses unsigned ones). Stage explicit paths
   (`git add <path> …`), never `-A`, `.` or `-f`. Check
   `git diff --cached --name-only` before committing: no `.tap`, `.tzx`,
   `.z80`, `.sna`, `.szx` or `.rom`, and nothing from `assets/`, `.scratch/`,
   `starquake-emulated-mockups/` or `.cargo/config.toml`. Push, then tick the
   task in the issue's *Plan*.
6. **Watch CI to completion in the foreground and read EVERY job**, for the
   commit you just pushed (a stale run shows for a few seconds after a push):

   ```bash
   while gh pr checks <n> --json bucket -q '.[].bucket' | grep -q pending; do sleep 30; done
   gh pr checks <n> --json name,bucket -q '.[] | "\(.bucket)\t\(.name)"'
   ```

   The required ones are `lint` and `test`; the repo guard must pass too. A
   subagent is never woken by its own background task, so don't background
   the watch and end. **A flaky test is a bug**: reproduce it, root-cause it,
   and fix the cause. A re-run unblocks a PR; it doesn't end the flake.

**Behind `main`?** Rebase locally (`git fetch origin && git rebase origin/main`),
re-run the gate, `git push --force-with-lease`. Never merge `main` into the
branch and never `gh pr update-branch`: a server-side update makes unsigned
commits, which the ruleset blocks, and linear history refuses a merge commit.

## The PR

Open it early as a draft, linked to the ticket. **`Closes #NN` only if this PR
completes every task in the ticket's plan**; otherwise `Part of #NN`. A task
left for the maintainer (a settings change, a manual step, a fork PR still
waiting) counts as open: a `Closes` would shut the ticket on merge with that
task still undone (zx-recompiled/starquake-recompiled#22).

**Review is the bottleneck**: the maintainer reviews alone, so the body is a
guide to reviewing, not a defence. It carries no attribution line, and neither
do commit messages.

1. **`## Where to look`**: the two or three judgement calls the maintainer
   might disagree with, each naming its file. None? Say so in one line.
2. `---`, then *Mechanically verified — skip unless curious.* and ONE short
   paragraph: which gates passed, what was checked, and which tests needed a
   tape and ran locally.

Nothing of a game's code, disassembly or material goes in the body or a
comment either: facts in our own words only. Comments follow the same rule as
the body: the answer goes in the **first line**. The PR title becomes the
squash commit's subject on `main`, so it follows the commit rule too: one
plain capitalised line. Keep mechanical churn (formatting, renames) in its own
commit; a regenerated schema or golden image is the exception and travels with
the change that moved it.

## Review the whole diff

Before the PR is handed over, read the change back as a reviewer would, all
of it against `main`, not just the last commit
(zx-recompiled/starquake-recompiled#74):

```bash
git fetch origin && git diff origin/main...HEAD
```

The gates prove it formats, lints and passes its tests. They cannot see:

- **Leftovers**: code, names, comments and docs from an earlier iteration, or
  from a design the maintainer changed along the way. Clippy catches dead
  code; it does not catch a comment describing behaviour that is gone.
- **The ticket**: each item in *Decisions*, and the approved mockup, against
  what the code actually does.
- **States and inputs**: what every key, pad button and stick does in every
  state of the thing built; what a key or button still held does when a menu
  closes or a screen changes under it; what happens between frames and what
  crosses threads (the emulation, the window, the sound callback).
- **Tests** that assert the decisions, not the current implementation.
- **Game material**: nothing of a game's code, disassembly, graphics, maps,
  text or sound in the diff; definitions hold facts only, each recorded in
  `docs/re/<game>.md` with its confidence; no tape, snapshot or ROM staged;
  no image holding a real game's picture.

**Sort each finding into one of two kinds** (zx-recompiled/starquake-recompiled#74):

- **A defect**, with one right answer: a bug, leftover code or a stale
  comment, a name from an earlier iteration, a test that does not test what it
  says, game material that must come out. **Fix it without asking**, in its
  own commit.
- **A judgement call**: anything that changes what the player sees or how it
  behaves, departs from the ticket's *Decisions* or the approved mockup,
  widens or narrows the scope, or leaves the copyright rules in doubt. **Do
  not fix it; ask.** When unsure which kind a finding is, it is a judgement
  call.

Ask by posting a review comment on the line it is about. It opens with the 🤖
"Comment by Claude" attribution line, says what is wrong, gives the
recommended fix, and ends with the three words the maintainer can reply with:

```bash
gh api repos/zx-sidekick/zx-sidekick/pulls/<n>/comments \
  -f commit_id="$(git rev-parse HEAD)" -f path=<file> -F line=<line> -f side=RIGHT \
  -f body="$(cat finding.md)"
```

| reply starts with | Claude |
|---|---|
| `fix` | fixes it as recommended (or as the reply amends), pushes, replies with the commit, and resolves the thread |
| `skip` | leaves it, replies to acknowledge, and resolves the thread |
| `ticket` | files it as a Backlog issue, replies with the link, and resolves the thread |

Any other reply is a question or an extra comment: answer it in the thread,
and act on it only when it asks for a change. Resolve a thread once it is
acted on and nothing is left to answer; **the ruleset blocks merging while any
conversation is unresolved**, so a thread left open holds the PR. Resolving
takes the thread's node id:

```bash
gh api graphql -f query='{ repository(owner:"zx-sidekick", name:"zx-sidekick") {
  pullRequest(number:<n>) { reviewThreads(first:50) { nodes { id isResolved
    comments(first:1) { nodes { databaseId } } } } } } }'
gh api graphql -f query='mutation { resolveReviewThread(input:{threadId:"<id>"}) { thread { isResolved } } }'
```

Replies arrive through the board monitor, which watches PR review comments. A
finding with no line to anchor to (something missing) goes on the file's first
changed line, saying so.

The PR body gets a *Found in review* section: each defect fixed, with its
commit, and how many judgement calls are waiting as comments (or that there
were none of either). The card moves to `Your review` with those still open;
the maintainer's replies are part of the review.

## Finish

- Docs: update `README.md` / `CLAUDE.md` if anything they say changed,
  `ZX-SIDEKICK.md` if the plan it records moved, `docs/definition-format.md`
  if the format did, and `docs/re/<game>.md` for every fact the change found
  or confirmed.
- **A mockup in the ticket?** Its image must reach `main` in this PR. Once
  merged, repoint the embed from `/raw/<branch>/` to `/raw/main/`, because
  merging deletes the branch and a branch embed 404s.
- `gh pr ready <n>`, watch CI green, then **STOP. Never merge.** The
  `ready to merge` label is the maintainer's; `merge-pr` lands it once they
  add it.
- **Re-read the ticket's plan before handing over.** Every box this PR covers
  is ticked. If any box is still open, including one that's the maintainer's,
  the PR body must say `Part of #NN`, not `Closes #NN`; fix it now.
- **Check the ticket's sub-issues too** before writing `Closes #NN`:
  `gh api repos/zx-sidekick/zx-sidekick/issues/<n>/sub_issues -q '.[] | select(.state=="open") | .number'`.
  Any still open (other than ones this PR closes) means `Part of #NN`:
  zx-recompiled/starquake-recompiled#76 closed its #1 with four sub-issues
  still open (zx-recompiled/starquake-recompiled#84). A PR for a sub-issue
  says `Closes` only for that sub-issue, never for its parent.
- Move the card to **`Your review`** (NOT `Your sign-off`, which is the
  pre-build gate), and post a Next-steps comment: *Next: review the PR and add
  `ready to merge`.* List any task still open after the merge in that comment,
  with whose it is.
