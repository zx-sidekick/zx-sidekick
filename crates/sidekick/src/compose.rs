//! What the window shows of a game being played, put together: the picture, the panel beside
//! it, the pins over the picture and the pause notice. The window draws these pieces in the
//! places and shades given here; `sidekick run` draws them into one picture.

use crate::{
    panel,
    picture::{layout, smallest, to_rgba, Layout, Part, FULL, LOGICAL_PIXEL, SCREEN},
};
use sidekick_core::Session;
use sidekick_def::{Border, Definition, Role};
use sidekick_ui::{Canvas, Face, Fonts, NoticeHint, PadLabels, PadMark, PauseNotice};

/// How bright the picture is, out of 255, under the pause notice or the guidance menu.
pub(crate) const DIMMED: u8 = 140;

/// Whether the game has guidance levels, and so the panel.
fn has_panel(definition: Option<&Definition>) -> bool {
    definition
        .and_then(|d| d.guidance.as_ref())
        .is_some_and(|g| !g.levels.is_empty())
}

/// Whether the game's definition leaves the border out.
fn borderless(definition: Option<&Definition>) -> bool {
    definition
        .and_then(|d| d.run.as_ref())
        .is_some_and(|r| r.border == Border::None)
}

/// The part of the machine's picture shown: the screen alone for a game whose definition
/// leaves the border out, once its tape has loaded; all of it otherwise.
pub(crate) fn shown(session: &Session) -> Part {
    if borderless(session.definition()) && !session.machine().tape_loading() {
        SCREEN
    } else {
        FULL
    }
}

/// Where the picture and, when the game has guidance levels, the panel go in a window `size`
/// device pixels large.
pub(crate) fn layout_for(session: &Session, size: (u32, u32)) -> Layout {
    let (_, _, w, h) = shown(session);
    layout(size, (w, h), has_panel(session.definition()))
}

/// The smallest window for a game, as it's played: without its border once the tape has
/// loaded, when its definition leaves the border out.
pub(crate) fn smallest_for(definition: Option<&Definition>) -> (u32, u32) {
    let (_, _, w, h) = if borderless(definition) { SCREEN } else { FULL };
    smallest((w, h), has_panel(definition))
}

/// The size of the part of the picture shown, in the logical pixels of what's drawn over it.
pub(crate) const fn logical((_, _, w, h): Part) -> (u32, u32) {
    (w * LOGICAL_PIXEL, h * LOGICAL_PIXEL)
}

/// Where a canvas `size` device pixels large goes to sit in the middle of `rect`.
pub(crate) const fn centred(
    (x, y, w, h): (u32, u32, u32, u32),
    size: (u32, u32),
) -> (u32, u32, u32, u32) {
    (
        x + w.saturating_sub(size.0) / 2,
        y + h.saturating_sub(size.1) / 2,
        size.0,
        size.1,
    )
}

/// Whether the game is in the state with `role`.
pub(crate) fn in_role(session: &Session, role: Role) -> bool {
    session
        .definition()
        .is_some_and(|d| d.is(session.state(), role))
}

/// The pause notice a definition words, for a pad marked as `labels` says.
pub(crate) fn pause_notice(definition: Option<&Definition>, labels: PadLabels) -> PauseNotice {
    let notice = definition
        .into_iter()
        .flat_map(|d| &d.states)
        .find(|s| s.role == Some(Role::Paused))
        .and_then(|s| s.notice.as_ref());
    let Some(notice) = notice else {
        return PauseNotice {
            pad: labels,
            ..PauseNotice::default()
        };
    };
    PauseNotice {
        text: notice.text.clone(),
        hints: notice
            .hints
            .iter()
            .map(|hint| NoticeHint {
                keys: hint.keys.clone(),
                pad: hint.pad.as_deref().and_then(|part| match part {
                    "west" => Some(PadMark::Face(Face::West)),
                    "south" => Some(PadMark::Face(Face::South)),
                    "east" => Some(PadMark::Face(Face::East)),
                    "north" => Some(PadMark::Face(Face::North)),
                    "dpad" | "stick" => Some(PadMark::Directions),
                    _ => None,
                }),
                does: hint.does.clone(),
            })
            .collect(),
        pad: labels,
    }
}

/// The window, `size` device pixels large, as the player sees a game at guidance `level` with
/// no menu or card open: the picture at the largest whole scale, the panel and its pins, and
/// the pause notice while the game is paused. RGBA, row by row.
pub(crate) fn window(
    session: &Session,
    level: usize,
    size: (u32, u32),
    labels: PadLabels,
    fonts: &mut Fonts,
) -> Vec<u8> {
    let mut out = Frame::new(size);
    let layout = layout_for(session, size);
    let part = shown(session);
    let paused = in_role(session, Role::Paused);
    let shade = if paused { DIMMED } else { 255 };
    let mut rgba = Vec::new();
    to_rgba(session.machine().picture(), &mut rgba);
    out.picture(&rgba, part, layout, shade);
    if let (Some(at), Some(panel)) = (layout.panel, panel::of(session, level, None)) {
        out.canvas(&panel.draw(fonts, (at.2, at.3)), at, 255);
        let pins = panel::pins(&panel);
        if !pins.is_empty() {
            let drawn = sidekick_ui::pins(&pins, fonts, layout.scale(), logical(part));
            out.canvas(&drawn, layout.picture, shade);
        }
    }
    if paused {
        let notice = pause_notice(session.definition(), labels).draw(fonts, layout.scale());
        let at = centred(layout.picture, notice.size());
        out.canvas(&notice, at, 255);
    }
    out.rgba
}

/// A picture being put together: black, RGBA.
struct Frame {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

impl Frame {
    fn new((width, height): (u32, u32)) -> Self {
        let mut rgba = vec![0; width as usize * height as usize * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        Self {
            rgba,
            width,
            height,
        }
    }

    /// The offset of a pixel, when it's in the frame.
    fn at(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| (y as usize * self.width as usize + x as usize) * 4)
    }

    /// The `part` of the Spectrum's picture (all of it in `rgba`, row by row) where `layout`
    /// puts it, each pixel `layout.pixel` square, its color times `shade` / 255.
    fn picture(&mut self, rgba: &[u8], part: Part, layout: Layout, shade: u8) {
        let (x, y, w, h) = layout.picture;
        let (left, top, _, _) = part;
        let scale = layout.pixel.max(1);
        let across = FULL.2 as usize;
        for dy in 0..h {
            for dx in 0..w {
                let source =
                    ((top + dy / scale) as usize * across + (left + dx / scale) as usize) * 4;
                let (Some(target), Some(pixel)) =
                    (self.at(x + dx, y + dy), rgba.get(source..source + 4))
                else {
                    continue;
                };
                for (k, &channel) in pixel.iter().take(3).enumerate() {
                    self.rgba[target + k] = mul(channel, shade);
                }
            }
        }
    }

    /// A canvas drawn over what's there at `rect`'s corner, its colors times `shade` / 255.
    fn canvas(&mut self, canvas: &Canvas, (x, y, _, _): (u32, u32, u32, u32), shade: u8) {
        let (w, h) = canvas.size();
        let pixels = canvas.straight_rgba();
        for dy in 0..h {
            for dx in 0..w {
                let Some(target) = self.at(x + dx, y + dy) else {
                    continue;
                };
                let source = (dy as usize * w as usize + dx as usize) * 4;
                let alpha = pixels[source + 3];
                for k in 0..3 {
                    let over = mul(mul(pixels[source + k], shade), alpha);
                    let under = mul(self.rgba[target + k], 255 - alpha);
                    self.rgba[target + k] = over.saturating_add(under);
                }
            }
        }
    }
}

/// `value` times `by` / 255.
fn mul(value: u8, by: u8) -> u8 {
    u8::try_from(u16::from(value) * u16::from(by) / 255).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidekick_core::{Machine, Model};
    use std::path::PathBuf;

    /// Where the stand-in game sits while paused: a loop the processor is parked in.
    const PAUSED_AT: u16 = 0x8000;
    const SIZE: (u32, u32) = (1368, 768);

    /// A game with guidance levels and a paused state, on a bare 48K whose screen is a plain
    /// stand-in: white paper, no ink.
    fn session() -> Session {
        session_with("", false)
    }

    /// The same game, with `run` as its `[run]` section, and with a tape that hasn't loaded
    /// when `loading`.
    fn session_with(run: &str, loading: bool) -> Session {
        let definition = Definition::from_toml(&format!(
            r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
            [run]
            {run}
            [[states]]
            name = "still"
            role = "paused"
            pc = {PAUSED_AT}
            notice = {{ text = "Paused." }}
            [[guidance.levels]]
            name = "Off"
            description = "Nothing."
            [[guidance.levels]]
            name = "Some"
            description = "A little."
            "#
        ))
        .expect("the test definition parses");
        let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K");
        for _ in 0..100 {
            machine.run_frame().expect("a frame");
        }
        for addr in 0x4000..0x5800 {
            machine.write(addr, 0);
        }
        for addr in 0x5800..0x5B00 {
            machine.write(addr, 0x38);
        }
        let mut s = Session::new(machine, Some(definition));
        s.run_frame().expect("a frame");
        if loading {
            // A tape put in, not a frame run since: not a block of it loaded.
            s.machine_mut()
                .load_tap(&[2, 0, 0xFF, 0xFF])
                .expect("a tape");
        }
        s
    }

    /// Inks the screen's four corner characters black, so its edges show against the border.
    fn mark_corners(s: &mut Session) {
        for row in [0_u16, 23] {
            for column in [0_u16, 31] {
                for line in 0..8 {
                    let addr = 0x4000 | ((row & 0x18) << 8) | (line << 8) | ((row & 7) << 5);
                    s.machine_mut().write(addr | column, 0xFF);
                }
            }
        }
        s.machine_mut().draw_screen_from_memory();
    }

    fn pause(s: &mut Session) {
        s.machine_mut().write(PAUSED_AT, 0x18);
        s.machine_mut().write(PAUSED_AT + 1, 0xFE);
        s.machine_mut().registers_mut().set_pc(PAUSED_AT);
        for _ in 0..2 {
            s.run_frame().expect("a frame");
        }
        assert!(in_role(s, Role::Paused));
    }

    fn pixel(rgba: &[u8], width: u32, (x, y): (u32, u32)) -> [u8; 3] {
        let at = (y as usize * width as usize + x as usize) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2]]
    }

    /// The middle of the part of the picture shown, as the window should show it.
    fn middle_of(session: &Session) -> [u8; 3] {
        let (x, y, w, h) = shown(session);
        let mut picture = Vec::new();
        to_rgba(session.machine().picture(), &mut picture);
        let at = ((y + h / 2) * FULL.2 + x + w / 2) as usize * 4;
        [picture[at], picture[at + 1], picture[at + 2]]
    }

    fn golden(name: &str, rgba: &[u8], (width, height): (u32, u32)) {
        let png = sidekick_ui::png::encode(rgba, width, height);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(name);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, &png).expect("the golden image writes");
            return;
        }
        let expected = std::fs::read(&path).unwrap_or_default();
        assert!(
            expected == png,
            "{name} differs from its golden image; look at it, then run with UPDATE_GOLDEN=1"
        );
    }

    #[test]
    fn the_window_is_the_picture_with_the_panel_beside_it() {
        let s = session();
        let rgba = window(&s, 1, SIZE, PadLabels::Letters, &mut Fonts::new());
        let layout = layout_for(&s, SIZE);
        assert_eq!(
            shown(&s),
            FULL,
            "a definition that says nothing keeps the border"
        );
        let (left, top, width, height) = layout.picture;
        assert_eq!(
            pixel(&rgba, SIZE.0, (left + width / 2, top + height / 2)),
            middle_of(&s),
            "the picture, scaled, where the layout puts it"
        );
        assert!(layout.panel.is_some());
        golden("window.png", &rgba, SIZE);
    }

    #[test]
    fn without_its_border_the_screen_alone_takes_a_larger_whole_scale() {
        const WIDE: (u32, u32) = (1920, 1080);
        let mut s = session_with(r#"border = "none""#, false);
        mark_corners(&mut s);
        assert_eq!(shown(&s), SCREEN);
        let layout = layout_for(&s, WIDE);
        assert_eq!(layout.pixel, 5);
        assert_eq!(layout.panel, Some((1346, 0, 574, 1080)));
        let rgba = window(&s, 1, WIDE, PadLabels::Letters, &mut Fonts::new());
        let (left, top, width, height) = layout.picture;
        assert_eq!(
            pixel(&rgba, WIDE.0, (left + width / 2, top + height / 2)),
            middle_of(&s)
        );
        assert_eq!(
            pixel(&rgba, WIDE.0, (left, top)),
            [0, 0, 0],
            "the screen's own top-left corner, inked, at the picture's corner"
        );
        assert_eq!(
            pixel(&rgba, WIDE.0, (left + width - 1, top + height - 1)),
            [0, 0, 0]
        );
        assert_eq!(smallest_for(s.definition()), (920, 768));
        golden("window-no-border.png", &rgba, WIDE);
    }

    #[test]
    fn while_its_tape_loads_the_border_shows() {
        let s = session_with(r#"border = "none""#, true);
        assert!(s.machine().tape_loading());
        assert_eq!(shown(&s), FULL);
        assert_eq!(layout_for(&s, (1920, 1080)).pixel, 4);
        assert_eq!(
            smallest_for(s.definition()),
            (920, 768),
            "the smallest window is the one the game is played in"
        );
        assert_eq!(smallest_for(session().definition()), (1048, 768));
    }

    #[test]
    fn paused_the_picture_dims_under_the_notice() {
        let mut s = session();
        let playing = window(&s, 1, SIZE, PadLabels::Letters, &mut Fonts::new());
        pause(&mut s);
        let paused = window(&s, 1, SIZE, PadLabels::Letters, &mut Fonts::new());
        let (x, y, _, _) = layout_for(&s, SIZE).picture;
        let corner = (x + 4, y + 4);
        assert_eq!(
            pixel(&paused, SIZE.0, corner),
            pixel(&playing, SIZE.0, corner).map(|c| mul(c, DIMMED))
        );
        golden("window-paused.png", &paused, SIZE);
    }
}
