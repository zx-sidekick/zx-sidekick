//! What the window shows of a game being played, put together: the picture, the panel beside
//! it, the pins over the picture and the pause notice. The window draws these pieces in the
//! places and shades given here; `sidekick run` draws them into one picture.

use crate::{
    panel,
    picture::{layout, to_rgba, Layout},
};
use sidekick_core::Session;
use sidekick_def::{Definition, Role};
use sidekick_ui::{
    Canvas, Face, Fonts, NoticeHint, PadLabels, PadMark, PauseNotice, PAUSE_NOTICE_HEIGHT,
    PAUSE_NOTICE_WIDTH,
};

/// The picture's size: the screen with 32 pixels of border all round.
pub(crate) const PICTURE: (u32, u32) = (320, 256);

/// How bright the picture is, out of 255, under the pause notice or the guidance menu.
pub(crate) const DIMMED: u8 = 140;

/// Where the picture and, when the game has guidance levels, the panel go in a window `size`
/// device pixels large.
pub(crate) fn layout_for(definition: Option<&Definition>, size: (u32, u32)) -> Layout {
    let panel = definition
        .and_then(|d| d.guidance.as_ref())
        .is_some_and(|g| !g.levels.is_empty());
    layout(size, PICTURE, panel)
}

/// Whether the game is in the state with `role`.
pub(crate) fn in_role(session: &Session, role: Role) -> bool {
    session
        .definition()
        .is_some_and(|d| d.is(session.state(), role))
}

/// Where the pause notice goes: in the middle of the picture.
pub(crate) const fn notice_at(layout: Layout) -> (u32, u32, u32, u32) {
    let (px, py, pw, ph) = layout.picture;
    let (w, h) = (
        PAUSE_NOTICE_WIDTH * layout.scale,
        PAUSE_NOTICE_HEIGHT * layout.scale,
    );
    (px + (pw - w) / 2, py + (ph - h) / 2, w, h)
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
    let layout = layout_for(session.definition(), size);
    let paused = in_role(session, Role::Paused);
    let shade = if paused { DIMMED } else { 255 };
    let mut rgba = Vec::new();
    to_rgba(session.machine().picture(), &mut rgba);
    out.picture(&rgba, layout.picture, shade);
    if let (Some(at), Some(panel)) = (layout.panel, panel::of(session, level, None)) {
        out.canvas(&panel.draw(fonts, layout.scale), at, 255);
        let pins = panel::pins(&panel);
        if !pins.is_empty() {
            let drawn = sidekick_ui::pins(&pins, fonts, layout.scale);
            out.canvas(&drawn, layout.picture, shade);
        }
    }
    if paused {
        let notice = pause_notice(session.definition(), labels);
        out.canvas(&notice.draw(fonts, layout.scale), notice_at(layout), 255);
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

    /// The Spectrum's picture (`PICTURE` large, RGBA), stretched by a whole number to fill
    /// `rect`, each pixel's color times `shade` / 255.
    fn picture(&mut self, rgba: &[u8], (x, y, w, h): (u32, u32, u32, u32), shade: u8) {
        let scale = (w / PICTURE.0).max(1);
        for dy in 0..h {
            for dx in 0..w {
                let source =
                    ((dy / scale) as usize * PICTURE.0 as usize + (dx / scale) as usize) * 4;
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
        let definition = Definition::from_toml(&format!(
            r#"
            schema = 1
            [game]
            id = "test"
            title = "Test"
            machine = "48k"
            [[media]]
            name = "the tape"
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
        s
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

    fn pixel(rgba: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
        let at = (y as usize * SIZE.0 as usize + x as usize) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2]]
    }

    fn golden(name: &str, rgba: &[u8]) {
        let png = sidekick_ui::png::encode(rgba, SIZE.0, SIZE.1);
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
        let layout = layout_for(s.definition(), SIZE);
        let (left, top, width, height) = layout.picture;
        let mut picture = Vec::new();
        to_rgba(s.machine().picture(), &mut picture);
        let middle = (PICTURE.1 / 2 * PICTURE.0 + PICTURE.0 / 2) as usize * 4;
        assert_eq!(
            pixel(&rgba, (left + width / 2, top + height / 2)),
            [picture[middle], picture[middle + 1], picture[middle + 2]],
            "the picture, scaled, where the layout puts it"
        );
        assert!(layout.panel.is_some());
        golden("window.png", &rgba);
    }

    #[test]
    fn paused_the_picture_dims_under_the_notice() {
        let mut s = session();
        let playing = window(&s, 1, SIZE, PadLabels::Letters, &mut Fonts::new());
        pause(&mut s);
        let paused = window(&s, 1, SIZE, PadLabels::Letters, &mut Fonts::new());
        let (x, y, _, _) = layout_for(s.definition(), SIZE).picture;
        let corner = (x + 4, y + 4);
        assert_eq!(
            pixel(&paused, corner),
            pixel(&playing, corner).map(|c| mul(c, DIMMED))
        );
        golden("window-paused.png", &paused);
    }
}
