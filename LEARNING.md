# Learning Rust with Paradigm Shift

A reading guide to the code: where each Rust and Bevy concept appears, in a sensible order. Read the file, find the concept, then change something small and run `make run` to see what happens.

Last verified: 2026-11-25

## Suggested reading order

| Step | File | Rust concepts | Bevy concepts |
|---|---|---|---|
| 1 | `src/main.rs` | modules (`mod`), `use`, constants, `#[derive]` | `App`, plugins, `DefaultPlugins`, states (`AppState`), `Startup` systems |
| 2 | `src/lens.rs` | `enum`, `impl` blocks, `match`, `Copy` types, match guards (`if owner == lens.0`) | resources (`ActiveLens`), messages (`ShiftBlocked`), change detection (`is_changed`), run conditions (`in_state`) |
| 3 | `src/level.rs` | structs, `Result<T, String>` and `?`, iterators (`map`, `filter`, `find_map`, `collect`), `Option` and `let … else`, `include_str!`, `serde` + RON, unit tests (`#[cfg(test)]`) | spawning entities, components (`Tile`), `OnEnter`, `DespawnOnExit` |
| 3b | `src/campaign.rs`, `build.rs` | `include!` of generated code, `Result` with `map_err` and `ok_or_else`, closures returned from functions (`impl Fn`), slices (`levels[..=level]`) | custom `SystemParam` (`Current`), run conditions you write yourself (`unlocked`) |
| 4 | `src/player.rs` | mutable borrows (`&mut`), a pure function separated from its system (`step` vs `move_player`), `f32::from`, associated constants (`Player::HALF_SIZE`) | queries (`Query`, `single_mut`), `Time`, keyboard input (`ButtonInput<KeyCode>`), system ordering (`.chain()`, `.after()`), `NextState` |
| 5 | `src/hud.rs` | `format!`, `String` vs `&str` | UI (`Node`, `Text`, `TextFont`, `TextColor`), `children!`, marker components, `MessageReader` |
| 6 | `src/card.rs` | borrowing data from a resource | a second state's screen, built from level data |
| 7 | `src/i18n.rs` | `match` on a tuple `(key, language)`, `&'static str`, `#[serde(default)]`, `include_bytes!` | replacing the default font asset, rebuilding UI when a resource changes (`is_changed`, `is_added`) |
| 7b | `src/lives.rs` | `Option<u32>` with `as_mut` and `saturating_sub`, `with_children` loops | a feature plugin gated by a run condition, `Added<T>` queries, a third state's screen (`GameOver`), system ordering across plugins (`.after(respawn_if_fallen)`) |
| 7c | `src/save.rs`, `src/title.rs` | traits and `Box<dyn Trait>`, `#[cfg(target_arch = "wasm32")]` modules, `Mutex`, `Serialize`/`Deserialize`, a blanket `impl<T: Store> Store for Arc<T>` | reading resources inside `Plugin::build`, the `Last` schedule, a default state (`Title`) before the game loop |
| 7d | `src/prompt.rs` | `match` with guards on an enum from another crate (`Key`), `chars().count()` vs `len()` for accented text, `String::pop` | reading raw `KeyboardInput` messages for text entry, `MessageReader::clear`, a run condition with `not(in_state(..))` |
| 7e | `src/timer.rs` | `BTreeMap`, `Option::is_none_or`, integer arithmetic for `m:ss.t`, match guards (`Some(_) if ...`) | messages between plugins (`LevelCompleted`), `.after(reach_goal)`, filling a slot on another screen (`CardSlot`) |
| 7f | `src/synth.rs`, `src/audio.rs` | `f32` math (`sin`, `powf`, `exp`), writing a binary format by hand (`to_le_bytes`), `Arc<[u8]>`, `HashMap` | `Assets<AudioSource>`, `AudioPlayer` and `PlaybackSettings`, `AudioSink` volume, a custom `SystemParam` grouping several `MessageReader`s |
| 7g | `src/journal.rs` | building UI in loops with `with_children`, `BTreeSet`, wrapping indices with `%` | run conditions combined with `or_else`, a screen that remembers where it was opened from (`JournalView::back_to`), rebuilding UI when a resource changes |
| 8 | `src/straw.rs` | `Option<f32>` as a timer, `as_mut`, `iter_mut().flatten()` | systems ordered after another plugin's system (`.after`), `Visibility`, messages between plugins (`Respawned`) |
| 9 | `src/solver.rs` | `BinaryHeap` with a custom `Ord`, `HashSet` of tuple keys, `Option` chains, indices instead of references for a tree | none: it calls the same pure functions the systems use (`player::step`, `lens::try_shift`) |
| 10 | `src/sim_tests.rs` | tuples as return values, `unwrap_or_else` with `panic!`, `assert_eq!` with a message | running plugins headless (`MinimalPlugins`), a fixed clock (`TimeUpdateStrategy`), simulated input, querying the `World` directly |

## Exercises

Small changes, roughly from easiest to hardest:

1. Change `JUMP_SPEED` or `GRAVITY` in `src/player.rs` and feel the difference. Which value makes level 2 impossible?
2. Change the lens colours in `Lens::color` (`src/lens.rs`).
3. Add a new level in `levels/` (see "Adding a level" in the README). `make test` tells you if the map is broken.
4. Add a `C` cell that kills the player (sends them back to the start) in every lens: a new `Cell` variant, a `match` arm in `Cell::from_char`, and a check in `player.rs`. The compiler lists every `match` you forgot to update.
5. Count how many times the player shifted in a level and show it on the concept card. You need a new resource.
6. Let the journal open mid-level without restarting it: a pause that the gameplay systems respect.
7. Make straw collapse cell by cell, starting where the player stands, instead of all at once.

## Notes

- `cargo` commands take a while the first time because Bevy has hundreds of dependencies; later builds only recompile your code.
- The compiler's error messages usually say exactly what to change. Read the whole message, including the `help:` lines.
- Bevy changes its API between versions. This project uses Bevy 0.20; examples written for older versions may not compile unchanged.
