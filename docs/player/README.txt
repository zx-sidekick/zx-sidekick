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

The high-score table is kept in the same folder, in high-scores.txt,
with the guidance each game on it was played with. A game played with
any training switch is not kept. Delete the file to start again from
the table the tape came with; a file that cannot be read is left
alone.

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
and every choice works. Q there quits, as the screen says: answer Y and
the program closes once the game has said its goodbye.

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

  Level 1              The codes you have been shown, in a rail down
                       the panel's right, drawn in the game's own
                       letters: under TELEPORTERS, the code of every
                       booth you have entered; under DOORS, the three
                       chips each security door asked you for, once its
                       screen has shown them to you. At the top left,
                       the core's nine slots as a square of three by
                       three:
                       what each still wants, what is delivered, and
                       what you are carrying. A door's code stays
                       hidden until you have seen it, and the codes are
                       forgotten when a new game starts.
  Level 2              A map of the rooms you have walked through:
                       every edge open or closed, walls inside a room
                       (dashed where a security door or a teleporter
                       pad divides it), your room, and the teleporters
                       you have seen.
  Level 3              What you have seen lying in those rooms, each
                       drawn as what it does:

                         a disc with 0, 1, 2, 4 or 8
                                     a chip, which answers that number
                                     in a door's or a pyramid's code
                         a disc with ?
                                     a chip that answers any one number,
                                     and is used up doing it
                         a card      opens any door and any pyramid, and
                                     is never used up
                         a key       switches the teleporter pads in the
                                     room you carry it into
                         a ring      a pyramid takes it in exchange for
                                     one of the core's missing pieces

                       The colour says what a thing is for: lilac for
                       a door (the chips and the card), yellow the
                       key, white the ring. A piece the core still
                       wants is drawn as itself, in pink. What you
                       carry is not drawn: it is with you.
  Level 4              The same for the rooms you have never walked
                       through. The game puts every item out at the
                       start of a game, so it knows where they all are;
                       this tells you, ringed to say you have not been
                       there yourself.
  Level 5              A route to a missing piece, and one to the core
                       while you carry a piece it needs. Each is a line
                       on the map and an arrow in the border the way to
                       leave the room, marked "item" or "core": pink
                       to the piece, orange to the core, as the legend
                       beside the core's slots says. The code a route
                       needs next, a teleporter's to select or a
                       security door's to bring the chips for, is
                       outlined in the rail in the route's colour. A
                       door's code can only be outlined once you have
                       seen it. The routes run over
                       the whole map, dashed through rooms you have not
                       visited, and once you have been in a piece's
                       room, to the side of the room it lies in. They
                       assume you can fly everywhere, that doors and
                       tubes work, and that lifts only go up: getting
                       the items and the battery for that is up to you.
  Level 6              Everything the program knows: every teleporter
                       and door code whether you have been shown it or
                       not, and the whole planet's map, the rooms you
                       have never entered drawn dimmer.

  Training             Four switches of their own, all off until you
                       turn them on:
                         Time stands still      energy drains only
                                                when something touches
                                                you
                         Full gun and platforms those two bars never
                                                run down
                         Endless lives          losing a life does not
                                                cost one
                         No harm from enemies   enemies, deadly patches
                                                and zappers cost no
                                                energy and cannot kill

  F11                  Leave fullscreen, or go back to it. The game
                       starts fullscreen; the window it leaves you with
                       is as large as your screen allows.
  Esc, or Select       Open the guidance picker. The game waits while
  on a gamepad         it is open.
  Tab, or the top      At levels 5 and 6, switch the route to a missing
  face button          piece between the three nearest, and from the
                       third back to the nearest. It goes back to the
                       nearest by itself when the piece is picked up or
                       is no longer one of the three nearest.
  Up and down          Choose a row.
  Left and right       Change the guidance level, or turn a training
                       switch off or on.
  Enter, or A          OK: keep what you changed. End this game and
                       Exit Starquake need a second press. When a change
                       would show on your score, the picker says so and
                       waits: Enter or A goes ahead, Esc, B or Select
                       cancel.
  Esc, B or Select     Cancel: leave the picker as it was when it
                       opened.

  The pad's letters are its own: A keeps and B cancels whatever pad you
  have, which is the bottom button on an Xbox pad and the right one on
  a Switch pad, since that is where each has its A. A PlayStation pad
  keeps with the cross and cancels with the circle. What the window
  draws follows the maker your pad reports, so a controller with a mode
  switch shows what its own mode says.

Raising the level, or turning a training switch on, shows on that
game's score, so the picker asks first, and the score names each switch
you used. When a game is over the panel says how
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
