---
name: mockup
description: >
  Use whenever visual or looks-driven work needs its pre-approval mockup: "make
  a mockup of X", "show me what it would look like", "design the screen
  first", or from inside a spec whose value is how it LOOKS (a new screen, a
  layout change, colours, typography, animation). Produces a screenshot,
  commits it under docs/mockups/ on the work branch, and embeds it in the
  ticket. The maintainer
  approves the picture BEFORE any real UI is built. Trigger before writing any
  UI code for looks-driven work, even if nobody asked for a mockup.
---

Visual work gets a mockup approved **before** the real UI exists. You produce
the mockup, the screenshot and the embed; the maintainer's yes to the picture
is part of the spec OK.

## Step 1: build it

Pick whichever shows the design fastest and most faithfully:

- **A real screenshot**, which this project can produce without a window:

  ```bash
  cargo run --release -p zx-sidekick-starquake -- "$SK_ASSETS/starquake.tap" \
    --headless 4000 docs/mockups/shots
  ```

  What the Spectrum draws is the original game's, running unchanged, and is
  never ours to redesign (`GOAL.md`). A screenshot shows it as it is, so a
  change to what the window draws around or over it can be seen in place:
  256×192 with its border, at the size the window scales it to.

  **Screenshots of the game are fine to commit**, original graphics and all
  (the maintainer's call on starquake-recompiled, carried over here). The no-game-data invariant is about the files the
  game is loaded from: tapes, snapshots, ROMs, and graphics, maps or text
  extracted from them as files, which CI's guard job looks for. A picture of
  the screen for a design review is not one of those. Keep to the frames the
  design needs, not a gallery of the game.
- **An HTML mockup** in the scratchpad, for the window's own screens that
  are not built yet — the tape prompt, a panel beside the picture, a settings
  sheet. Say which it is, so nobody reads a host-side sketch as something the
  1985 machine would draw.

**Look at the PNG yourself before posting it**, to catch a blank or clipped
render.

## Step 2: commit the image

`docs/mockups/<YYYY-MM-DD>-<name>.png`, on the **work branch** (never straight
to `main`). The repo is the image host, since GitHub has no upload API for
issue attachments.

## Step 3: embed with exactly this URL form

```markdown
![mockup](https://github.com/zx-sidekick/zx-sidekick-starquake/raw/<branch>/docs/mockups/<file>.png)
```

This repo is public, so `raw.githubusercontent.com` would also render. Use the
`github.com/…/raw/…` form anyway: it is the one that keeps working if the repo
is ever made private again, and `github.com/…/blob/…` is a click-through link
rather than an embed.

Put it in the ticket's *Mockup* section. If you post it in a comment instead,
the comment opens with the 🤖 attribution line.

**The embed must end up pointing at `main`.** A `/raw/<branch>/` embed dies
when the branch is deleted on merge, silently and retroactively. The PR that
merges the image repoints the embed to `/raw/main/…` (`build-slice`, Finish).
Not a commit SHA: PRs are squash-merged, so the branch's commits aren't in
`main`'s history.

## Step 4: STOP for approval

Move the card to `Your sign-off` and post a Next-steps comment asking for a
yes, or for changes via an answer block. No real UI code before that. To
iterate, re-render to the same filename on the same branch: the embed then
shows the new version.
