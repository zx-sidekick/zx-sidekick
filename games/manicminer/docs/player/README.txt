ZX SIDEKICK: MANIC MINER
========================

Manic Miner (Matthew Smith, Bug-Byte Software, 1983), played from your
own copy of the game. ZX Sidekick runs the original program, unchanged,
in a ZX Spectrum inside the app; you never see the Spectrum itself.

Not affiliated with or endorsed by the rights holders of Manic Miner or
the ZX Spectrum.


YOU NEED THE ORIGINAL GAME
--------------------------

This program contains no part of the original game, and of the
Spectrum ROM only its character set, for the game's text. It runs your own copy of Manic Miner, a .tap tape of the original
Bug-Byte release, and will not start without one.

World of Spectrum keeps Spectrum software available and removes titles
whose rights holders object. A dump of a tape you own works just as
well.

  https://worldofspectrum.net/


RUNNING IT
----------

Run zx-sidekick-manicminer, or zx-sidekick-manicminer.exe on Windows.
If it cannot find your tape it asks for it: pick the file, or drop it
onto the window. A zip holding the tape works as it is. The tape is
then kept for next time in the usual place for application data:

  Linux     ~/.local/share/zx-sidekick-manicminer/
  macOS     ~/Library/Application Support/zx-sidekick-manicminer/
  Windows   %APPDATA%\zx-sidekick-manicminer\

It also finds the tape, or the zip, if you put it in the same folder as
the program, named manicminer.tap or manic.tap in any case. You can
name it on the command line as well.

The game prints its text (the cavern's name, the scores) with the
Spectrum ROM's letters, which this program includes; it needs nothing
else from a ROM. The character set is (c) Amstrad plc: Amstrad have
kindly given their permission for the redistribution of their
copyrighted material but retain that copyright.

macOS: the program is not signed, so macOS blocks it the first time.
In Terminal, in this folder, run:

  xattr -d com.apple.quarantine zx-sidekick-manicminer

Or try to open it once, then allow it under System Settings, Privacy &
Security.


CONTROLS
--------

The keyboard is the Spectrum's, and the game reads it as it would on
the real machine: Q, E, T, U or O left; W, R, Y, I or P right; the
bottom row (Space, Z to M) jumps. Enter starts a game on the title
screen. Shift (Caps Shift) with Space ends a game.

The game finds a Kempston joystick at its title screen, and ZX Sidekick
gives it one:

  Arrow keys           Move.
  Left Ctrl, Alt,      Jump.
  full stop or comma
  Gamepad              D-pad or left stick to move; A or X (the bottom
                       or left button) jumps. Start pauses, and on the
                       title screen starts a game. Over USB or
                       Bluetooth.

Pausing, with Start or with the game's own pause keys (A to G), stops
the game where it is and the window says so. Any key, a direction, jump
or Start goes on. The keys H to Enter turn the music on and off, as the
game always did.


TRAINING
--------

Esc, or Select on a gamepad, opens the training picker over the game,
which waits while it is open. Up and down choose a row; left and right
change it; Enter or A keeps what you chose, and Esc, B or Select closes
the picker without changing anything.

  Endless lives        A life lost is not taken from the lives left.
  Air stays full       The air never runs down, not even under the
                       light beam.
  Safe falls           A fall of any height lands Willy safely.
  No harm from         Guardians, Eugene, the Kong Beast and the
  guardians            Skylabs pass through Willy.
  No harm from         Nasty tiles do not kill.
  nasties
  Go to cavern         Choose a cavern and press Enter or A: the game
                       goes there with its own cheat and starts that
                       cavern again. The cheat's boot appears beside
                       your lives, as it always did.

End this game goes back to the title screen, and Exit Manic Miner
closes the program; each asks you to press Enter or A a second time.
