//! ZX Sidekick for Manic Miner (#142): a window, keyboard and sound around an
//! emulated Spectrum running the player's own copy of the game.
//!
//! The machine runs on its own thread, a frame at a time. The main thread
//! runs the window: it draws the most recent frame and feeds keyboard state
//! to the machine.
//!
//! Usage: `zx-sidekick-manicminer [TAPE] [--headless FRAMES [SCREENSHOT_DIR]]`

// As for Starquake: a release build on Windows opens no console behind the
// window, so anything fatal goes through `fatal` below.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod frontend;
mod picker;

use std::path::PathBuf;

/// Reports a fatal startup problem on stderr and, in a window, in a message
/// box too, since a program started from a file manager has no console.
fn fatal(message: &str, dialog: bool) -> ! {
    eprintln!("{message}");
    if dialog {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("ZX Sidekick")
            .set_description(message)
            .show();
    }
    std::process::exit(1)
}

/// Takes `--headless [FRAMES [DIR]]` and everything after it out of `args`,
/// and returns the frames and the folder, with their defaults. What is left
/// in `args` is the tape, if one was named.
fn headless_args(args: &mut Vec<String>) -> Option<(u64, PathBuf)> {
    let i = args.iter().position(|a| a == "--headless")?;
    let rest: Vec<String> = args.drain(i..).skip(1).collect();
    let frames = rest.first().and_then(|f| f.parse().ok()).unwrap_or(3000);
    let dir = rest
        .get(1)
        .map_or_else(|| PathBuf::from("screenshots"), PathBuf::from);
    Some((frames, dir))
}

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
        Some((frames, dir)) => {
            let path = path.unwrap_or_else(|| {
                fatal(
                    &sidekick_frontend::tape::not_found_message(&frontend::GAME, &folders),
                    false,
                )
            });
            frontend::headless(&path, frames, &dir)
        }
        // In a window, no tape means asking for one.
        None => frontend::run(path.as_deref()),
    };
    if let Err(e) = result {
        fatal(&format!("error: {e}"), windowed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn headless_takes_its_frames_and_folder_and_leaves_the_tape() {
        let mut a = args(&["manic.tap", "--headless", "120", "out"]);
        assert_eq!(headless_args(&mut a), Some((120, PathBuf::from("out"))));
        assert_eq!(a, ["manic.tap"]);
        let mut a = args(&["manic.tap"]);
        assert_eq!(headless_args(&mut a), None);
        let mut a = args(&["--headless"]);
        assert_eq!(
            headless_args(&mut a),
            Some((3000, PathBuf::from("screenshots")))
        );
    }
}
