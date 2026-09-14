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

/// Reports a fatal startup problem somewhere it can actually be seen, and
/// gives up.
///
/// On Windows a release build has no console, so `eprintln!` goes nowhere
/// when the program is started by double-clicking it — which is exactly how
/// somebody who has just unpacked the archive will start it, and exactly
/// when they are most likely to have forgotten the tape. A message box is
/// the only place that sentence can land. Everywhere else, stderr is right.
fn fatal(message: &str) -> ! {
    eprintln!("{message}");
    #[cfg(all(windows, not(debug_assertions)))]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }
        let (text, title) = (wide(message), wide("ZX Sidekick"));
        // SAFETY: both strings are NUL-terminated and outlive the call, and
        // a null window handle is what MessageBoxW wants for an owner-less
        // box.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    std::process::exit(1)
}

/// Takes `--headless [FRAMES [DIR [LEVEL]]]` and everything after it out of
/// `args`, and returns the frames, the folder and the guidance level, with
/// their defaults. What is left in `args` is the tape, if one was named.
fn headless_args(args: &mut Vec<String>) -> Option<(u64, PathBuf, u8)> {
    let i = args.iter().position(|a| a == "--headless")?;
    let rest: Vec<String> = args.drain(i..).skip(1).collect();
    let frames = rest.first().and_then(|f| f.parse().ok()).unwrap_or(3000);
    let dir = rest
        .get(1)
        .map_or_else(|| PathBuf::from("screenshots"), PathBuf::from);
    let level = rest.get(2).and_then(|l| l.parse().ok()).unwrap_or(0);
    Some((frames, dir, level))
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let headless = headless_args(&mut args);
    // A tape named on the command line is used as it is; otherwise the usual
    // places are searched.
    let folders = frontend::tape::folders();
    let path = args.first().map(PathBuf::from).or_else(|| {
        frontend::tape::find(&folders, sidekick::starquake::is_supported_tape).map(|tape| tape.from)
    });
    // Without a window there is nobody to ask, so no tape is the end.
    let required = || {
        path.clone()
            .unwrap_or_else(|| fatal(&frontend::tape::not_found_message(&folders)))
    };
    let result = match headless {
        Some((frames, dir, level)) => frontend::headless::run(&required(), frames, &dir, level),
        // In a window, no tape means asking for one.
        None => frontend::run(path.as_deref()),
    };
    if let Err(e) = result {
        fatal(&format!("error: {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn no_headless_flag_leaves_the_arguments_alone() {
        let mut a = args(&["game.tap"]);
        assert_eq!(headless_args(&mut a), None);
        assert_eq!(a, ["game.tap"]);
    }

    #[test]
    fn headless_takes_its_frames_and_folder_and_leaves_the_tape() {
        let mut a = args(&["game.tap", "--headless", "120", "out"]);
        assert_eq!(headless_args(&mut a), Some((120, PathBuf::from("out"), 0)));
        assert_eq!(a, ["game.tap"]);
        let mut a = args(&["game.tap", "--headless", "120", "out", "3"]);
        assert_eq!(headless_args(&mut a), Some((120, PathBuf::from("out"), 3)));
        assert_eq!(a, ["game.tap"]);
    }

    #[test]
    fn headless_defaults_its_frames_and_folder() {
        let mut a = args(&["--headless"]);
        assert_eq!(
            headless_args(&mut a),
            Some((3000, PathBuf::from("screenshots"), 0))
        );
        assert!(a.is_empty());
        let mut a = args(&["--headless", "lots"]);
        assert_eq!(
            headless_args(&mut a),
            Some((3000, PathBuf::from("screenshots"), 0))
        );
    }
}
