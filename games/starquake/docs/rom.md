# Does Starquake need the Spectrum ROM?

**Yes, but only three routines of it — so ZX Sidekick can run without a ROM file** by answering those three calls itself.

## Evidence

Measured on 2026-09-14, in the earlier ZX Sidekick build, with the `zx-runtime` interpreter from starquake-recompiled (since replaced by `rustzx-z80`, on which the checks below were rerun), starting from a snapshot of the loaded game with the real 48K ROM in place, over 6000 frames: the title screen, starting a game with the Kempston joystick, and play under random joystick input.

- **Without the ROM the run diverges.** The same 6000 frames with zeros in place of the ROM end with different RAM and a different program counter.
- **397 distinct ROM addresses run**, but **control enters the ROM from the game's own code at only three addresses**:

| Entry | ROM routine | Called from | Calls in the run |
|---|---|---|---|
| `0x0038` | MASK-INT: the 50 Hz interrupt (the game runs in interrupt mode 1), which counts frames and scans the keyboard | anywhere, by interrupt | every frame |
| `0x15F2` | PRINT-A-2: print one character through the current channel | `0xD3CF` only | 18,515 |
| `0x30A9` | HL-HL×DE: 16-bit multiply | `0x9DF6`, `0xA844`, `0xC6CD`, `0xD4D4`, `0xDA06` | 745 |

Everything else that ran in the ROM (KEY-SCAN, KEYBOARD, PRINT-OUT, PO-CHAR, CL-SET, the colour routines) was reached from those three.

- **System variables the game relies on those routines keeping** (known from starquake-recompiled's layout): `FRAMES` (`0x5C78`), which seeds its random numbers and times its sound, and the print state `S_POSN` (`0x5C88`), `ATTR_T` (`0x5C8F`), `MASK_T` (`0x5C90`) and `P_FLAG` (`0x5C91`).

## Decision: answer the three calls (implemented)

When the program counter reaches one of the three entries in the otherwise empty bottom 16K, which ignores writes as a ROM does, `sidekick::rom` does what the routine does to memory and registers, charges the time it takes, and returns. The interrupt is taken as a step of its own (`Zx::step`, from our fork of `rustzx-z80`), so it arrives at `0x0038` with none of the handler run, like a call arrives at a routine. A `JR $` (a jump to itself) sits at each entry as a safety stop: it runs only if an answer were ever missed, and then holds the processor there rather than letting it run into empty memory. No ROM file is in the repository or needed on the player's machine. It reproduces the *behaviour* of three Sinclair ROM routines, the way emulators' tape traps do, and copies no ROM code or data.

- **The interrupt** counts `FRAMES` and enables interrupts. The keyboard state the ROM also keeps (`KSTATE`, `LAST_K`, bit 5 of `FLAGS`) is not kept: Starquake reads the keyboard ports itself.
- **The multiply** runs the same shift-and-add, sixteen rounds, so HL, A, the flags and the time come out as the ROM's do.
- **The print** draws with `CHARS`, `UDG` and the block graphics, and keeps the print position, colours and the waiting state for control-code operands exactly where the ROM keeps them: `S_POSN`, `DF_CC`, `ATTR_T`, `MASK_T`, `P_FLAG`, `TVDATA`, and the output routine address of the current channel. That last one matters: Starquake resets the channel between strings, cancelling any code still waiting.

**Checked against the real ROM** by `sk-check rom` (development only, with a ROM supplied locally): through the menu, a new game and 6000 frames of play under random joystick input, every call the real ROM made was repeated from the same state by the answer, and memory, stack, return address and (for the interrupt and the multiply) all registers agreed. On 2026-09-14, on `rustzx-z80`: **20,326 of 20,326 calls**, interrupts included; 226 more were not compared because an interrupt landed inside them, which the answers are not.

**Timing is modelled.** The mean T-states per call, the real ROM against the answer:

| Call | ROM | Answer |
|---|---|---|
| interrupt, taking it included | 895 | 895 |
| print a character | 1610 | 1610 |
| print a control code or operand | 541 | 541 |
| multiply | 969 | 960 (exact count, without the contended stack delays) |

## Other options considered

1. **Ask the player for a ROM file as well as the tape.** Exact timing, but one more file to find, and a worse first run.
2. **Bundle the Sinclair ROM** under Amstrad's permission for emulators. Exact and easy for the player, but it puts ROM data in the repository, which the project's first rule forbids.
3. **An openly licensed replacement ROM** (such as OpenSE BASIC). It would have to keep PRINT-A-2 and HL-HL×DE at exactly `0x15F2` and `0x30A9` and behave identically; unverified, and trying it means downloading it first.

## How the game starts

The tape is a BASIC loader, a loading screen, and one 49,152-byte code block covering all of `0x4000`–`0xFFFF`, system variables and stack included. The ROM's loader pushes its own return address, loads the block over it, and returns through what the block put there: **`0x5E24`, with the stack pointer at `0x5E20`**, interrupts still disabled. `sk-check entry` establishes this by booting the real ROM, typing `LOAD ""`, and feeding its loader the tape's blocks; `starquake::facts` records the result, so the player's tape alone is enough.
