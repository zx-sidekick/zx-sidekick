//! A `.tap` loads as `LOAD ""` would load it, and runs.

use sidekick_core::{Machine, Model};

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

/// `10 POKE 40000,42`, auto-running from line 10: a BASIC program of our own, with the
/// numbers' hidden five-byte forms as the ROM's editor stores them.
fn poke_program() -> Vec<u8> {
    let mut line = vec![0xF4]; // POKE
    line.extend_from_slice(b"40000");
    line.extend_from_slice(&[0x0E, 0x00, 0x00, 0x40, 0x9C, 0x00]);
    line.push(b',');
    line.extend_from_slice(b"42");
    line.extend_from_slice(&[0x0E, 0x00, 0x00, 0x2A, 0x00, 0x00]);
    line.push(0x0D);
    let mut program = vec![0x00, 0x0A];
    program.extend_from_slice(
        &u16::try_from(line.len())
            .expect("a short line")
            .to_le_bytes(),
    );
    program.extend_from_slice(&line);

    let len = u16::try_from(program.len())
        .expect("a short program")
        .to_le_bytes();
    let mut header = vec![0x00]; // a program
    header.extend_from_slice(b"poke      ");
    header.extend_from_slice(&len); // its length
    header.extend_from_slice(&10u16.to_le_bytes()); // auto-run from line 10
    header.extend_from_slice(&len); // where its variables start
    let mut tap = block(0x00, &header);
    tap.extend(block(0xFF, &program));
    tap
}

#[test]
fn a_tape_loads_and_its_program_runs() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    machine.load_tap(&poke_program()).expect("the tape goes in");
    let ran = (0..300).any(|_| {
        machine.run_frame().expect("a frame");
        machine.peek(40000) == 42
    });
    assert!(ran, "the program loaded and poked 40000");
}

#[test]
fn a_broken_tape_is_refused() {
    let mut machine = Machine::new(Model::Sinclair48K).expect("a 48K machine");
    for broken in [
        &[][..],
        &[0x13],
        &[0x05, 0x00, 0xFF, 0x01],
        &[0x01, 0x00, 0xFF],
    ] {
        assert!(machine.load_tap(broken).is_err(), "{broken:02X?}");
    }
    assert!(machine.load_tap(&poke_program()).is_ok());
}
