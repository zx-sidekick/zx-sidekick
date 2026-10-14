//! What the window shows of a game being played, put together: the picture, the panel beside
//! it, the pins over the picture and the pause notice. The window draws these pieces in the
//! places and shades given here; `sidekick run` draws them into one picture.

use crate::{
    panel,
    picture::{layout, smallest, to_rgba, Layout, Part, FULL, LEAST_PIXEL, LOGICAL_PIXEL, SCREEN},
};
use sidekick_core::Session;
use sidekick_def::{Border, Definition, Role};
use sidekick_ui::{
    device, Canvas, Face, Fonts, NoticeHint, PadLabels, PadMark, PauseNotice, PAUSE_NOTICE_HEIGHT,
    PAUSE_NOTICE_WIDTH,
};

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

/// The room, in a card's logical pixels, kept between a card and the window's edge.
const MARGIN: u32 = 16;

/// The scale a card `logical` pixels large is drawn at: the picture's own third, as on the
/// mockups, but never larger than fits a window `output` device pixels large with a margin
/// all round.
#[allow(
    clippy::cast_precision_loss,
    reason = "screen sizes are far below f32's limit"
)]
pub(crate) fn card_scale(layout: Layout, output: (u32, u32), logical: (u32, u32)) -> f32 {
    let fit = |room: u32, length: u32| room as f32 / (length + 2 * MARGIN) as f32;
    layout
        .scale()
        .min(fit(output.0, logical.0))
        .min(fit(output.1, logical.1))
}

/// A card's place, `at`, moved inside the window's margin (at the card's `scale`) where it
/// would cross it.
pub(crate) fn within(
    (x, y, w, h): (u32, u32, u32, u32),
    output: (u32, u32),
    scale: f32,
) -> (u32, u32, u32, u32) {
    let margin = device(MARGIN, scale);
    let inside =
        |at: u32, length: u32, room: u32| at.min(room.saturating_sub(length + margin)).max(margin);
    (inside(x, w, output.0), inside(y, h, output.1), w, h)
}

/// Where a card `logical` pixels large goes, centred on `on`, and its scale: the picture's
/// third, made smaller to fit the window and kept inside it.
pub(crate) fn card_over(
    on: (u32, u32, u32, u32),
    output: (u32, u32),
    layout: Layout,
    logical: (u32, u32),
) -> ((u32, u32, u32, u32), f32) {
    let scale = card_scale(layout, output, logical);
    let size = (device(logical.0, scale), device(logical.1, scale));
    (within(centred(on, size), output, scale), scale)
}

/// Whether the game is in the state with `role`.
pub(crate) fn in_role(session: &Session, role: Role) -> bool {
    session
        .definition()
        .is_some_and(|d| d.is(session.state(), role))
}

/// Where a card `logical` pixels large goes at the foot of `on`, and its scale: as
/// [`card_over`], but resting a margin above the bottom of `on`, so the middle stays clear.
pub(crate) fn card_at_foot(
    on: (u32, u32, u32, u32),
    output: (u32, u32),
    layout: Layout,
    logical: (u32, u32),
) -> ((u32, u32, u32, u32), f32) {
    let ((x, _, w, h), scale) = card_over(on, output, layout, logical);
    let foot = (on.1 + on.3).saturating_sub(h + device(MARGIN, scale));
    (within((x, foot, w, h), output, scale), scale)
}

/// The top and foot, in device pixels, of the screen's character row just above `row`: the
/// line a card placed at `row` keeps in sight. `screen_top` is where the screen starts in the
/// part of the picture shown.
pub(crate) const fn line_above(layout: Layout, screen_top: u32, row: u32) -> (u32, u32) {
    let foot = screen_top + row * 8;
    let top = foot.saturating_sub(8);
    let (_, y, _, _) = layout.picture;
    (y + top * layout.pixel, y + foot * layout.pixel)
}

/// The smallest scale a card is shrunk to so it fits under a line: its scale beside the
/// smallest picture, where it's still read easily.
#[allow(clippy::cast_precision_loss, reason = "two small whole numbers")]
const LEAST_UNDER_LINE: f32 = LEAST_PIXEL as f32 / LOGICAL_PIXEL as f32;

/// Where a card `logical` pixels large goes so the picture's `line` (its top and foot, in device
/// pixels) stays in sight, and its scale. It sits just under the line, centred across the
/// picture, at the scale it has over the picture; where the window has no room for it there,
/// it's made smaller, down to [`LEAST_UNDER_LINE`]; past that it sits just above the line
/// instead, made smaller only if that has no room either.
#[allow(
    clippy::cast_precision_loss,
    reason = "screen sizes are far below f32's limit"
)]
pub(crate) fn card_under_line(
    line: (u32, u32),
    output: (u32, u32),
    layout: Layout,
    logical: (u32, u32),
) -> ((u32, u32, u32, u32), f32) {
    let usual = card_scale(layout, output, logical);
    let (px, _, pw, _) = layout.picture;
    let place = |y: u32, scale: f32| {
        let (w, h) = (device(logical.0, scale), device(logical.1, scale));
        let (x, _, _, _) = within((px + pw.saturating_sub(w) / 2, y, w, h), output, scale);
        ((x, y, w, h), scale)
    };
    // The most a card and its margin may take in `room` device pixels: one less than an even
    // share, since each is rounded to whole pixels.
    let fitting = |room: u32| room.saturating_sub(1) as f32 / (logical.1 + MARGIN) as f32;
    let below = output.1.saturating_sub(line.1);
    let fits_below = |scale: f32| device(logical.1, scale) + device(MARGIN, scale) <= below;
    if fits_below(usual) {
        return place(line.1, usual);
    }
    let shrunk = fitting(below).min(usual);
    if shrunk >= LEAST_UNDER_LINE {
        return place(line.1, shrunk);
    }
    let scale = usual.min(fitting(line.0));
    place(line.0.saturating_sub(device(logical.1, scale)), scale)
}

/// Whether the state the game was last known in has `role`: steady through frames that match
/// no state, as a title passed only every few frames does.
pub(crate) fn last_in_role(session: &Session, role: Role) -> bool {
    session
        .definition()
        .is_some_and(|d| d.is(session.progress().last_known.as_deref(), role))
}

/// The pause notice a definition words, for a pad marked as `labels` says.
pub(crate) fn pause_notice(definition: Option<&Definition>, labels: PadLabels) -> PauseNotice {
    notice_of(definition, Role::Paused, labels).unwrap_or_else(|| PauseNotice {
        pad: labels,
        ..PauseNotice::default()
    })
}

/// Frames a state seen only now and then still counts as there, for a notice: half a second.
/// A title passed every few frames keeps its card; one left for a screen that matches no
/// state loses it.
const STILL_THERE: u32 = 25;

/// The notice at the picture's foot, the picture not dimmed: on the loading screen, what goes
/// on; at the title, how the game plays, when the definition words it.
pub(crate) fn foot_notice(session: &Session, labels: PadLabels) -> Option<PauseNotice> {
    if session.on_loading_screen() {
        Some(loading_notice(labels))
    } else if last_in_role(session, Role::Title) && session.frames_unmatched() < STILL_THERE {
        notice_of(session.definition(), Role::Title, labels)
    } else {
        None
    }
}

/// The loading screen's notice: a key goes on loading.
fn loading_notice(labels: PadLabels) -> PauseNotice {
    PauseNotice {
        heading: "Loading screen".to_owned(),
        text: "The rest of the game loads once you press a key.".to_owned(),
        hints: vec![NoticeHint {
            keys: Some("Any key".to_owned()),
            pad: Some(PadMark::Start),
            does: "go on".to_owned(),
        }],
        pad: labels,
    }
}

/// A notice's keys as this computer names them: a Mac has no right Control, and its apps may
/// take the right Option tapped twice for a shortcut, so the right Ctrl that fires is named as
/// `/`, which fires too.
fn host_keys(keys: &str, mac: bool) -> String {
    if mac {
        keys.replace("Right Ctrl", "/")
    } else {
        keys.to_owned()
    }
}

/// The notice of the state with `role`, as the definition words it.
fn notice_of(
    definition: Option<&Definition>,
    role: Role,
    labels: PadLabels,
) -> Option<PauseNotice> {
    let notice = definition
        .into_iter()
        .flat_map(|d| &d.states)
        .find(|s| s.role == Some(role))
        .and_then(|s| s.notice.as_ref())?;
    let heading = notice.heading.clone().unwrap_or_else(|| {
        if role == Role::Title {
            "How it plays"
        } else {
            "Paused"
        }
        .to_owned()
    });
    Some(PauseNotice {
        heading,
        text: notice.text.clone(),
        hints: notice
            .hints
            .iter()
            .map(|hint| NoticeHint {
                keys: hint
                    .keys
                    .as_deref()
                    .map(|keys| host_keys(keys, cfg!(target_os = "macos"))),
                pad: hint.pad.as_deref().and_then(|part| match part {
                    "west" => Some(PadMark::Face(Face::West)),
                    "south" => Some(PadMark::Face(Face::South)),
                    "east" => Some(PadMark::Face(Face::East)),
                    "north" => Some(PadMark::Face(Face::North)),
                    "dpad" | "stick" => Some(PadMark::Directions),
                    "start" => Some(PadMark::Start),
                    _ => None,
                }),
                does: hint.does.clone(),
            })
            .collect(),
        pad: labels,
    })
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
        let (at, scale) = card_over(
            layout.picture,
            size,
            layout,
            (PAUSE_NOTICE_WIDTH, PAUSE_NOTICE_HEIGHT),
        );
        let notice = pause_notice(session.definition(), labels).draw(fonts, scale);
        out.canvas(&notice, at, 255);
    } else if let Some(notice) = foot_notice(session, labels) {
        let (at, scale) = card_at_foot(
            layout.picture,
            size,
            layout,
            (PAUSE_NOTICE_WIDTH, PAUSE_NOTICE_HEIGHT),
        );
        out.canvas(&notice.draw(fonts, scale), at, 255);
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
    #[test]
    fn a_mac_names_the_right_ctrl_as_the_slash() {
        assert_eq!(host_keys("Right Ctrl", true), "/");
        assert_eq!(host_keys("Right Ctrl", false), "Right Ctrl");
        assert_eq!(host_keys("0", true), "0");
    }

    /// Where the text-entry card goes in a window `size` large, under a borderless screen's
    /// row 19, and that row's top and foot.
    fn entry_card_by_row_19(size: (u32, u32)) -> ((u32, u32, u32, u32), f32, (u32, u32)) {
        let layout = layout(size, (256, 192), true);
        let (_, y, _, _) = layout.picture;
        let row_19 = (y + 152 * layout.pixel, y + 160 * layout.pixel);
        let line = line_above(layout, 0, 20);
        assert_eq!(line, row_19);
        let (at, scale) = card_under_line(line, size, layout, (ENTRY_WIDTH, ENTRY_HEIGHT));
        (at, scale, row_19)
    }

    #[test]
    fn the_entry_card_never_covers_the_line_above_its_row() {
        let sizes = [
            (1368, 768),
            (1440, 900),
            (1280, 720),
            (1920, 1080),
            (2560, 1440),
            (900, 1600),
            (700, 1000),
            (1100, 600),
        ];
        for size in sizes {
            let ((x, y, w, h), scale, (top, foot)) = entry_card_by_row_19(size);
            assert!(
                y + h <= top || y >= foot,
                "{size:?}: {y}..{} over {top}..{foot}",
                y + h
            );
            assert!(
                x + w <= size.0 && y + h <= size.1,
                "{size:?}: inside the window"
            );
            assert!(
                scale
                    >= LEAST_UNDER_LINE.min(card_scale(
                        layout(size, (256, 192), true),
                        size,
                        (ENTRY_WIDTH, ENTRY_HEIGHT)
                    ))
            );
        }
    }

    #[test]
    fn the_entry_card_sits_under_the_line_shrunk_where_it_must_and_above_it_past_that() {
        let ((_, y, _, _), scale, (_, foot)) = entry_card_by_row_19((1368, 768));
        assert_eq!(
            (y, scale),
            (foot, 1.0),
            "the mockup's size, just under the line"
        );
        for size in [(1280, 720), (1920, 1080), (2560, 1440)] {
            let usual = layout(size, (256, 192), true).scale();
            let ((_, y, _, _), scale, (_, foot)) = entry_card_by_row_19(size);
            assert_eq!(y, foot, "{size:?}: just under the line");
            assert!(scale < usual, "{size:?}: smaller, to fit");
        }
        let ((_, y, _, h), _, (top, _)) = entry_card_by_row_19((1100, 600));
        assert_eq!(y + h, top, "no room under it: just above the line");
    }

    #[test]
    fn the_title_s_notice_rests_at_the_picture_s_foot() {
        let layout = layout((1920, 1080), (256, 192), true);
        let picture = layout.picture;
        let ((x, y, w, h), _) = card_at_foot(
            picture,
            (1920, 1080),
            layout,
            (PAUSE_NOTICE_WIDTH, PAUSE_NOTICE_HEIGHT),
        );
        assert!(y + h < picture.1 + picture.3, "inside the picture's foot");
        assert!(y > picture.1 + picture.3 / 2, "in its lower half");
        assert_eq!(x + w / 2, picture.0 + picture.2 / 2, "centred across it");
    }

    use super::*;
    use sidekick_core::{Machine, Model};
    use sidekick_ui::{ENTRY_HEIGHT, ENTRY_WIDTH, MENU_WIDTH};
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

    /// Whether `rect` lies inside a window `output` large.
    const fn inside((x, y, w, h): (u32, u32, u32, u32), output: (u32, u32)) -> bool {
        x + w <= output.0 && y + h <= output.1
    }

    #[test]
    fn a_card_fits_a_narrow_window() {
        for output in [(920, 768), (1000, 1400), (919, 600), (1920, 1080)] {
            let laid = layout(output, (SCREEN.2, SCREEN.3), true);
            for height in [300, 560, 700] {
                let (menu, scale) = card_over(
                    (0, 0, output.0, output.1),
                    output,
                    laid,
                    (MENU_WIDTH, height),
                );
                assert!(inside(menu, output), "the menu at {output:?}: {menu:?}");
                assert!(menu.0 > 0 && menu.1 > 0, "with a margin: {menu:?}");
                assert!(scale <= laid.scale());
            }
            let (notice, _) = card_over(
                laid.picture,
                output,
                laid,
                (PAUSE_NOTICE_WIDTH, PAUSE_NOTICE_HEIGHT),
            );
            assert!(inside(notice, output), "the notice at {output:?}");
        }
    }

    #[test]
    fn a_card_takes_the_picture_s_third_where_the_window_has_room() {
        let output = (1920, 1080);
        let laid = layout(output, (SCREEN.2, SCREEN.3), true);
        let (menu, scale) = card_over((0, 0, 1920, 1080), output, laid, (MENU_WIDTH, 400));
        assert!((scale - 5.0 / 3.0).abs() < f32::EPSILON);
        assert_eq!(menu, (528, 206, 863, 667));
        let narrow = layout((920, 768), (SCREEN.2, SCREEN.3), true);
        let (menu, scale) = card_over((0, 0, 920, 768), (920, 768), narrow, (MENU_WIDTH, 400));
        assert!(
            (scale - 2.0 / 3.0).abs() < f32::EPSILON,
            "the picture's third fits"
        );
        assert_eq!((menu.2, menu.3), (345, 267));
        let (notice, _) = card_over(
            narrow.picture,
            (920, 768),
            narrow,
            (PAUSE_NOTICE_WIDTH, PAUSE_NOTICE_HEIGHT),
        );
        assert_eq!(notice.0, 110, "centred on the picture");
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
