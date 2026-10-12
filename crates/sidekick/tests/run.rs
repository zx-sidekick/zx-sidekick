//! `sidekick run`: a tape run without a window, its picture written as a PNG.

use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

/// A `.tap` block: its length, then the flag, the data and their XOR.
fn block(flag: u8, data: &[u8]) -> Vec<u8> {
    let checksum = data.iter().fold(flag, |x, b| x ^ b);
    let len = u16::try_from(data.len() + 2).expect("a short block");
    let mut out = len.to_le_bytes().to_vec();
    out.push(flag);
    out.extend_from_slice(data);
    out.push(checksum);
    out
}

/// `10 PAUSE 0: BORDER 2`, auto-running: waits for a key, then turns the border red. A
/// program of our own, with the numbers' hidden five-byte forms.
fn wait_then_red() -> Vec<u8> {
    let small = |n: u8| [0x0E, 0x00, 0x00, n, 0x00, 0x00];
    let mut line = vec![0xF2, b'0']; // PAUSE 0
    line.extend_from_slice(&small(0));
    line.extend_from_slice(&[b':', 0xE7, b'2']); // : BORDER 2
    line.extend_from_slice(&small(2));
    line.push(0x0D);
    let mut program = vec![0x00, 0x0A];
    program.extend_from_slice(&u16::try_from(line.len()).expect("short").to_le_bytes());
    program.extend_from_slice(&line);
    let len = u16::try_from(program.len()).expect("short").to_le_bytes();
    let mut header = vec![0x00];
    header.extend_from_slice(b"wait      ");
    header.extend_from_slice(&len);
    header.extend_from_slice(&10u16.to_le_bytes());
    header.extend_from_slice(&len);
    let mut tap = block(0x00, &header);
    tap.extend(block(0xFF, &program));
    tap
}

/// A PNG's width, height, and its first pixel's RGB.
fn read_png(path: &Path) -> (u32, u32, [u8; 3]) {
    let png = fs::read(path).expect("the PNG");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "a PNG signature");
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let (width, height) = (be(&png[16..20]), be(&png[20..24]));
    let mut at = 8;
    let mut idat = Vec::new();
    while at < png.len() {
        let len = usize::try_from(be(&png[at..at + 4])).expect("a length");
        if &png[at + 4..at + 8] == b"IDAT" {
            idat.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let rows = miniz_oxide::inflate::decompress_to_vec_zlib(&idat).expect("its data inflates");
    assert_eq!(rows[0], 0, "no filter");
    (width, height, [rows[1], rows[2], rows[3]])
}

fn sidekick(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sidekick"))
        .args(args)
        .output()
        .expect("sidekick runs");
    (
        output.status.code().expect("a status"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_run_writes_the_picture_at_its_scale() {
    let dir = TempDir::new().unwrap();
    let tape = dir.path().join("wait.tap");
    fs::write(&tape, wait_then_red()).unwrap();
    let shot = dir.path().join("shot.png");
    let (status, stderr) = sidekick(&[
        "run",
        tape.to_str().unwrap(),
        "--frames",
        "200",
        "--screenshot",
        shot.to_str().unwrap(),
        "--scale",
        "3",
    ]);
    assert_eq!(status, 0, "{stderr}");
    let (width, height, corner) = read_png(&shot);
    assert_eq!((width, height), (960, 768));
    assert_eq!(
        corner,
        [215, 215, 215],
        "a white border: still waiting for a key"
    );
}

#[test]
fn a_key_reaches_the_program() {
    let dir = TempDir::new().unwrap();
    let tape = dir.path().join("wait.tap");
    fs::write(&tape, wait_then_red()).unwrap();
    let shot = dir.path().join("shot.png");
    let (status, stderr) = sidekick(&[
        "run",
        tape.to_str().unwrap(),
        "--frames",
        "250",
        "--key",
        "SPACE@200",
        "--screenshot",
        shot.to_str().unwrap(),
    ]);
    assert_eq!(status, 0, "{stderr}");
    let (width, height, corner) = read_png(&shot);
    assert_eq!((width, height), (320, 256));
    assert_eq!(corner, [215, 0, 0], "a red border: the key got through");
}

#[test]
fn values_and_the_state_are_printed_after_the_last_frame() {
    let dir = TempDir::new().unwrap();
    let tape = dir.path().join("wait.tap");
    fs::write(&tape, wait_then_red()).unwrap();
    let definition = dir.path().join("wait.toml");
    fs::write(
        &definition,
        r#"
schema = 1
[game]
id = "wait"
title = "Wait"
machine = "48k"
[[media]]
name = "our own test program"

# BORDCR, where the ROM keeps the border's colour times eight.
[values.border]
addr = 23624
type = "u8"

[values.prog]
addr = 23635
type = "u16"

[[states]]
name = "red"
when = { value = "border", at_least = 16, at_most = 23 }

[[states]]
name = "white"
when = { value = "border", bits = 0x38 }
"#,
    )
    .unwrap();
    let run = |frames: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_sidekick"))
            .env("ZX_SIDEKICK_DATA", dir.path())
            .args([
                "run",
                tape.to_str().unwrap(),
                "--definition",
                definition.to_str().unwrap(),
                "--values",
                "--frames",
                frames,
                "--key",
                "SPACE@200",
            ])
            .output()
            .expect("sidekick runs");
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    assert_eq!(
        run("150"),
        "state = \"white\"\nborder = 56\nprog = 23755\n",
        "waiting for a key"
    );
    assert_eq!(
        run("250"),
        "state = \"red\"\nborder = 23\nprog = 23755\n",
        "BORDER 2, with white ink for the lower screen"
    );
}

#[test]
fn bad_options_are_refused() {
    let dir = TempDir::new().unwrap();
    let tape = dir.path().join("wait.tap");
    fs::write(&tape, wait_then_red()).unwrap();
    let tape = tape.to_str().unwrap();
    for args in [
        ["run", tape, "--key", "SPACE"].as_slice(),
        &["run", tape, "--key", "F1@10"],
        &["run", tape, "--scale", "0"],
    ] {
        let (status, _) = sidekick(args);
        assert_ne!(status, 0, "{args:?}");
    }
}
