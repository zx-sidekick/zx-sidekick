//! Facts about Starquake (Stephen Crow / Bubble Bus, 1985). Addresses and
//! values only: nothing here is the game's program.

use zx_spectrum::Key;

/// SHA-1 of the one tape this version knows the facts of.
pub const TAPE_SHA1: &str = "65450d6f33692c2c2868c0b497037f2cfd0ef3bd";

/// Where the game starts once its code block has loaded. The block covers all
/// of RAM, the stack included, and the ROM's loader returns through the
/// address the block leaves on its stack: here. Found by running the real
/// loader with a ROM, once, in development (`docs/rom.md`).
pub const ENTRY_PC: u16 = 0x5E24;
/// The stack pointer at that moment.
pub const ENTRY_SP: u16 = 0x5E20;
/// `IY` as the ROM keeps it, pointing into its system variables.
pub const ENTRY_IY: u16 = 0x5C3A;
/// The interrupt vector register the ROM sets at start-up.
pub const ENTRY_I: u8 = 0x3F;

/// The control method chosen on the title screen, 1 to 5 as the screen
/// numbers them: Kempston joystick, cursor joystick, Sinclair joystick,
/// keyboard, user-defined keys. Found on 2026-09-14 by choosing each option
/// on the player's tape and looking at what changed.
pub const CONTROL_METHOD: u16 = 0x5E58;

/// The four key tables the game reads in methods 2 to 5, five bytes each in
/// the order left, right, down, up, fire, each byte a key as [`key`] reads
/// it. The tape ships them as `5 8 6 7 0`, `1 2 3 4 5`, `O P A Q M` and
/// `Q W E R T`; the define-keys screen rewrites the last. Method 1 reads the
/// Kempston port instead.
pub const KEY_TABLES: u16 = 0x5E5C;

/// The pause key, as [`key`] reads it: Space as the tape ships it, and
/// whatever the define-keys screen was given after that. Methods 2 to 5
/// pause with it; the Kempston method pauses with Space whatever it holds.
/// Checked by hand and on the tape on 2026-09-14, in every method.
pub const PAUSE_KEY: u16 = 0x5E70;

/// The game's play-time key reader: the one routine that consults
/// [`CONTROL_METHOD`] and the tables, run once a frame during play and not
/// at all on the title screen (which reads the keyboard at `0xDA2B`) or the
/// define-keys screen (`0xD5D4`). It starts by reading the pause key. A key
/// pressed as the program counter arrives here reaches the game in play and
/// nowhere else. Found on 2026-09-14 by tracing every `IN` the game runs on
/// each screen.
pub const PLAY_INPUT: u16 = 0xC55D;

/// Where the same routine goes on to read the directions and fire, after
/// the pause key; and where a paused game waits, looping back here without
/// the pause read until a direction or fire resumes it. A key pressed only
/// at [`PLAY_INPUT`] never reaches a paused game. Found on 2026-09-14 by
/// tracing a paused game on the player's tape, in every control method.
/// The machine keeps the pause key from the game between the two, so the
/// game never pauses itself, and gives it back here.
pub const CONTROLS_INPUT: u16 = 0xC566;

/// The key read the title screen waits on until its first key, and the
/// intro text waits on for its any-key: every half-row, and on the title
/// screen the digits choose while `0` starts a game. Once a key has been
/// pressed the title screen reads through [`MENU_KEY`] instead, with its
/// highlight cycling between reads. Found on 2026-09-14 by tracing every
/// `IN` the game runs on each screen.
pub const MENU_INPUT: u16 = 0xDA2B;

/// The keyboard-reading routine the title screen calls, once a key has been
/// pressed, to take the next choice, and calls again a few frames after a
/// choice to confirm the key is still down. The define-keys screen reads its
/// keys through the same routine, from a different place: the two are told
/// apart by the return address on the stack as the routine is entered,
/// [`MENU_KEY_FROM_TITLE`] for the title screen. Found on 2026-09-14 by
/// tracing the calls leading to each key read on the player's tape.
pub const MENU_KEY: u16 = 0xD5C8;

/// The return address on the stack when the title screen calls
/// [`MENU_KEY`]: the instruction after its `CALL` at `0x6021`. The
/// define-keys screen's call returns to `0x625A`.
pub const MENU_KEY_FROM_TITLE: u16 = 0x6024;

/// Entry points of the game's routines that say what the program is doing,
/// for the panel beside it: which part of the program is running, and when
/// End this game may hold its keys. Found by studying the program in the
/// earlier ZX Sidekick build, and checked on the player's tape by
/// `sk-check facts`.
pub mod routine {
    /// The title screen's menu.
    pub const MENU: u16 = 0x5E81;
    /// The top of the play loop, once a frame while Blob is being played.
    pub const MAIN_LOOP: u16 = 0xA523;
    /// Where the play loop hands over to a security door, a teleporter booth
    /// or the pyramid, each of which then runs as a screen of its own.
    pub const MODAL: u16 = 0xA412;
    /// Blob losing a life.
    pub const DEATH: u16 = 0xC350;
    /// The end of a game: the scores, entering initials, the high-score
    /// table.
    pub const GAME_OVER: u16 = 0x6730;
    /// The CORE OF HEROES screen, showing the high-score table. The game
    /// reaches it after a game over once the table is final: a new entry
    /// already put in and named, or none (#47).
    pub const HEROES: u16 = 0x654B;
    /// Setting up a new game.
    pub const NEW_GAME: u16 = 0x629D;
    /// Entering a room, up to the play loop.
    pub const ENTER_ROOM: u16 = 0xA426;
    /// Where touching a security door calls [`MODAL`] for the door's screen,
    /// which shows the door's access code (#49). Checked on the player's
    /// tape by `sk-check facts`; a booth calls it from elsewhere.
    pub const DOOR_SCREEN: u16 = 0xCBEA;
    /// A teleporter booth, one of the screens play hands over to. It prints
    /// the code of the teleporter Blob is standing in.
    pub const TELEPORT_BOOTH: u16 = 0xCED4;
    /// Drawing a 2 × 2 graphic: the attribute in A, the character row in B,
    /// the column in C, and the graphic's 32 bytes at HL (see
    /// [`super::at::GRAPHICS`]), laid on the screen by XOR. Pickups in a room and
    /// the core's holes are drawn with it.
    pub const DRAW_GRAPHIC: u16 = 0xDB24;
    /// Drawing a room's tiles, from its first instruction to its last.
    pub const BUILD_ROOM_TILES: u16 = 0xA80A;
    pub const BUILD_ROOM_TILES_END: u16 = 0xAA30;
}

/// Where the game keeps what the guidance panel shows. Addresses in the
/// original program, found in the earlier ZX Sidekick build and checked on
/// the player's tape by `sk-check facts`.
pub mod at {
    /// The room Blob is in, a word from 0 to 511.
    pub const ROOM: u16 = 0xD2C8;
    /// The teleporters: fifteen entries of five letters, the code, then the
    /// room the teleporter is in as a word.
    pub const TELEPORTER_NAMES: u16 = 0xD036;
    pub const TELEPORTER_COUNT: usize = 15;
    /// The six entity slots, 32 bytes each; slot 0 is Blob, whose position
    /// is at offsets 5 (pixels from the left) and 6 (from the bottom).
    pub const ENTITIES: u16 = 0xDD18;
    /// The markers the current room's tiles left: from here to the address
    /// held at [`MARKERS_END`], three bytes each (x, y, kind).
    pub const MARKERS: u16 = 0x96FC;
    pub const MARKERS_END: u16 = 0x96FA;
    /// The code the last door or pyramid screen asked for: its column, row
    /// and length, then a (graphic, matched) pair for each item. A security
    /// door asks for three chips, the pyramid for two (#33, #49).
    pub const CODE: u16 = 0xD5F4;
    /// The system variable `CHARS`: 256 less than the address of the
    /// font's space, as the ROM's printing reads it.
    pub const CHARS: u16 = 0x5C36;
    /// The game's own font, which `CHARS` points 256 bytes below during
    /// play: 96 letters of 8 × 8 pixels from the space, a row to a byte
    /// (#49).
    pub const FONT: u16 = 0xADD4;
    /// Why the room was entered, as [`super::entry`] names the values.
    pub const ENTRY_REASON: u16 = 0xD2C4;
    /// The rooms not yet visited this game: 512 bits, most significant
    /// first.
    pub const UNVISITED_ROOMS: u16 = 0xA390;
    /// The core's nine holes: bit 7 set while a hole is open, the low bits
    /// then the graphic of the piece that fills it; a filled hole holds its
    /// own number.
    pub const CORE_SLOTS: u16 = 0xD2DE;
    /// The lives left, 3 at a new game and one fewer each death, and the
    /// digit the panel prints for it, as text (#8).
    pub const LIVES: u16 = 0xD2CC;
    pub const LIVES_DIGIT: u16 = 0xD442;
    /// Blob's energy, 127 as play starts, falling by 4 whenever the drain
    /// counter reaches [`super::DRAIN_DROP`] (#8).
    pub const ENERGY: u16 = 0xD2CD;
    /// How full the platform bar and the gun bar are: one falls as Blob
    /// lays platforms, the other by one a shot (#8).
    pub const PLATFORMS: u16 = 0xD2CE;
    pub const GUN: u16 = 0xD2CF;
    /// The drain counter in Blob's slot (offset `0x18`): it rises by one a
    /// frame, and touching an enemy pushes it on, so energy falls far
    /// faster on contact than by time alone (#8).
    pub const DRAIN: u16 = 0xDD30;
    /// The 45 items, four bytes each: column (and colour), row with the room's
    /// top bit, the room's low byte, graphic.
    pub const ITEMS: u16 = 0x94E8;
    pub const ITEM_COUNT: usize = 45;
    /// The graphics pickups and the core's holes are drawn with: 32 bytes
    /// each, from graphic 0, four 8 × 8 cells top left, top right, bottom
    /// left, bottom right.
    pub const GRAPHICS: u16 = 0x9088;
    /// The high-score table: [`HIGH_SCORE_COUNT`] entries of ten bytes, best
    /// first, each a three-letter name, the score as six digits of text and
    /// the percentage of the mission done (#47).
    pub const HIGH_SCORES: u16 = 0x64FA;
    pub const HIGH_SCORE_COUNT: usize = 8;
    /// The score of the game just over, as six digits of text, which the
    /// game ranks against the table.
    pub const FINAL_SCORE: u16 = 0x67EA;
    /// The restore list the room builder writes, and the pointer into it.
    pub const RESTORE_LIST: u16 = 0x5B20;
    pub const RESTORE_PTR: u16 = 0xEA60;
}

/// Why a room was entered, as the game keeps it at [`at::ENTRY_REASON`].
/// Found on 2026-09-15 by typing codes into a booth on the player's tape:
/// any code that teleports, the booth's own included, leaves
/// [`entry::TELEPORTED`]; a code not recognised leaves 3.
pub mod entry {
    /// Walking in through an edge or a wall passage.
    pub const WALKED: u8 = 0;
    /// Arriving by teleport.
    pub const TELEPORTED: u8 = 4;
}

/// The core room, which the game runs as a screen of its own.
pub const CORE_ROOM: u16 = 199;

/// Has the game draw `room` on this machine, which should be a copy, and
/// reads its cells and markers.
///
/// # Panics
///
/// If the game's room drawing does not finish, which means the machine does
/// not hold Starquake.
pub fn read_room(machine: &mut crate::Machine, room: u16) -> crate::map::Room {
    let z = &mut machine.zx;
    // As the game leaves the screen before drawing a room: the room area
    // (rows 6 to 23) blank, in bright white on black, which Blob can pass.
    z.mem[0x4000..0x5800].fill(0);
    z.mem[0x5800 + 6 * 32..0x5B00].fill(0x47);
    z.mem[usize::from(at::RESTORE_LIST)..usize::from(at::RESTORE_LIST) + 0xA0].fill(0);
    z.write16(at::RESTORE_PTR, at::RESTORE_LIST);
    z.write16(at::ROOM, room);
    assert!(
        machine.call(
            routine::BUILD_ROOM_TILES,
            routine::BUILD_ROOM_TILES_END,
            5_000_000
        ),
        "room {room} did not finish drawing"
    );
    let z = &machine.zx;
    let end = z.read16(at::MARKERS_END).max(at::MARKERS);
    let markers: Vec<(u8, u8, u8)> = (at::MARKERS..end)
        .step_by(3)
        .map(|a| {
            let a = usize::from(a);
            (z.mem[a], z.mem[a + 1], z.mem[a + 2])
        })
        .collect();
    crate::map::Room::read(
        |row, col| z.mem[0x5800 + usize::from(row) * 32 + usize::from(col)],
        &markers,
    )
}

/// Every room's openings, read by having the game draw each room on a copy
/// of `machine`.
///
/// # Panics
///
/// As [`read_room`].
#[must_use]
pub fn all_openings(machine: &crate::Machine) -> Vec<crate::map::Openings> {
    crate::map::openings(&all_rooms(machine), CORE_ROOM)
}

/// Every room as the map reads it, in number order, each read by having the
/// game draw it on a copy of `machine`.
///
/// # Panics
///
/// As [`read_room`].
#[must_use]
pub fn all_rooms(machine: &crate::Machine) -> Vec<crate::map::Room> {
    (0..crate::map::COLS * crate::map::ROWS)
        .map(|room| read_room(&mut machine.clone(), room))
        .collect()
}

/// What the drain counter reaches before energy falls by four (#8).
pub const DRAIN_DROP: u8 = 0x78;

/// The marker a teleporter booth's tile leaves in its room.
pub const BOOTH_MARKER: u8 = 0x0D;

/// The marker a security door's tile leaves in its room.
pub const DOOR_MARKER: u8 = 0x00;

/// The game's font from `mem` (the machine's whole 64K): 96 letters of
/// eight bytes each, from the space. `None` when `mem` is too short.
#[must_use]
pub fn font(mem: &[u8]) -> Option<&[u8]> {
    mem.get(usize::from(at::FONT)..usize::from(at::FONT) + 96 * 8)
}

/// The three chips a security door's screen asked for, by graphic, from the
/// code in `mem` (the machine's whole 64K), when the last screen was a
/// door's: a pyramid's code, of two items, is not one.
#[must_use]
pub fn door_code(mem: &[u8]) -> Option<[u8; 3]> {
    let code = mem.get(usize::from(at::CODE)..usize::from(at::CODE) + 9)?;
    (code[2] == 3).then(|| [code[3], code[5], code[7]])
}

/// Every room with a security door in it, and the spot of its door marker,
/// found by having the game build each room on a copy (#66). The rooms are
/// the tape's own and do not change from game to game, so this is read once.
#[must_use]
pub fn door_rooms(machine: &crate::Machine) -> Vec<(u16, (u8, u8))> {
    let mut found = Vec::new();
    for room in 0..512u16 {
        let mut m = machine.clone();
        read_room(&mut m, room);
        let z = &m.zx;
        let end = z.read16(at::MARKERS_END).max(at::MARKERS);
        if let Some(a) = (at::MARKERS..end)
            .step_by(3)
            .find(|&a| z.mem[usize::from(a) + 2] == DOOR_MARKER)
        {
            let a = usize::from(a);
            found.push((room, (z.mem[a], z.mem[a + 1])));
        }
    }
    found
}

/// The code a security door asks for this game, read by walking Blob into
/// the door on a copy of the machine, which is the only way its screen is
/// reached (#66). `spot` is the door marker's, from [`door_rooms`].
#[must_use]
pub fn read_door_code(machine: &crate::Machine, room: u16, spot: (u8, u8)) -> Option<[u8; 3]> {
    let mut entered = machine.clone();
    entered.zx.write16(at::ROOM, room);
    entered.zx.mem[usize::from(at::ENTRY_REASON)] = 0;
    if !entered.call(routine::ENTER_ROOM, routine::MAIN_LOOP, 20_000_000) {
        return None;
    }
    entered.zx.t = 0;
    entered.zx.set_interrupts(true);
    let (x, y) = spot;
    // Into the door from one side or the other, whichever reaches it. The
    // joystick goes through the machine, which presses whatever the control
    // method the player chose listens for: writing the Kempston port would
    // reach the game only in one of the five.
    [crate::machine::JOY_RIGHT, crate::machine::JOY_LEFT]
        .into_iter()
        .find_map(|input| {
            let mut m = entered.clone();
            m.zx.mem[usize::from(at::ENTITIES) + 5] = x;
            m.zx.mem[usize::from(at::ENTITIES) + 6] = y;
            m.watch = vec![routine::DOOR_SCREEN, routine::ENTER_ROOM];
            let mut called = false;
            for _ in 0..600 {
                m.zx.release_all_keys();
                m.joystick = if called { 0 } else { input };
                for hit in m.run_frame() {
                    if hit == routine::DOOR_SCREEN {
                        called = true;
                    } else if called {
                        // Back in the room: the screen left its code behind.
                        return door_code(&m.zx.mem[..]);
                    }
                }
                if !called && m.zx.mem[usize::from(at::ENTITIES) + 5].abs_diff(x) > 8 {
                    return None;
                }
            }
            None
        })
}

/// A teleporter whose booth has been entered: the room it is in and its
/// code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeenTeleporter {
    pub room: u16,
    pub code: [u8; 5],
}

/// One entry of the high-score table, as its ten bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct HighScore {
    pub name: [u8; 3],
    pub score: [u8; 6],
    pub percent: u8,
}

/// The high-score table in `mem` (the machine's whole 64K), best first;
/// `None` when `mem` is too short.
#[must_use]
pub fn high_scores(mem: &[u8]) -> Option<[HighScore; at::HIGH_SCORE_COUNT]> {
    let start = usize::from(at::HIGH_SCORES);
    let bytes = mem.get(start..start + 10 * at::HIGH_SCORE_COUNT)?;
    Some(std::array::from_fn(|i| {
        let e = &bytes[i * 10..i * 10 + 10];
        HighScore {
            name: [e[0], e[1], e[2]],
            score: std::array::from_fn(|k| e[3 + k]),
            percent: e[9],
        }
    }))
}

/// Puts `table` into `mem` as the game's high-score table, which the game
/// then ranks against and shows as its own.
pub fn write_high_scores(mem: &mut [u8], table: &[HighScore; at::HIGH_SCORE_COUNT]) {
    let start = usize::from(at::HIGH_SCORES);
    for (i, entry) in table.iter().enumerate() {
        let e = &mut mem[start + i * 10..start + i * 10 + 10];
        e[..3].copy_from_slice(&entry.name);
        e[3..9].copy_from_slice(&entry.score);
        e[9] = entry.percent;
    }
}

/// The code of the teleporter in `room`, from the game's table in `mem`
/// (the machine's whole 64K), if there is one there.
#[must_use]
pub fn teleporter_code(mem: &[u8], room: u16) -> Option<[u8; 5]> {
    (0..at::TELEPORTER_COUNT).find_map(|i| {
        let entry = mem.get(usize::from(at::TELEPORTER_NAMES) + i * 7..)?;
        let (code, here) = (entry.get(..5)?, entry.get(5..7)?);
        (u16::from_le_bytes([here[0], here[1]]) == room).then(|| code.try_into().ok())?
    })
}

/// Every teleporter on the planet, from the same table, whether or not its
/// booth has been entered: level 6 tells them all (#66).
#[must_use]
pub fn all_teleporters(mem: &[u8]) -> Vec<SeenTeleporter> {
    (0..at::TELEPORTER_COUNT)
        .filter_map(|i| {
            let entry = mem.get(usize::from(at::TELEPORTER_NAMES) + i * 7..)?;
            let (code, here) = (entry.get(..5)?, entry.get(5..7)?);
            let room = u16::from_le_bytes([here[0], here[1]]);
            Some(SeenTeleporter {
                room,
                code: code.try_into().ok()?,
            })
        })
        .collect()
}

/// The keys that abandon a game in play when held together, the game's own
/// way: A, S, D, F and G, the whole of the keyboard's half-row 1, as its
/// half-row and bits. Found in the earlier ZX Sidekick build and checked on
/// the player's tape by `sk-check facts`.
pub const END_GAME_KEYS: (usize, u8) = (1, 0x1F);

/// What End this game holds: [`END_GAME_KEYS`], from the top of the play
/// loop until play hands over to a death or to a door, booth or pyramid
/// screen, which would read them as letters.
#[must_use]
pub fn end_game_hold() -> crate::machine::Hold {
    crate::machine::Hold {
        from: routine::MAIN_LOOP,
        until: vec![routine::MODAL, routine::DEATH],
        row: END_GAME_KEYS.0,
        bits: END_GAME_KEYS.1,
    }
}

/// The key a byte of the game's tables names. Letters and digits are their
/// ASCII; the four keys with no character of their own are the codes the
/// define-keys screen writes for them, seen on the player's tape: `*` for
/// Space, `\` for Enter, `[` for Caps Shift and `]` for Symbol Shift.
#[must_use]
pub fn key(code: u8) -> Option<Key> {
    let name = match code {
        b'*' => "space",
        b'\\' => "enter",
        b'[' => "caps",
        b']' => "symbol",
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
            return Key::by_name(&(code as char).to_string());
        }
        _ => return None,
    };
    Key::by_name(name)
}

/// Whether `bytes` are the tape these facts are about.
#[must_use]
pub fn is_supported_tape(bytes: &[u8]) -> bool {
    zx_core::sha1::sha1_hex(bytes) == TAPE_SHA1
}

/// One of the game's items, as its four bytes in the item table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item(pub [u8; 4]);

impl Item {
    #[must_use]
    pub fn room(&self) -> u16 {
        u16::from(self.0[2]) | u16::from(self.0[1] >> 7) << 8
    }
    /// The screen row it sits at in its room; 0 before its room has been
    /// entered, 1 while being picked up, 2 to 5 while carried.
    #[must_use]
    pub fn row(&self) -> u8 {
        self.0[1] & 0x7F
    }
    #[must_use]
    pub fn graphic(&self) -> u8 {
        self.0[3]
    }
    /// The screen column it is drawn at once placed in its room.
    #[must_use]
    pub fn column(&self) -> u8 {
        self.0[0] & 0x1F
    }
}

/// What an item does, by its graphic (#36), which is what the map's icon
/// for it says. Read from the game's own code by way of
/// `starquake/starquake-recompiled`, which was written from the tape:
/// the codes a security door and a Cheops pyramid ask for take five
/// numbered chips, a wildcard that is used up and a master key that is
/// not; the pad key switches a teleporter pad; the packs act the moment
/// they are picked up; and a pyramid takes anything else in exchange for
/// a core piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A numbered chip, `0`, `1`, `2`, `4` or `8`: what a door's code and
    /// a pyramid's code ask for. Used up when it answers one.
    Chip(u8),
    /// The chip that answers any one slot of a code, and is used up.
    AnyChip,
    /// The card that answers any slot of any code and is never used up,
    /// so it opens every door and every pyramid.
    DoorCard,
    /// The key that switches the teleporter pads in a room, by being
    /// carried onto one.
    PadKey,
    /// A pack: it acts the moment it is picked up, giving a life or
    /// filling one of the bars. Number 25 is the Cheops pyramid itself.
    Pack,
    /// Anything else, a core piece included: what a Cheops pyramid takes
    /// in exchange for a core piece.
    Trade,
}

/// What the item with this `graphic` does. A core piece is [`Kind::Trade`]
/// too, since a pyramid takes it the same way; the map tells the two apart
/// by what the core wants, which is the game's own choice each game.
#[must_use]
pub fn kind(graphic: u8) -> Kind {
    match graphic {
        9 => Kind::Chip(b'0'),
        10 => Kind::Chip(b'1'),
        11 => Kind::Chip(b'2'),
        12 => Kind::Chip(b'4'),
        13 => Kind::Chip(b'8'),
        14 => Kind::AnyChip,
        15 => Kind::DoorCard,
        16 => Kind::PadKey,
        17..=25 => Kind::Pack,
        _ => Kind::Trade,
    }
}

/// The items and the core's holes, from the machine's memory.
#[must_use]
pub fn items_and_core(mem: &[u8]) -> (Vec<Item>, [u8; 9]) {
    let items = (0..at::ITEM_COUNT)
        .map(|i| {
            let a = usize::from(at::ITEMS) + i * 4;
            Item([mem[a], mem[a + 1], mem[a + 2], mem[a + 3]])
        })
        .collect();
    let core = std::array::from_fn(|i| mem[usize::from(at::CORE_SLOTS) + i]);
    (items, core)
}

/// Graphic `number`'s 32 bytes, from the machine's memory `mem`.
///
/// # Panics
///
/// If `mem` is not the machine's whole memory.
#[must_use]
pub fn graphic(mem: &[u8], number: u8) -> [u8; 32] {
    let at = usize::from(at::GRAPHICS) + usize::from(number) * 32;
    mem[at..at + 32].try_into().expect("32 bytes")
}

/// What the core shows in hole `index`, from its byte `slot`: the graphic,
/// and whether the hole is still open. An open hole shows the piece that
/// fills it; a filled one keeps only its own number, and shows that
/// graphic as its placeholder.
#[must_use]
pub fn hole(index: usize, slot: u8) -> (u8, bool) {
    if slot & 0x80 == 0 {
        (index as u8, false)
    } else {
        (slot & 0x7F, true)
    }
}

/// The rooms of the items that would fill a hole still open in the core.
///
/// A hole is open while its slot has bit 7 set, and the rest of the byte is
/// the graphic of the piece that fills it: the core takes any carried item
/// with that graphic, so every item with it counts, wherever it is. An item
/// being picked up or carried is not somewhere to go, and nor is one already
/// delivered, which the game parks in the core room at row 10. An item not
/// yet placed in its room has row 0 and still counts; its room is known.
///
/// A hole whose piece is being carried marks nothing: most pieces come in
/// twos, and the other one is not needed while you have one.
#[must_use]
pub fn missing_piece_rooms(core_slots: &[u8; 9], items: &[Item]) -> crate::map::RoomSet {
    let mut rooms = crate::map::RoomSet::default();
    for item in missing_pieces(core_slots, items) {
        rooms.set(item.room(), true);
    }
    rooms
}

/// The items [`missing_piece_rooms`] marks the rooms of, for a route to
/// the spot one is at once it is placed (#50).
#[must_use]
pub fn missing_pieces(core_slots: &[u8; 9], items: &[Item]) -> Vec<Item> {
    let carried = |item: &Item| (1..=5).contains(&item.row());
    let wanted = |graphic: u8| {
        core_slots
            .iter()
            .any(|&slot| slot & 0x80 != 0 && slot & 0x7F == graphic)
            && !items.iter().any(|i| carried(i) && i.graphic() == graphic)
    };
    items
        .iter()
        .copied()
        .filter(|item| {
            let delivered = item.room() == CORE_ROOM && item.row() == 0x0A;
            wanted(item.graphic()) && !carried(item) && !delivered
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_item_is_named_by_its_graphic() {
        assert_eq!(kind(9), Kind::Chip(b'0'));
        assert_eq!(kind(13), Kind::Chip(b'8'));
        assert_eq!(kind(14), Kind::AnyChip);
        assert_eq!(kind(15), Kind::DoorCard);
        assert_eq!(kind(16), Kind::PadKey);
        assert_eq!(kind(17), Kind::Pack);
        assert_eq!(kind(25), Kind::Pack, "the Cheops pyramid");
        assert_eq!(kind(26), Kind::Trade, "the transistor");
        assert_eq!(kind(28), Kind::Trade, "the antenna");
        assert_eq!(kind(33), Kind::Trade, "a core piece trades the same way");
    }

    #[test]
    fn the_high_score_table_reads_back_as_written() {
        let mut mem = vec![0u8; 0x10000];
        let mut table = [HighScore::default(); at::HIGH_SCORE_COUNT];
        table[0] = HighScore {
            name: *b"STA",
            score: *b"109825",
            percent: 31,
        };
        table[7] = HighScore {
            name: *b"KES",
            score: *b"009875",
            percent: 5,
        };
        write_high_scores(&mut mem, &table);
        let a = usize::from(at::HIGH_SCORES);
        assert_eq!(&mem[a..a + 10], b"STA109825\x1f");
        assert_eq!(high_scores(&mem), Some(table));
        assert_eq!(high_scores(&mem[..0x6500]), None, "cut short");
    }

    #[test]
    fn the_font_is_96_letters_from_its_address() {
        let mut mem = vec![0u8; 0x10000];
        let a = usize::from(at::FONT) + (usize::from(b'A') - 0x20) * 8;
        mem[a] = 0x3C;
        let font = font(&mem).unwrap();
        assert_eq!(font.len(), 768);
        assert_eq!(font[(usize::from(b'A') - 0x20) * 8], 0x3C);
        assert_eq!(super::font(&mem[..0xB000]), None, "cut short");
    }

    #[test]
    fn a_door_code_is_three_chips_and_a_pyramid_code_is_none() {
        let mut mem = vec![0u8; 0x10000];
        let at = usize::from(at::CODE);
        mem[at..at + 9].copy_from_slice(&[0x0B, 0x11, 3, 11, 3, 12, 3, 11, 3]);
        assert_eq!(door_code(&mem), Some([11, 12, 11]));
        mem[at + 2] = 2;
        assert_eq!(door_code(&mem), None, "the pyramid asks for two");
    }
    use crate::map::RoomSet;

    /// An item in `room` at `row`, with `graphic`.
    fn item(room: u16, row: u8, graphic: u8) -> Item {
        Item([
            0,
            ((room >> 8) as u8).rotate_right(1) | row,
            room as u8,
            graphic,
        ])
    }

    #[test]
    fn a_piece_for_an_open_hole_marks_its_room() {
        // Hole 0 wants graphic 0x09 and hole 1 is filled (it holds its own
        // number); the rest want graphics nothing here has.
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        slots[1] = 1;
        let items = [
            item(300, 0, 0x09), // not placed yet: counts
            item(40, 12, 0x09), // a second one with the same graphic
            item(41, 12, 0x01), // wanted by no open hole
            item(42, 12, 0x0F), // not a core piece at all
        ];
        assert_eq!(missing_pieces(&slots, &items), items[..2]);
        let rooms = missing_piece_rooms(&slots, &items);
        assert!(rooms.contains(300) && rooms.contains(40));
        assert!(!rooms.contains(41) && !rooms.contains(42));
    }

    #[test]
    fn carried_and_delivered_pieces_are_not_marked() {
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        slots[1] = 0x80 | 0x0A;
        slots[2] = 0x80 | 0x0B;
        let items = [
            item(CORE_ROOM, 0x0A, 0x09), // delivered
            item(51, 2, 0x0A),           // in the inventory
            item(52, 5, 0x0B),           // last inventory slot
        ];
        assert_eq!(missing_pieces(&slots, &items), []);
        assert_eq!(missing_piece_rooms(&slots, &items), RoomSet::default());
    }

    #[test]
    fn carrying_a_piece_unmarks_its_twin() {
        let mut slots = [0x80 | 0x30; 9];
        slots[0] = 0x80 | 0x09;
        let twin = item(60, 12, 0x09);
        assert!(missing_piece_rooms(&slots, &[twin]).contains(60));
        for row in [1, 2, 5] {
            let rooms = missing_piece_rooms(&slots, &[item(61, row, 0x09), twin]);
            assert!(!rooms.contains(60), "carried at row {row}");
        }
    }

    #[test]
    fn every_key_of_the_matrix_has_a_code_and_nothing_else_does() {
        let mut seen = std::collections::HashSet::new();
        let codes = (b'0'..=b'9').chain(b'A'..=b'Z').chain(*b"*\\[]");
        for code in codes {
            let Some(Key::Matrix(row, bit)) = key(code) else {
                panic!("{} should name a key", code as char);
            };
            assert!(
                seen.insert((row, bit)),
                "{} names a key twice",
                code as char
            );
        }
        assert_eq!(seen.len(), 40);
        assert_eq!(key(b'o'), key(b'O'));
        assert_eq!(key(b'*'), Key::by_name("space"));
        assert_eq!(key(b'\\'), Key::by_name("enter"));
        assert_eq!(key(b'['), Key::by_name("caps"));
        assert_eq!(key(b']'), Key::by_name("symbol"));
        for code in [0, b' ', b'\r', b'!', 0x7F, 0xFF] {
            assert_eq!(key(code), None, "{code:#04x}");
        }
    }

    #[test]
    fn an_item_s_column_is_in_its_first_byte() {
        let i = Item([0x95, 19, 16, 33]);
        assert_eq!((i.column(), i.row(), i.graphic()), (21, 19, 33));
    }

    #[test]
    fn an_open_hole_shows_its_piece_and_a_filled_one_its_own_number() {
        assert_eq!(hole(0, 0x9B), (0x1B, true));
        assert_eq!(hole(4, 0x04), (4, false));
        let mut mem = vec![0u8; 0x10000];
        let at = usize::from(at::GRAPHICS) + 33 * 32;
        mem[at] = 0xAB;
        mem[at + 31] = 0xCD;
        let g = graphic(&mem, 33);
        assert_eq!((g[0], g[31]), (0xAB, 0xCD));
    }

    #[test]
    fn a_teleporter_code_comes_from_its_room_s_entry() {
        let mut mem = vec![0u8; 0x10000];
        let entry = usize::from(at::TELEPORTER_NAMES) + 7 * 3;
        mem[entry..entry + 5].copy_from_slice(b"ABCDE");
        mem[entry + 5..entry + 7].copy_from_slice(&300u16.to_le_bytes());
        assert_eq!(teleporter_code(&mem, 300), Some(*b"ABCDE"));
        assert_eq!(teleporter_code(&mem, 301), None);
        assert_eq!(teleporter_code(&mem[..0xD040], 300), None, "cut short");
    }

    #[test]
    fn only_the_known_tape_is_supported() {
        assert!(!is_supported_tape(&[]));
        assert!(!is_supported_tape(b"not a tape"));
        assert_eq!(TAPE_SHA1.len(), 40);
    }
}
