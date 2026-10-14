# ZX Sidekick: the player's guide

ZX Sidekick plays original ZX Spectrum games from your own copy, in an emulated Spectrum, with a gamepad, optional training switches, a guidance panel and save games. What it adds for a game comes from that game's definition, in `games/` beside the program. This release has one: Starquake.

**No game is included; you need your own copy.** ZX Sidekick is not affiliated with or endorsed by any game's rights holders. The Spectrum ROMs are included with Amstrad's permission (`THIRD-PARTY.md`).

## What you need

- **The program for your system:** macOS 11 or later (Apple silicon or Intel), 64-bit Windows 10 or 11, or 64-bit Linux on x86 with a desktop running Wayland or X11 and glibc 2.35 or later (Ubuntu 22.04, Debian 12 or newer).
- **Your own copy of the game's tape.** A definition names the tape it works with, and the program checks the file you give it against that; a different dump is refused, with a message naming the one it needs. For Starquake that's the tape World of Spectrum distributes: download `Starquake.tap.zip` from <https://worldofspectrum.net/item/0004873>. The `.zip` can be given as it is.

## Installing

Unpack the archive anywhere you like. The folder holds the program (`sidekick`, or `sidekick.exe` on Windows), `games/` with the definitions, this guide, `AUTHORS-GUIDE.md` for writing definitions, the README, the licence, `THIRD-PARTY.md` and `licenses/`. Keep `games/` beside the program: that's where it looks for them.

`starquake.zip`, on the release page, is Starquake's definition on its own, for a program already installed: put it in `games/` beside the program, or in `games/` in the data folder (below).

### Opening it the first time on macOS

The program isn't signed by a registered Apple developer, so macOS stops it the first time.

1. Double-click `sidekick`. macOS says it can't check it for malicious software; click **Done**.
2. Open **System Settings → Privacy & Security**, scroll down to the message that `sidekick` was blocked, and click **Open Anyway**. Confirm with your password.

On macOS 14 and earlier, Control-click (or right-click) `sidekick`, choose **Open**, then **Open** again.

Or, in Terminal, in the unpacked folder:

```
xattr -d com.apple.quarantine sidekick
./sidekick
```

Double-clicking it opens a Terminal window beside the game, which shows any messages; close it once you've quit.

### Opening it the first time on Windows

The program isn't signed, so SmartScreen says "Windows protected your PC". Click **More info**, then **Run anyway**.

### Linux

Run `./sidekick` from the unpacked folder, or open it from a file manager that runs programs.

## First run

With one game installed, the program starts it. Without its tape, it asks for it:

- **Locate the tape…** (Enter, or A on a pad) opens a file dialog: choose the `.tap`, `.tzx` or the `.zip` it came in. Dropping the file onto the window does the same.
- The download button (W) opens the page the tape comes from.

The tape is kept in the data folder, so it's found the next time.

The game starts fullscreen. **F11** switches between fullscreen and a window; on a Mac, **Control-Command-F** does. The tape loads quickly; on the loading screen a card says to press any key or button to go on.

On Starquake's title the program has already chosen the joystick for you, so ignore the game's list of controls: press any key or button to begin (not the arrows or the D-pad), and play with the arrow keys or a gamepad.

When a game starts, a **How to play** card says what the game is about and how its main moves are made, with the keys (and a pad's buttons, while one is plugged in) for each. Enter, Esc or A plays on. Turn **Show next time** off (left or right, or a click) to skip it at the start of later games; the menu's *How to play* row shows it any time.

## Keys and pad

**Keyboard.** The keyboard plays as a pad, with the **arrow keys** and **Space** only: the arrows move (in Starquake, up picks up and down builds, as in the original), Space fires, and **Enter** is Start (in Starquake, it pauses). No other key plays a game, except where it asks for letters (a teleport code, your name), which you type; on a pad, a card spells them. A game that needs the Spectrum's own keys turns its keyboard on in its definition, and the guidance menu's **Use the Spectrum keyboard** switch can turn it on for the session: then letters, digits, Enter, Space, Shift (CAPS SHIFT), the left Ctrl and Alt (SYMBOL SHIFT) and Backspace (DELETE) reach the game as on the original, and the arrows and the **right Ctrl**, the **right Alt (Option)** and **/** still play as the pad, the arrows moving and the others firing.

On the loading screen any key or button goes on, letters and digits too, and on Starquake's title any key or button starts a game; the arrows, the D-pad and the stick do nothing on either.

The program's own keys:

| Key | Does |
|---|---|
| Esc | The guidance menu (the game pauses) |
| F5 | Save |
| F9 | Load |
| F11 (Mac: Control-Command-F) | Fullscreen or a window |

**Gamepad.** Any controller your system knows, by USB or Bluetooth, plugged in at any time. The D-pad and left stick move; **Select** opens the guidance menu. Buttons go by position, and the hints on screen use the letters or symbols of your pad's maker. In Starquake:

| Pad | Keyboard | Does |
|---|---|---|
| D-pad, left stick | Arrow keys | Move left and right; up boards a hover platform where it waits, and on it, up and down fly it. The arrow keys' up and down pick up or board, and build, everywhere |
| West button | Space | Fire |
| South button | ↓ | Down: build a platform, or fly down |
| East button | ↑ | Up: pick up or swap an item, or board a hover platform |
| North button | Tab | At guidance level 5, the next route |
| Start | Enter | Pause |
| Any button but the D-pad and stick | Any key but the arrows | On the title, start a game, and go on from the story that follows |
| Select | Esc | The guidance menu |

Starquake's own keys (its list of controls, A S D F G together to end a game) don't reach it while the keyboard is a pad; the guidance menu's *End this game* ends one. When the game is paused, a card says how to go on: move or fire.

A button still held when a menu closes is held back from the game until you let it go, so the press that closed the menu doesn't also fire or pause.

## Guidance

Beside the picture, the panel offers optional help, drawn by ZX Sidekick, never by the game. Nothing in it changes what the game does. Choose a level in the guidance menu; each level adds to the ones below it. Starquake's:

| Level | Name | Adds |
|:-:|---|---|
| 0 | Off | Nothing: the original. |
| 1 | Codes and the core | The teleport codes you've seen, the doors' cards, and the core's pieces once you've been to the core. |
| 2 | The map you have walked | A map of the rooms you've visited. |
| 3 | What you have seen | The items in the rooms you have walked through, core pieces still needed in pink. |
| 4 | What you have not | Items in the rooms you haven't reached. |
| 5 | Routes | The shortest way to the nearest missing piece, and to the core. Tab, or the north button, switches between the nearest pieces. |
| 6 | Everything | Every code, every door's cards, and the whole planet. |

Levels 0 to 3 show only what you could have written down yourself; 4 and up tell you things you couldn't have known.

At a teleport booth, a pad opens a card to choose one of the codes you've seen, or spell one letter by letter. On the keyboard, type the code as on the original.

The codes you discover are kept between games. The menu's **Forget the teleport codes** clears them.

## The guidance menu and training switches

**Esc**, or **Select** on a pad, opens the menu and pauses the game. Up and down move between rows; left and right change a row; Enter or A chooses.

- **Guidance level:** left and right, or Enter to step up.
- **Training mode:** a switch for each of the game's trainers. Starquake's are Full energy, Full bridging platforms, Full laser, Endless lives and No harm from enemies.
- **Back to the game** applies the changes. **Esc** or **B** leaves without them.
- **Save, Load, Open a save…** (below).
- **End this game** and **Exit** ask for a second press.

A game remembers the highest guidance level it used and the training switches it had on, and shows them beside its score at the end. Raising either asks first: "Your score will show this".

## High scores

Starquake's high-score table is kept between runs. Only games played here are kept, so it starts with the game's own entries, which are never kept. Each entry carries the highest guidance level its game had; a training game is shown, then left out; a game a save was loaded into is marked "loaded".

When the game asks for your name, a pad opens a card offering the names entered before, the most recent first. The west button switches to spelling one: up and down change a letter, and A takes it and moves to the next. On the keyboard, type the name as on the original.

## Saves

Saves work for every game, at any moment: in play, paused, or with the menu open.

- **F5** saves to the quick slot, and a notice says "Game saved".
- **F9** goes back to the last save point: the quick slot, or the save you started from or opened, until an F5 saves a newer one.
- The menu's row offers **Save** and **Load** for the same, and **Open a save…** for any save file (`.sav`, or a plain `.szx` snapshot, which loads the machine alone).
- **Quitting during a game** (closing the window, Exit in the menu, or Cmd-Q on a Mac) saves it. The next start asks "Continue where you left off?": Enter or A continues, anything else starts afresh.

A save of another game, or from another tape, is refused. A game a save was loaded into is marked "loaded" beside its score.

## Where files are kept

Everything the program keeps is in its data folder:

| System | Data folder |
|---|---|
| macOS | `~/Library/Application Support/ZX-Sidekick/` |
| Windows | `%APPDATA%\ZX Sidekick\data\` |
| Linux | `~/.local/share/zxsidekick/` (or `$XDG_DATA_HOME/zxsidekick/`) |

Setting the `ZX_SIDEKICK_DATA` environment variable moves it.

In it:

- `<game>/`, one folder a game (`starquake/`): its tape as you gave it (`tape.tap` or `tape.tzx`), the quick save `quick.sav`, the save made on quitting `quit.sav`, its kept lists (Starquake: `teleporter-codes.txt`, `doors-met.txt`, `high-score-names.txt`), its high-score table, `high-scores.txt`, and `how-to-play.txt` once How to play's *Show next time* has been changed.
- `tapes/`, for tapes played without a definition: one folder a tape, holding its saves.
- `games/`, for definitions you add. One there with the same id as one beside the program wins.

To start again from nothing, quit and delete a game's folder; you'll be asked for its tape again.

## Other games

A tape no definition knows still plays: open it with `sidekick TAPE` (the `.tap`, `.tzx` or the `.zip` it came in). It runs with the keyboard, and the pad on the Kempston joystick, without the panel, training switches or kept scores; saves still work.

Anyone can write a definition for another game: `AUTHORS-GUIDE.md` (in the source, `docs/definition-format.md`) says how.

## The command line

`sidekick` starts the only game installed, or a list to choose from when there are several. `sidekick GAME-ID` starts that game, `sidekick TAPE` plays a tape, `--games DIR` adds a folder of definitions, and `--load FILE` starts from a save. `sidekick --help` lists the rest.

On Windows the program opens no console window, so the command line's output (`--help`, `sidekick check` and the like) shows only when it's sent to a pipe or a file. In PowerShell: `.\sidekick.exe --help | Out-Host`, or `.\sidekick.exe --help > help.txt`. The same goes for a message saying why it couldn't start.
