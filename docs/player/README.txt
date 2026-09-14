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
  Left Ctrl, Alt,      Fire.
  full stop or comma
  Gamepad              D-pad or left stick to move, any face or
                       shoulder button to fire, Start pauses. To go
                       on, move or fire: that is how the game itself
                       resumes, its pause key does not. Over USB or
                       Bluetooth.
                       Some controllers need the right mode: an 8BitDo
                       in Switch mode is detected but sends no input.


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
