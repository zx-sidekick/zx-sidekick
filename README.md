# ZX Sidekick

Play original ZX Spectrum games from your own copy, with a gamepad, optional training switches, a guidance panel, save games and kept high scores. The original program runs, unchanged, in an emulated Spectrum. What ZX Sidekick adds for each game comes from a **definition**, a TOML or JSON file anyone can write, in [`games/`](games/).

Early days: the plan is [`ZX-SIDEKICK.md`](ZX-SIDEKICK.md), the definition format is [`docs/definition-format.md`](docs/definition-format.md), and the reference game is [Starquake](docs/games/starquake.md).

[![ZX Sidekick running Starquake: the title menu, play, the guidance menu, the pause notice, saving and loading](docs/media/showcase.gif)](docs/media/showcase.mp4)

A minute of ZX Sidekick running Starquake from the player's own tape: the guidance menu (a level and two training switches), the question before the score shows more, the pause notice, saving with F5 and loading with F9, and the menu's saves row. The captions are the video's own. Starquake's map, core and routes aren't in the panel yet; their facts are still being found. [Full-size video (MP4)](docs/media/showcase.mp4).

**No game is included; you need your own copy.** ZX Sidekick is not affiliated with or endorsed by any game's rights holders. The Spectrum ROMs are included with Amstrad's permission ([`THIRD-PARTY.md`](THIRD-PARTY.md)).

## Building

```
scripts/check.sh
```

The emulated machine comes from the [zx-sidekick fork of RustZX](https://github.com/zx-sidekick/rustzx), fetched by Cargo. To work on it alongside this repository, see "Working on the engine" in `ZX-SIDEKICK.md`.

## Licence

MIT: see [`LICENSE`](LICENSE). This covers ZX Sidekick's own code, docs and definitions. Third-party material, including the Spectrum ROMs, is under its own terms: see [`THIRD-PARTY.md`](THIRD-PARTY.md).
