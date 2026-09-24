//! ZX Sidekick for Starquake: a window, keyboard and sound around an
//! emulated Spectrum running the player's own copy of the game.
//!
//! The machine runs on its own thread, a frame at a time. The main thread
//! runs the window: it draws the most recent frame and feeds keyboard state
//! to the machine.
//!
//! Usage: `zx-sidekick-starquake [TAPE] [--headless FRAMES [SCREENSHOT_DIR [LEVEL]]]`

// Every Rust program links as a console application, which on Windows means
// a command prompt opens behind the game window. A release build asks for
// the windows subsystem instead so that it does not. Debug builds keep the
// console, since that is where anyone debugging wants the output.
//
// The cost is that `eprintln!` reaches nobody when the program is started
// from a file manager, so anything fatal goes through `fatal` below.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod frontend;

use std::path::PathBuf;

use sidekick_frontend::cli::{fatal, headless_args};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let headless = headless_args(&mut args);
    // A tape named on the command line is used as it is; otherwise the usual
    // places are searched.
    let folders = sidekick_frontend::tape::folders(&frontend::GAME);
    let path = args
        .first()
        .map(PathBuf::from)
        .or_else(|| sidekick_frontend::tape::find(&frontend::GAME, &folders).map(|tape| tape.from));
    // Without a window there is nobody to ask, so no tape is the end: said
    // on the terminal, which is where a headless run is watched from.
    let required = || {
        path.clone().unwrap_or_else(|| {
            fatal(
                &sidekick_frontend::tape::not_found_message(&frontend::GAME, &folders),
                false,
            )
        })
    };
    let windowed = headless.is_none();
    let result = match headless {
        Some((frames, dir, level)) => frontend::headless::run(&required(), frames, &dir, level),
        // In a window, no tape means asking for one.
        None => frontend::run(path.as_deref()),
    };
    if let Err(e) = result {
        fatal(&format!("error: {e}"), windowed);
    }
}
