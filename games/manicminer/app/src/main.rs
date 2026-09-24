//! ZX Sidekick for Manic Miner (#142): a window, keyboard and sound around an
//! emulated Spectrum running the player's own copy of the game.
//!
//! The machine runs on its own thread, a frame at a time. The main thread
//! runs the window: it draws the most recent frame and feeds keyboard state
//! to the machine.
//!
//! Usage: `zx-sidekick-manicminer [TAPE] [--headless FRAMES [SCREENSHOT_DIR [LEVEL]]]`

// As for Starquake: a release build on Windows opens no console behind the
// window, so anything fatal goes through `fatal` below.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod frontend;
mod panel;
mod picker;

use std::path::PathBuf;

use sidekick_frontend::cli::{fatal, headless_args};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let headless = headless_args(&mut args);
    let folders = sidekick_frontend::tape::folders(&frontend::GAME);
    let path = args
        .first()
        .map(PathBuf::from)
        .or_else(|| sidekick_frontend::tape::find(&frontend::GAME, &folders).map(|tape| tape.from));
    let windowed = headless.is_none();
    let result = match headless {
        Some((frames, dir, level)) => {
            let path = path.unwrap_or_else(|| {
                fatal(
                    &sidekick_frontend::tape::not_found_message(&frontend::GAME, &folders),
                    false,
                )
            });
            frontend::headless(&path, frames, &dir, level)
        }
        // In a window, no tape means asking for one.
        None => frontend::run(path.as_deref()),
    };
    if let Err(e) = result {
        fatal(&format!("error: {e}"), windowed);
    }
}
