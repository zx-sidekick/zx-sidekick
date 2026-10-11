---
name: mockup
description: >
  Use whenever visual or looks-driven ZX Sidekick work needs its pre-approval
  mockup: "make a mockup of X", "show me what it would look like", "design the
  screen first", or from inside a spec whose value is how it LOOKS (a panel
  widget, a menu, a card, a notice, the window's layout, colours,
  typography). Produces a headless render of the program's drawing (a
  screenshot, with a game running behind it, when that shows it best) or a
  sketch for UI that doesn't exist yet, commits it under docs/mockups/ on the
  work branch, and embeds it in the ticket. The maintainer approves the
  picture BEFORE any real UI is built. Trigger before writing any UI code for
  looks-driven work, even if nobody asked for a mockup.
---

Visual work gets a mockup approved **before** the real UI exists. You produce
the mockup, the image and the embed; the maintainer's yes to the picture is
part of the spec OK.

## The reference look

The screenshots in `docs/mockups/starquake/` are the reference design:
layout, wording and behaviour, for every game (`ZX-SIDEKICK.md`, "Mockups",
lists them). Match them pixel for pixel: the game's own font and icons at
whole multiples of a Spectrum pixel, Inter for the program's own text.

**Screenshots are allowed** (CLAUDE.md, rule 3): a picture of a game running,
as a player sees it, may be committed and embedded. Graphics, fonts, maps or
sprite sheets extracted as files may not.

## Step 1: build it

Pick whichever shows the design fastest and most faithfully.

- **A headless render of the program's drawing**, the default. `sidekick-ui`
  draws the panel, menus, cards and notices in software into an RGBA buffer,
  the same path the golden-image tests use, so render the window into a PNG
  at its 1368×768. With the player's tape in `assets/`, the game can run
  behind it, which makes it a screenshot; without one, use a plain stand-in.
  A change can be seen in place: a throwaway edit on the work branch (a
  colour, a widget placed by hand), rendered headless, never kept as the
  build and never committed.
- **A sketch**, only for UI that doesn't exist yet and can't be rendered: the
  library screen, the menu for a game without a definition. Draw it as an
  image (HTML or SVG in the scratchpad, rendered to PNG; the source is never
  committed), in the reference look. Say which it is, so nobody reads a
  sketch as the program.

**Look at the PNG yourself before posting it**, to catch a blank or clipped
render.

## Step 2: commit the image

`docs/mockups/<YYYY-MM-DD>-<name>.png`, on the **work branch** (never straight
to `main`), staged by its explicit path. The repo is the image host, since
GitHub has no upload API for issue attachments.

## Step 3: embed with exactly this URL form

```markdown
![mockup](https://github.com/zx-sidekick/zx-sidekick/raw/<branch>/docs/mockups/<file>.png)
```

Use the `github.com/…/raw/…` form, not `raw.githubusercontent.com`: it is the
one that keeps rendering whether the repo is public or private, and
`github.com/…/blob/…` is a click-through link rather than an embed.

Put it in the ticket's *Mockup* section. If you post it in a comment instead,
the comment opens with the 🤖 "Comment by Claude" attribution line.

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
