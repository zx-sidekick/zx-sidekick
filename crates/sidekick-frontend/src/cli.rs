//! What every game's program does with its command line and with a problem
//! it cannot start past.

use std::path::PathBuf;

/// Reports a fatal startup problem somewhere it can actually be seen, and
/// gives up.
///
/// Started from a file manager, which is exactly how somebody who has just
/// unpacked the archive will start it, and exactly when they are most likely
/// to have forgotten the tape, the program has no console for `eprintln!`
/// to reach: on Windows a release build has none at all, and on macOS and
/// Linux there is none to look at. So the message goes to stderr and, when
/// the program is running as a window (`dialog`), into a message box too,
/// through the same dialog crate the screen that asks for the tape uses
/// (#77). A headless run is a terminal's, and a box there would wait for
/// nobody: CI's smoke test runs one from an empty folder and expects the
/// message and exit code 1, promptly.
pub fn fatal(message: &str, dialog: bool) -> ! {
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

/// Takes `--headless [FRAMES [DIR [LEVEL]]]` and everything after it out of
/// `args`, and returns the frames, the folder and the guidance level, with
/// their defaults. What is left in `args` is the tape, if one was named.
pub fn headless_args(args: &mut Vec<String>) -> Option<(u64, PathBuf, u8)> {
    let i = args.iter().position(|a| a == "--headless")?;
    let rest: Vec<String> = args.drain(i..).skip(1).collect();
    let frames = rest.first().and_then(|f| f.parse().ok()).unwrap_or(3000);
    let dir = rest
        .get(1)
        .map_or_else(|| PathBuf::from("screenshots"), PathBuf::from);
    let level = rest.get(2).and_then(|l| l.parse().ok()).unwrap_or(0);
    Some((frames, dir, level))
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
