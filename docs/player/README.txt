ZX SIDEKICK: STARQUAKE
======================

Starquake (Stephen Crow, Bubble Bus Software, 1985), played from your
own copy of the game. ZX Sidekick runs the original program, unchanged,
in a ZX Spectrum inside the app; you never see the Spectrum itself.

Not affiliated with or endorsed by the rights holders of Starquake or
the ZX Spectrum.


YOU NEED THE ORIGINAL GAME
--------------------------

This program contains no part of the original game, and no Spectrum
ROM. It runs your own copy of Starquake, a .tap tape, and will not
start without one.

World of Spectrum keeps Spectrum software available and removes titles
whose rights holders object. It lists Starquake as available. A dump of
a tape you own works just as well.

  https://worldofspectrum.net/


RUNNING IT
----------

Run zx-sidekick-starquake, or zx-sidekick-starquake.exe on Windows. If
it cannot find your tape it asks for it: pick the file, or drop it onto
the window. The zip World of Spectrum serves works as it is, with no
need to unpack it. The tape is then kept for next time in the usual
place for application data:

  Linux     ~/.local/share/zx-sidekick-starquake/
  macOS     ~/Library/Application Support/zx-sidekick-starquake/
  Windows   %APPDATA%\zx-sidekick-starquake\

It also finds the tape, or the zip, if you put it in the same folder as
the program, named starquake.tap or STARQUAK.TAP in any case. You can
name it on the command line as well.

macOS: the program is not signed, so macOS blocks it the first time.
In Terminal, in this folder, run:

  xattr -d com.apple.quarantine zx-sidekick-starquake

Or try to open it once, then allow it under System Settings, Privacy &
Security.


CONTROLS
--------

The keyboard is the Spectrum's: the letters, digits, Enter, Space,
Shift (Caps Shift) and right Ctrl (Symbol Shift) are the keys of the
same name, and the game reads them as it would on the real machine. At
the title screen the digits choose how to play, as the screen lists,
and every choice works.

On top of that there is a joystick that works whichever choice you
made: in play, the program presses the keys the game is listening for.

  Arrow keys           Move. They press no key of their own, so they
                       do nothing on the title screen.
  Left Ctrl, Alt,      Fire. On the title screen, start a game.
  full stop or comma
  Gamepad              D-pad or left stick to move. A (the bottom
                       button) is down, which lays a platform under
                       you; X (the left button) fires. Start pauses.
                       On the title screen Start or X starts a game,
                       and goes past the text that follows. Over USB
                       or Bluetooth.
                       Some controllers need the right mode: an 8BitDo
                       in Switch mode is detected but sends no input.

Pausing, with Start or with the game's own pause key (Space, or the key
you defined), stops the game where it is and the window says so. Any
key, a direction, fire or Start goes on.


GUIDANCE
--------

The panel beside the picture is for guidance: optional help, in levels
from 0 (none) to 5, each adding to the ones below.

  Level 1              The codes of the teleporters you have seen, once
                       you have entered their booths.
  Level 2              A map of the rooms you have visited: every edge
                       open or closed, walls inside a room (dashed where
                       a security door or a teleporter pad divides it),
                       your room, and the teleporters you have seen.
  Level 3              The missing core pieces marked on the map, in
                       rooms you have visited or not, and the core's
                       nine holes beside it: the pieces still needed,
                       the ones delivered dimmed, and a piece you carry
                       outlined.
  Level 4              Two routes over the ways you have walked: to the
                       nearest missing piece in pink, and to the core in
                       orange while you carry a piece it needs. Each is a
                       line on the map and an arrow in the border the
                       way to leave the room, marked "item" or "core",
                       with a legend under the map; and the code to
                       select when a teleporter is next.
  Level 5              The same two routes through the whole map,
                       dashed through rooms you have not visited, and
                       once you have been in a piece's room, to the
                       side of the room it lies in. It assumes you can
                       fly everywhere, that doors and tubes work, and
                       that lifts only go up: getting the items and
                       the battery for that is up to you. Training mode is still to be built.

  Esc, or Select       Open the guidance picker. The game waits while
  on a gamepad         it is open.
  Up and down          Choose a row.
  Left and right       Change the guidance level or training mode.
  Enter, or A          OK: keep what you changed. End this game and
                       Exit Starquake need a second press.
  Esc, B or Select     Cancel: leave the picker as it was when it
                       opened.

Raising the level, or turning training mode on, shows on that game's
score, so the picker asks first. When a game is over the panel says how
much help it had. Nothing is saved: every start is at level 0.


LEGAL
-----

Starquake is copyright (c) 1985 Stephen Crow / Bubble Bus Software.
ZX Sidekick is not affiliated with, endorsed by, or approved by the
rights holders.

The program is licensed under either the MIT licence (LICENSE-MIT) or
the Apache 2.0 licence (LICENSE-APACHE), at your option. That licence
covers only this program and grants no rights in Starquake itself.
THIRD-PARTY.txt lists the libraries built into the program and their
licences, among them the Z80 processor from RustZX. The text is set in
Inter, under the SIL Open Font License (LICENSE-Inter.txt).

@starquake started this project and steered it, and Claude,
Anthropic's AI assistant, wrote it. The source code is at:

  https://github.com/zx-sidekick/zx-sidekick-starquake
