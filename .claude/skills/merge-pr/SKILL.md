---
name: merge-pr
description: >
  Use whenever a pull request should be merged or landed: "merge #NN", "land
  this PR", "merge the ready PRs", or a board pass finding a PR that carries
  `ready to merge`. Runs this repo's merge checks: the `ready to merge` label
  (the maintainer's approval; NEVER merge without it), green CI, a title that
  still describes the diff, and a rebase if behind, then squash-merges.
  Trigger for any merge request even if the user doesn't say "skill".
---

**A PR is mergeable only when it carries the `ready to merge` label.** The
label *is* the maintainer's approval (GitHub won't let the author approve
their own PR, and `gh` acts as the maintainer). It's their "ask one last
time", given on GitHub. Without it you stop and say so. Never add it yourself.

**Anything that becomes permanent at merge is verified AT merge**: the label
can be withdrawn, CI can go stale on a push, and the title can be outrun by
its own diff. Re-read all three at the last moment.

## Step 1: the label

```bash
gh pr view <n> --json labels,mergeable,mergeStateStatus \
  --jq '{labels: [.labels[].name], mergeable, state: .mergeStateStatus}'
```

No `ready to merge`? **STOP**, and tell the maintainer it's missing. "Merge
it" in chat without the label means: surface that the label is missing, and
let them add it or explicitly override.

## Step 2: CI must be green

```bash
gh pr checks <n>
```

Every check must pass, and it must belong to the PR's current head commit.
Never merge red or pending CI.

## Step 2b: every conversation resolved

Nothing is merged while a review conversation is unresolved (starquake-recompiled#74).
On starquake-recompiled a ruleset enforces that; here the organisation's free
plan allows no ruleset on a private repository (`.github/rulesets/main.json`
holds the one to apply if that changes), so this step is the only thing that
checks it, and the same goes for green CI and squash-only merging.
A thread still open is a finding or a comment not yet acted on: act on it
first (`build-slice`, *Review the whole diff*), or ask the maintainer.

```bash
gh api graphql -f query='{ repository(owner:"zx-sidekick", name:"zx-sidekick-starquake") {
  pullRequest(number:<n>) { reviewThreads(first:50) { nodes { isResolved path line } } } } }' \
  --jq '[.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved | not)]'
```

## Step 3: the title still describes the diff

The squash commit takes the PR title as its subject, permanently:

```bash
gh pr view <n> --json title,files -q '.title + "\n" + ([.files[].path] | join("\n"))'
```

If the scope changed since the PR was opened, fix the title with
`gh pr edit <n> --title "…"`.

## Step 4: rebase if behind

If `mergeStateStatus` is `BEHIND`/`DIRTY` or `mergeable` is `CONFLICTING`:

```bash
git fetch origin --quiet && git checkout <branch> && git rebase origin/main
scripts/check.sh    # the combined result must be green
git push --force-with-lease
```

A force-push re-runs CI and can drop the label, so go back to Step 1.

## Step 5: merge

```bash
gh pr merge <n> --squash --match-head-commit "$(gh pr view <n> --json headRefOid -q .headRefOid)"
```

`--match-head-commit` refuses the merge if anything was pushed since you
checked. The repo deletes merged branches automatically. Then:

- Update the local checkout: `git checkout main && git pull --ff-only`. Do
  this only if no agent is working in the shared checkout; the merge itself is
  server-side and never needs it.
- **Check every issue the merge closed still deserved closing.** Read its
  plan: if any task is unticked (a maintainer step counts), the `Closes` was
  wrong. Reopen it (`gh issue reopen <n>`), move it to the state of whoever
  holds that task, and post a Next-steps comment naming it (starquake-recompiled#22).
- Otherwise the board's `Item closed → Done` workflow moves the card. Check
  that it did (`board.sh get <issue>`); if it didn't, move it and report that
  the workflow is off.
- **Wait for `main`'s own CI and read it.** A green PR does not mean a green
  `main`: nothing requires a branch to be up to date before
  merging, so the checks passed against an older base. If `main` is red, fixing
  it comes before anything else and does not need asking about.

  ```bash
  until [ "$(gh run list -R "$R" --branch main --workflow CI --limit 1 --json status -q '.[0].status')" = "completed" ]; do sleep 30; done
  gh run list -R "$R" --branch main --workflow CI --limit 1 --json conclusion,headSha
  ```

- **Run the checks against the game on the merged `main`**
  (`SK_ASSETS=… scripts/check.sh`) if the merge touched the machine, the ROM
  answers, the processor or its pin. CI cannot run them, so a merge is the
  last point anything checks.

**Merging several:** one at a time, re-checking the next PR after each merge,
since merging one advances `main`. Two PRs touching the same files can't both
stay clean: rebase the second, don't force it.

If `gh pr merge` is refused by the permission classifier while the label is
present and CI is green, re-read the label and retry once. If the label is
absent, that's a real stop.
