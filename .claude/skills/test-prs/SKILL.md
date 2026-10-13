---
name: test-prs
description: >
  Use when @starquake wants open ZX Sidekick PRs tried by hand: "test every
  PR", "let's test the PRs", "ask me if it works", "/test-prs". For each PR in
  `Your review`: build it, start it where its change shows, ask through the
  testing tool (AskUserQuestion) whether it works, and on a yes label it
  `ready to merge` and record the test in the PR. A problem is sorted into this
  PR's or another ticket's. At the end, ask to merge everything labelled, and
  merge with `merge-pr`. Trigger even if the user doesn't say "skill".
---

You test open PRs with the maintainer, one at a time, and label the ones that
work. The maintainer's yes to a hands-on test is what lets you add
`ready to merge` (CLAUDE.md); nothing else does.

## Before the first PR

- List the PRs to test: those whose ticket is in `Your review`, oldest first
  (`.claude/scripts/board.sh list "Your review"`, then each ticket's PR).
- Build each PR's release binary, one after another in the background, each in
  its own worktree with its own `CARGO_TARGET_DIR` (`target/wt-<issue>`):
  `cargo build --release -p sidekick`. To wait for one, run the same build in
  the foreground: cargo waits for the lock and returns once it's built.
- Test data lives outside the repository: a data folder holding only the kept
  tape (`ZX_SIDEKICK_DATA`), so no "continue where you left off?" offer gets in
  the way; crafted situations, `.pok` files and copies of a definition in the
  scratchpad. Never edit `games/` in the repository for a test.

## Each PR

1. **One window at a time.** Stop the window you started last (TaskStop its
   background task) before launching the next. Windows left open pile up.
2. **Start where the change shows**, so the maintainer doesn't have to walk
   there: `--load` a crafted situation (`room = 20`, BLOB's place, switches),
   the definition's own situations (`door.toml`), or the title for start-up
   changes. Launch it in the background.
3. **Ask with AskUserQuestion**, in the same turn as the launch. Say the PR
   number and what it changes in a few words; say what to do and what to look
   at, concretely (the level to set, the room on the map by row and column, the
   key to press). Options: *Works*, *Problem* and *Couldn't test*, each with a
   line. Add a second question for a second thing worth trying.
4. **A PR with nothing to see in a window** (a command, a script helper): run
   it yourself and show the result, as text or with SendUserFile for pictures,
   and ask whether it's right. For one with nothing visible at all, ask whether
   to label it on its automated tests, and say what that means: no hands-on
   test, trusting the tests alone.
5. **"Restart" means stop and launch again**, then ask the same question.

## When it doesn't work

Find out what the maintainer saw before deciding: ask for the room, the side,
a screenshot. Then sort it:

- **This PR's**: fix it on the branch (gate, commit, push), rebuild, and ask
  again.
- **Another ticket's**: what the PR never set out to do (a wall inside a room
  when the PR is about edges, a gap in a switch the PR only imports). Explain
  in one or two sentences, add it to that ticket as a case to get right, and
  ask whether to label this PR anyway.
- **Something new**: file it in the Backlog with its route, and ask the same.
- **The test was badly chosen** (a patch whose effect can't be seen alone):
  say so, change the test so the PR's own work shows, and ask again.

Facts the test overturns (a byte read wrongly from the code) are corrected
where they're written, on the PR's branch, in the same pass.

## On a yes

- `gh pr edit <n> --add-label "ready to merge"`.
- Append to the PR body a `## Checked by hand` section: one or two lines saying
  what @starquake did and saw, and any request it gave rise to, with its
  ticket.
- A request that isn't a defect ("show it longer", "scale the panel") becomes
  a Backlog ticket, routed; it doesn't hold the PR.

## At the end

List what's labelled, what isn't and why, and the tickets filed on the way.
Ask once whether to merge everything labelled. On a yes, merge with
`merge-pr`, one PR at a time: rebase each onto `main` locally, run the gate,
force-push, and merge once its required checks pass, re-reading the label at
that moment. PRs that touch the same files conflict as each lands; resolve
them keeping both sides' intent, and run the gate again before pushing.
