# Reused code

Everything listed here is our own work, under the same licence (MIT OR Apache-2.0), from:

- **starquake-recompiled**, [starquake/starquake-recompiled](https://github.com/starquake/starquake-recompiled), at commit `15d43902f06aafc8ff09b91659fda2ce6e176015`;
- **the earlier ZX Sidekick build**, `starquake/zx-sidekick-starquake` (private), at commit `5b61d71`, which itself took the files marked below from starquake-recompiled at that commit.

It is generic Spectrum, ROM-behaviour or frontend code. **None of starquake-recompiled's Starquake game logic is reused** (`games/starquake/src` outside the frontend files below); see `GOAL.md`, rule 2. The processor is not our work: it is `rustzx-z80`, a dependency.

| Here | From | Changes |
|---|---|---|
| `LICENSE-MIT`, `LICENSE-APACHE`, `rust-toolchain.toml`, `deny.toml`, `.gitignore`, `assets/README.md` | the earlier build (the first five from starquake-recompiled) | `deny.toml` allows our rustzx fork as a git source |
