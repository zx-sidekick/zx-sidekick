//! The guidance panel beside the game, the picker, and the note of how much
//! help a game had (#25), drawn to the approved mockups; and, over the
//! picture, the pause notice.
//!
//! Everything is drawn in the overlay's layout units (`overlay.rs`): the
//! picture takes the left `PICTURE_W`, the panel the rest.

use super::guidance::Guidance;
use super::notice;
use super::overlay::{HEIGHT, PICTURE_W, WIDTH};
use super::text::{Canvas, Fonts, Rgb};
use super::track::Scene;

const PANEL: Rgb = [0x0f, 0x11, 0x17];
const RULE: Rgb = [0x22, 0x26, 0x2f];

pub struct Panel {
    fonts: Fonts,
}

impl Panel {
    pub fn new() -> Panel {
        Panel {
            fonts: Fonts::load(),
        }
    }

    /// Draws the overlay: the panel, the pause notice over the picture when
    /// the game is `paused`, and the picker over everything when it is open.
    pub fn draw(&mut self, canvas: &mut Canvas, guidance: &Guidance, _scene: Scene, paused: bool) {
        canvas.round_rect(PICTURE_W, 0.0, WIDTH - PICTURE_W, HEIGHT, 0.0, PANEL);
        canvas.round_rect(PICTURE_W, 0.0, 1.0, HEIGHT, 0.0, RULE);
        if paused && !guidance.picker_open() {
            notice::draw(&mut self.fonts, canvas);
        }
    }
}
