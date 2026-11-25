# Paradigm Shift

A puzzle platformer about seeing the same world through different paradigms. Press SHIFT to change lens: ground that is solid in one way of seeing is only a faint outline in the other, and no single lens reaches the goal. Each level makes you experience one idea from philosophy, anthropology, communication or argumentation, then a concept card names it.

The longer-term story follows one traveler across multiple worlds, entering the perspectives of the people who live in them. The opening world has six levels; two further worlds, The Listening City and The Common Ground, extend the playable draft to twelve levels.

Made in Rust with [Bevy](https://bevyengine.org) for [GitHub Game Off 2026](https://itch.io/jam/game-off-2026) (theme: SHIFT). Playable in English, German and Spanish.

Last verified: 2026-11-23

## Controls

| Key | Action |
|---|---|
| ← → or A D | Move |
| Space, ↑ or W | Jump |
| SHIFT | Change lens (refused if the other view has a wall where you stand) |
| R | Restart the level |
| L | Change language (English, Deutsch, Español) |
| ENTER | Continue after a concept card |
| N | Skip the level (development builds only) |

## Levels

| # | World | Level | Perspectives | Concept |
|---|---|---|---|---|
| 1 | The First Crossing | Two ways of seeing | Order / Freedom | Paradigm (Kuhn) |
| 2 | The First Crossing | Inside and outside | Outsider (etic) / Insider (emic) | Emic and etic (Pike, Harris) |
| 3 | The First Crossing | Men of straw | Win the debate / Understand the other | Straw man |
| 4 | The First Crossing | Either, or…? | Either/or / Look for more | False dilemma |
| 5 | The First Crossing | The unfitting star | The old map / The new map | Anomaly (Kuhn) |
| 6 | The First Crossing | The shared crossing | What I was told / What I found | Synthesis |
| 7 | The Listening City | Signal and noise | The sender / The listener | Signal and noise (Shannon) |
| 8 | The Listening City | The silent reply | What was said / What was left unsaid | One cannot not communicate (Watzlawick) |
| 9 | The Listening City | A fair reading | The quick reply / The charitable reading | Principle of charity |
| 10 | The Common Ground | The borrowed answer | The expert's map / The residents' account | Appeal to authority |
| 11 | The Common Ground | The unassigned seat | The rule maker / The person affected | Veil of ignorance (Rawls) |
| 12 | The Common Ground | The nearby need | The shared rule / The nearby need | Ethics of care |

The planned concepts and the full design are in [docs/design.md](docs/design.md).

## Development

Requirements: Rust (stable, via rustup) with the `wasm32-unknown-unknown` target, which `rust-toolchain.toml` requests, and [trunk](https://trunkrs.dev) for the web build (`cargo install --locked trunk`). Native builds on Linux also need the ALSA, udev, Wayland and xkbcommon development packages, plus clang and lld (see `.cargo/config.toml`).

```bash
make run     # play natively in a window
make test    # unit and simulation tests: maps, translations, and a bot that solves every level
make check   # fmt, clippy and tests
make serve   # web build with live reload at http://127.0.0.1:8080
make web     # optimized web build zipped in build/ for itch.io
```

`make run` enables the `dev` feature (Bevy dynamic linking) for fast rebuilds; release and web builds never use it.

### Letting a coding agent see the running game

`make brp` runs the native game with the `brp` feature: [bevy_brp_extras](https://crates.io/crates/bevy_brp_extras) serves the Bevy Remote Protocol on `127.0.0.1:15702`. A coding agent with the [bevy_brp_mcp](https://crates.io/crates/bevy_brp_mcp) server (`cargo install --locked bevy_brp_mcp --version 0.23.0`, registered as the `brp` MCP server) can then list entities, read the `Player` component and the `ActiveLens` resource, take screenshots and send keys, for example to replay a level after a change. Only types registered in `src/brp.rs` are visible; register more there when needed. The feature is for local development only: the web build and `make web` never include it.

## Adding a level

1. Copy a file in `levels/` and edit it. Map legend: `#` solid in every lens, `A` solid only in the first lens, `B` solid only in the second, `S` straw (looks solid, collapses when stood on), `P` start, `G` goal, `.` empty. All rows must have the same width, with exactly one `P` and one `G`.
2. Write every text in English (`en`), German (`de`) and Spanish (`es`).
3. Add it to `LEVEL_FILES` in `src/level.rs`.
4. Add a scripted solution to `src/sim_tests.rs` (see `level_1_is_solvable`): a headless bot plays the level and the test fails if it cannot reach the goal.
5. Run `make test`: a broken map, a missing translation or an unsolvable level fails there with the file name and the reason.

The player jumps about three tiles high and five tiles far.

## Learning Rust with this project

[LEARNING.md](LEARNING.md) maps each part of the code to the Rust and Bevy concepts it uses.

## Languages

Level texts (titles, hints, lens names, concept cards) are in each level file; the few interface strings are in `ui()` in `src/i18n.rs`. A missing German or Spanish text falls back to English in the game, but fails `make test`. The German and Spanish texts were drafted with AI help and still need a review by native speakers.

## AI disclosure

Game Off asks entrants to disclose AI use. This project's code and texts were written with the help of Claude Code (Anthropic); design decisions and the final review are the author's.

## Credits and license

Font: [Atkinson Hyperlegible](https://www.brailleinstitute.org/freefont/) by the Braille Institute, under the SIL Open Font License (`assets/fonts/OFL.txt`). Code: MIT.
