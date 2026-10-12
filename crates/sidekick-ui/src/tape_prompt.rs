//! The tape prompt: shown when a game's tape isn't found (`docs/mockups/starquake/tape-prompt.png`).

use crate::{colors, Align, Canvas, Color, Fonts, Line, Style, Weight};

/// The prompt's size, in logical pixels.
const WIDTH: u32 = 960;
/// The prompt's size, in logical pixels.
const HEIGHT: u32 = 768;

/// What the tape prompt says, from the game's definition.
#[derive(Clone, Debug)]
pub struct TapePrompt {
    /// The game's title.
    pub title: String,
    /// The credit and notice along the bottom.
    pub credit: String,
    /// The tape's file name, for the hint under the button.
    pub tape_file: String,
    /// The download's file name, for "Download … there".
    pub download_file: Option<String>,
    /// Where the tape can be downloaded: the page's name and its address.
    pub download: Option<(String, String)>,
    /// A tape that was offered and isn't the one needed.
    pub wrong: Option<WrongDump>,
}

/// A tape that isn't the dump a game needs.
#[derive(Clone, Debug)]
pub struct WrongDump {
    /// The file it came in.
    pub file: String,
    /// The dump that is needed, in words.
    pub wanted: String,
    /// The start of that dump's SHA-1.
    pub sha1: String,
}

const LABEL: Style = Style::new(11.0, colors::LABEL)
    .weight(Weight::Medium)
    .tracking(0.12);
const ERROR_BG: Color = Color::rgb(0x2A, 0x14, 0x16);
const ERROR_EDGE: Color = Color::rgb(0x6A, 0x2A, 0x2E);
const ERROR_TEXT: Color = Color::rgb(0xD9, 0xB4, 0xB6);
const DROP_BG: Color = Color::rgb(0x10, 0x12, 0x1A);

impl TapePrompt {
    /// Draws the prompt at `scale` device pixels to a logical one.
    #[must_use]
    pub fn draw(&self, fonts: &mut Fonts, scale: u32) -> Canvas {
        let mut c = Canvas::new(WIDTH, HEIGHT, scale);
        c.fill(colors::PROMPT);
        let label_height = Fonts::normal_line_height(&LABEL);
        c.text(
            fonts,
            &self.title.to_uppercase(),
            (120.0, 97.0),
            label_height,
            Align::Left,
            &LABEL,
        );
        let heading = Style::new(31.0, colors::TEXT).weight(Weight::SemiBold);
        c.text(
            fonts,
            "This needs your copy of the game",
            (120.0, 118.0),
            Fonts::normal_line_height(&heading),
            Align::Left,
            &heading,
        );
        let body = Style::new(16.0, colors::BODY);
        c.paragraph(
            fonts,
            &format!(
                "Nothing from the original game is included. {} runs from your own tape, here in an emulated Spectrum, each time it starts.",
                self.title
            ),
            (120.0, 167.0),
            660.0,
            24.0,
            &body,
        );

        let y = self.wrong.as_ref().map_or(250.0, |wrong| {
            self.wrong_dump(&mut c, fonts, wrong);
            336.0
        });
        self.drop_zone(&mut c, fonts, y);
        if let Some((page, address)) = &self.download {
            self.download(&mut c, fonts, y + 214.0, page, address);
        }

        let small = Style::new(12.0, colors::FAINT);
        c.text(
            fonts,
            &self.credit,
            (120.0, 711.0),
            18.0,
            Align::Left,
            &small,
        );
        hints(
            &mut c,
            fonts,
            (840.0, 709.0),
            &[("Enter", "locate"), ("W", "website"), ("Esc", "quit")],
        );
        c
    }

    fn wrong_dump(&self, c: &mut Canvas, fonts: &mut Fonts, wrong: &WrongDump) {
        c.rect(
            (120.0, 236.0, 720.0, 90.0),
            8.0,
            Some(ERROR_BG),
            Some(Line::solid(ERROR_EDGE, 1.0)),
        );
        c.circle((147.0, 261.0), 9.0, Some(colors::RED), None);
        let bang = Style::new(12.0, ERROR_BG).weight(Weight::Bold);
        c.text(fonts, "!", (147.0, 252.0), 18.0, Align::Center, &bang);
        let heading = Style::new(15.0, colors::TEXT).weight(Weight::SemiBold);
        c.text(
            fonts,
            &format!("That isn't the {} tape this version needs", self.title),
            (166.0, 249.0),
            Fonts::normal_line_height(&heading),
            Align::Left,
            &heading,
        );
        c.paragraph(
            fonts,
            &format!(
                "The tape in {} is a different dump. This version works only with {} (SHA-1 {}…).",
                wrong.file, wrong.wanted, wrong.sha1
            ),
            (166.0, 273.0),
            650.0,
            20.0,
            &Style::new(13.0, ERROR_TEXT),
        );
    }

    fn drop_zone(&self, c: &mut Canvas, fonts: &mut Fonts, y: f32) {
        c.rect(
            (120.0, y, 720.0, 164.0),
            12.0,
            Some(DROP_BG),
            Some(Line {
                color: colors::EDGE,
                width: 1.5,
                dash: Some((6.0, 4.0)),
            }),
        );
        c.rect(
            (383.0, y + 36.0, 194.0, 44.0),
            6.0,
            Some(colors::ACCENT),
            None,
        );
        let button = Style::new(16.0, colors::PROMPT).weight(Weight::SemiBold);
        c.text(
            fonts,
            "Locate the tape...",
            (480.0, y + 36.0),
            44.0,
            Align::Center,
            &button,
        );
        let or = Style::new(15.0, colors::MUTED);
        c.text(
            fonts,
            "or drop it onto this window",
            (480.0, y + 98.0),
            Fonts::normal_line_height(&or),
            Align::Center,
            &or,
        );
        let hint = Style::new(12.0, colors::FAINT);
        let strong = Style::new(12.0, colors::BODY);
        spans(
            c,
            fonts,
            (480.0, y + 126.0),
            &[
                (self.tape_file.as_str(), strong),
                (", or the ", hint),
                (".zip", strong),
                (" it was downloaded in, in any upper or lower case", hint),
            ],
        );
    }

    fn download(&self, c: &mut Canvas, fonts: &mut Fonts, d: f32, page: &str, address: &str) {
        let label_height = Fonts::normal_line_height(&LABEL);
        c.text(
            fonts,
            "DON'T HAVE IT?",
            (120.0, d),
            label_height,
            Align::Left,
            &LABEL,
        );
        let file = self.download_file.as_deref().map_or_else(String::new, |f| {
            format!(" Download {f} there, then locate it here.")
        });
        c.paragraph(
            fonts,
            &format!("{page} keeps Spectrum software available, and removes titles whose rights holders object.{file}"),
            (120.0, d + 22.0),
            490.0,
            22.0,
            &Style::new(15.0, colors::BODY),
        );
        let url = Style::new(13.0, colors::FAINT);
        c.text(
            fonts,
            address,
            (120.0, d + 94.0),
            Fonts::normal_line_height(&url),
            Align::Left,
            &url,
        );
        c.rect(
            (616.0, d + 36.0, 224.0, 38.0),
            6.0,
            Some(colors::CARD),
            Some(Line::solid(colors::EDGE, 1.0)),
        );
        let open = Style::new(15.0, colors::TEXT).weight(Weight::Medium);
        c.text(
            fonts,
            &format!("Open {page} ↗"),
            (728.0, d + 37.0),
            36.0,
            Align::Center,
            &open,
        );
    }
}

/// Runs of differently styled text on one line, centred on `x`.
fn spans(c: &mut Canvas, fonts: &mut Fonts, (x, y): (f32, f32), runs: &[(&str, Style)]) {
    let width: f32 = runs
        .iter()
        .map(|(text, style)| fonts.measure(text, style))
        .sum();
    let mut left = x - width / 2.0;
    for (text, style) in runs {
        left += c.text(fonts, text, (left, y), 16.0, Align::Left, style);
    }
}

/// Key hints along the bottom, right-aligned on `x`: each key in a squarish badge, then what it
/// does.
fn hints(c: &mut Canvas, fonts: &mut Fonts, (right, y): (f32, f32), items: &[(&str, &str)]) {
    let key = Style::new(12.0, colors::TEXT).weight(Weight::SemiBold);
    let what = Style::new(12.0, colors::MUTED);
    let widths: Vec<(f32, f32)> = items
        .iter()
        .map(|(k, w)| (fonts.measure(k, &key) + 10.0, fonts.measure(w, &what)))
        .collect();
    let gaps: f32 = widths.iter().skip(1).map(|_| 14.0).sum();
    let total: f32 = widths.iter().map(|(k, w)| k + 6.0 + w).sum::<f32>() + gaps;
    let mut x = right - total;
    for ((k, w), (kw, ww)) in items.iter().zip(widths) {
        c.rect(
            (x, y, kw, 20.0),
            3.0,
            None,
            Some(Line::solid(colors::KEY_EDGE, 1.0)),
        );
        c.text(fonts, k, (x + kw / 2.0, y + 1.0), 18.0, Align::Center, &key);
        x += kw + 6.0;
        c.text(fonts, w, (x, y + 1.0), 18.0, Align::Left, &what);
        x += ww + 14.0;
    }
}
