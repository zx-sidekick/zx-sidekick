---
name: merge-pr
description: >
  Use whenever a ZX Sidekick pull request should be merged or landed: "merge
  #NN", "land this PR", "merge the ready PRs", or a board pass finding a PR
  that carries `ready to merge`. Runs this repo's merge checks: the
  `ready to merge` label (the maintainer's approval; NEVER merge without it),
  green required checks, every review thread resolved, a title that still
  describes the diff, and a local rebase if behind, then squash-merges.
  Dependabot PRs are not merged here: a workflow auto-merges them. Trigger for
  any merge request even if the user doesn't say "skill".
---

**A PR is mergeable only when it carries the `ready to merge` label.** The
label *is* the maintainer's approval (GitHub won't let the author approve
their own PR, and `gh` acts as the maintainer). It's their "ask one last
time", given on GitHub. Without it you stop and say so. Never add it yourself.

**Dependabot's PRs are the exception, and not yours to merge.**
`.github/workflows/dependabot-auto-merge.yml` squash-merges them (cargo and
GitHub Actions bumps) once the required checks pass. Never merge one by hand
and never add the label to one; keeping it mergeable (a local rebase) is
`work-the-board`'s.

**Anything that becomes permanent at merge is verified AT merge**: the label
can be withdrawn, CI can go stale on a push, and the title can be outrun by
its own diff. Re-read all three at the last moment.

The ruleset (`.github/rulesets/main.json`) is what the merge must satisfy:
squash only, signed commits, linear history, every review thread resolved,
the branch up to date with `main`, and the required checks `lint` and `build (linux|macos|windows)`
green.

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

Every check must pass, the repo guard included, and it must belong to the
PR's current head commit. Never merge red or pending CI.

## Step 2b: every conversation resolved

The ruleset refuses to merge while a review conversation is unresolved
(zx-recompiled/starquake-recompiled#74). A thread still open is a finding or
a comment not yet acted on: act on it first (`build-slice`, *Review the whole
diff*), or ask the maintainer.

```bash
gh api graphql -f query='{ repository(owner:"zx-sidekick", name:"zx-sidekick") {
  pullRequest(number:<n>) { reviewThreads(first:50) { nodes { isResolved path line } } } } }' \
  --jq '[.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved | not)]'
```

## Step 3: the title still describes the diff

The squash commit takes the PR title as its subject, permanently, so it is a
plain one-line capitalised subject like any commit:

```bash
gh pr view <n> --json title,files -q '.title + "\n" + ([.files[].path] | join("\n"))'
```

If the scope changed since the PR was opened, fix the title with
`gh pr edit <n> --title "…"`. While the file list is open, check it holds no
tape, snapshot or ROM and nothing of a game's material: the repo guard only
catches file extensions.

## Step 4: rebase if behind

The ruleset is strict, so a branch behind `main` does not merge. If
`mergeStateStatus` is `BEHIND`/`DIRTY` or `mergeable` is `CONFLICTING`, rebase
locally (in the branch's worktree if it has one; never `git stash`, since
stashes are shared across worktrees):

```bash
git fetch origin --quiet && git checkout <branch> && git rebase origin/main
scripts/check.sh    # the combined result must be green
git push --force-with-lease
```

**Never merge `main` into the branch, and never `gh pr update-branch`** (or the
"Update branch" button): a server-side update makes unsigned commits, which the
ruleset blocks, and a merge commit breaks linear history. The local rebase
re-signs the commits with the local SSH key.

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
  holds that task, and post a Next-steps comment naming it
  (zx-recompiled/starquake-recompiled#22).
- Otherwise the board's `Item closed → Done` workflow moves the card. Check
  that it did (`board.sh get <issue>`); if it didn't, move it and report that
  the workflow is off.
- **Wait for `main`'s own CI and read it.** A green PR does not guarantee a
  green `main`: a job that runs only on `main`, or a flake, can still fail
  there. If `main` is red, fixing it comes before anything else and does not
  need asking about.

  ```bash
  until [ "$(gh run list -R zx-sidekick/zx-sidekick --branch main --workflow ci.yml --limit 1 --json status -q '.[0].status')" = "completed" ]; do sleep 30; done
  gh run list -R zx-sidekick/zx-sidekick --branch main --workflow ci.yml --limit 1 --json conclusion,headSha
  ```

**Merging several:** one at a time, re-checking the next PR after each merge.
Merging one advances `main` and puts every other open PR behind it, which the
strict ruleset refuses: rebase the next one locally (Step 4), wait for its CI,
and re-read its label before merging it.

If `gh pr merge` is refused by the permission classifier while the label is
present and CI is green, re-read the label and retry once. If the label is
absent, that's a real stop.
