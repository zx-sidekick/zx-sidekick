# ZX Sidekick

ZX Sidekick runs original ZX Spectrum games, from the player's own copy, in an emulated machine, and adds training switches, gamepad controls and guidance around them. What it does for a game comes from that game's **definition** in `games/`. The plan is `ZX-SIDEKICK.md`; the definition format is `docs/definition-format.md`.

## Working with copyrighted game code

Games and the Spectrum ROM belong to their rights holders. These rules keep the project on the right side of that. They apply to every session, every game, and every file, issue, PR and comment.

1. **A legally distributed copy, never shared.** Work from the supported dump named by SHA-1 in the game's definition. For Starquake, that's the copy World of Spectrum distributes. Keep it in `assets/` (ignored by git). Never commit, upload, attach or share it. Players download their own.
2. **Disassembly is for finding facts and understanding how the game works, nothing else.** It is never used to recreate, port or translate the game, and never published. It lives only in `.scratch/` (ignored by git). None of it goes into a commit, a doc, an issue, a PR, a comment, or a summary in chat.
3. **Publish facts only, in our own words.** Addresses, what the bytes there mean, and short patch bytes with the bytes they replace (`expect`). Write each fact in `docs/re/<game>.md`, marked **confirmed** (seen running), **read** (understood from the disassembly) or **guess**. No listings, no code excerpts. No extracted maps, text, graphics or sound either: the program reads those from the player's tape at run time.
4. **Use whatever is quickest and most accurate.** Our own disassembly, tracing, memory search, watchpoints: no method is preferred. Read a public disassembly only if it's published with the rights holder's permission, or holds annotations only (for example a SkoolKit control file applied to our own copy). Never download an unlicensed full listing.
5. **Never circumvent copy protection.** A game with a protected loader (Speedlock and the like) loads in the emulator as it would on a Spectrum. Nobody decrypts or strips it to find facts.
6. **No money on any game.** No donations, tiers or paid builds tied to a game. Ask the rights holder before anything commercial; asking for permission is always the better option.

The repository never holds tapes, snapshots, ROM files or the mockup images. The repo guard (`.github/workflows/repo-guard.yml`) fails on any `.tap`, `.tzx`, `.z80`, `.sna`, `.szx` or `.rom`. The ROM the emulator needs comes inside the `rustzx-core` dependency, under Amstrad's permission (`THIRD-PARTY.md`).

## The engine

The machine is `rustzx-core` and `rustzx-z80` from the [zx-sidekick fork of RustZX](https://github.com/zx-sidekick/rustzx), a git dependency pinned to one commit in `Cargo.toml`. Changes to the machine are made there, as numbered patches recorded in its `ZX-SIDEKICK.md`, each by pull request. Then bump the `rev` here. "Working on the engine" in `ZX-SIDEKICK.md` says how to build against a local checkout.

## Checks

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
