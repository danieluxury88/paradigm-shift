//! Headless simulation tests: the real game plugins run without a window or
//! renderer, with simulated key presses and a fixed 60 fps clock. They check
//! that levels play as designed, not just that their files parse.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use crate::AppState;
use crate::campaign::{Campaign, Progress};
use crate::i18n::Language;
use crate::lens::ActiveLens;
use crate::level::{Cell, LevelGrid};
use crate::lives::Lives;
use crate::player::Player;
use crate::save::Store;
use crate::solver;

const FRAME: Duration = Duration::from_micros(16_667);

/// A game without rendering, starting at the given level (0-based).
fn game(level: usize) -> App {
    game_with_store(level, Box::new(crate::save::MemoryStore::default()))
}

/// Like `game`, with the save kept in the given store.
fn game_with_store(level: usize, store: Box<dyn crate::save::Store>) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ClearColor>()
        // Screens need a language and a bold font; no text is rendered here.
        .init_resource::<crate::i18n::Language>()
        .insert_resource(crate::i18n::Fonts {
            bold: Handle::default(),
        })
        .init_state::<AppState>()
        .add_message::<KeyboardInput>()
        .insert_resource(crate::save::SaveStore(store))
        .add_plugins((
            crate::campaign::CampaignPlugin,
            crate::save::SavePlugin,
            crate::level::LevelPlugin,
            crate::lens::LensPlugin,
            crate::player::PlayerPlugin,
            crate::straw::StrawPlugin,
            crate::lives::LivesPlugin,
            crate::card::CardPlugin,
            crate::prompt::PromptPlugin,
            crate::timer::TimerPlugin,
            crate::journal::JournalPlugin,
            crate::gadgets::GadgetsPlugin,
        ));
    app.world_mut().resource_mut::<Progress>().level = level;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    app.update();
    app.update();
    app
}

fn player_position(app: &mut App) -> Vec2 {
    let mut query = app.world_mut().query_filtered::<&Transform, With<Player>>();
    query
        .single(app.world())
        .expect("one player")
        .translation
        .truncate()
}

fn straw_cells(app: &App) -> usize {
    let grid = app.world().resource::<LevelGrid>();
    grid.cells
        .iter()
        .flatten()
        .filter(|&&c| c == Cell::Straw)
        .count()
}

fn hold(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

fn release(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
}

/// Runs `frames` updates. `just_pressed` is cleared after each frame, as
/// Bevy's input plugin would do, so a press acts once.
fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
}

#[test]
fn holding_right_moves_right_steadily() {
    let mut app = game(0);
    run(&mut app, 30); // land on the ground first
    let mut previous = player_position(&mut app);
    hold(&mut app, KeyCode::ArrowRight);
    for _ in 0..60 {
        run(&mut app, 1);
        let now = player_position(&mut app);
        assert!(now.x >= previous.x, "moved left: {previous} -> {now}");
        previous = now;
    }
    // One second at 220 px/s, minus nothing: the first stretch is flat.
    assert!(previous.x > -300.0, "too slow: {previous}");
}

#[test]
fn jumping_into_the_level_one_wall_does_not_pass_through() {
    let mut app = game(0);
    run(&mut app, 30);

    hold(&mut app, KeyCode::ArrowRight);
    run(&mut app, 85);
    tap(&mut app, KeyCode::Space);
    run(&mut app, 120);

    let position = player_position(&mut app);
    assert!(
        position.x < 0.0,
        "jumping must not pass the vertical wall; ended at {position}"
    );
}

#[test]
fn straw_collapses_under_the_player_and_returns_after_respawn() {
    let mut app = game(2);
    let original = straw_cells(&app);
    assert!(original > 0, "level 3 has straw");

    run(&mut app, 30);
    hold(&mut app, KeyCode::ArrowRight);
    let mut collapsed = false;
    for _ in 0..240 {
        run(&mut app, 1);
        if straw_cells(&app) == 0 {
            collapsed = true;
            break;
        }
    }
    assert!(
        collapsed,
        "walking onto the straw bridge should collapse it"
    );
    release(&mut app, KeyCode::ArrowRight);

    // The player falls through and is put back at the start with the straw restored.
    let mut restored = false;
    for _ in 0..240 {
        run(&mut app, 1);
        if straw_cells(&app) == original {
            restored = true;
            break;
        }
    }
    assert!(restored, "straw comes back after the respawn");
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Playing
    );
}

fn in_card(app: &App) -> bool {
    *app.world().resource::<State<AppState>>().get() == AppState::Card
}

fn tap(app: &mut App, key: KeyCode) {
    hold(app, key);
    run(app, 1);
    release(app, key);
}

/// Finds a route with the solver, then plays it in the real app: every
/// plugin, every system order, the same fixed clock.
fn assert_solvable(level: usize) -> App {
    let (mut app, grid, start, delta) = landed_start(level);
    let plan = solver::solve(&grid, start, delta)
        .unwrap_or_else(|| panic!("level {}: the solver found no route", level + 1));

    for (i, action) in plan.iter().enumerate() {
        release(&mut app, KeyCode::ArrowLeft);
        release(&mut app, KeyCode::ArrowRight);
        match action.direction {
            1 => hold(&mut app, KeyCode::ArrowRight),
            -1 => hold(&mut app, KeyCode::ArrowLeft),
            _ => {}
        }
        if action.jump {
            hold(&mut app, KeyCode::Space);
        }
        if action.shift {
            hold(&mut app, KeyCode::ShiftLeft);
        }
        for _ in 0..solver::FRAMES_PER_ACTION {
            run(&mut app, 1);
            release(&mut app, KeyCode::Space);
            release(&mut app, KeyCode::ShiftLeft);
            if in_card(&app) {
                return app;
            }
        }
        if i + 1 == plan.len() {
            // Touching the goal sets the next state, which applies at the
            // start of the following update.
            run(&mut app, 1);
        }
    }
    assert!(
        in_card(&app),
        "level {}: the replayed route missed the goal, ended at {}",
        level + 1,
        player_position(&mut app)
    );
    app
}

/// The start state of a level after the player has landed.
/// A level after the player has landed, with what the solver needs: the
/// grid, the exact start state and the frame time the app uses.
fn landed_start(level: usize) -> (App, LevelGrid, solver::State, f32) {
    let mut app = game(level);
    run(&mut app, 30);
    let mut query = app.world_mut().query::<(&Player, &Transform)>();
    let (player, transform) = query.single(app.world()).expect("one player");
    let state = solver::State {
        position: transform.translation.truncate(),
        player: player.clone(),
        lens: app.world().resource::<ActiveLens>().0,
    };
    let delta = app.world().resource::<Time>().delta_secs();
    let grid = app.world().resource::<LevelGrid>().clone();
    (app, grid, state, delta)
}

/// Levels (1-based) known to be solvable in one lens. Their maps still need a
/// redesign; the list keeps new levels from breaking the promise while that
/// is pending. Remove a level from here once its map needs SHIFT.
const KNOWN_SINGLE_LENS: &[usize] = &[2, 5, 12];

/// The core promise of the game: no single lens reaches the goal.
#[test]
fn no_level_is_solvable_in_one_lens() {
    let count = game(0).world().resource::<Campaign>().levels.len();
    let mut single_lens = Vec::new();
    for level in 0..count {
        let (_, grid, start, delta) = landed_start(level);
        if solver::solve_without_shift(&grid, start, delta).is_some() {
            single_lens.push(level + 1);
        }
    }
    assert_eq!(
        single_lens, KNOWN_SINGLE_LENS,
        "levels solvable without SHIFT (update KNOWN_SINGLE_LENS only after a map change)"
    );
}

#[test]
fn every_level_is_solvable() {
    let count = game(0).world().resource::<Campaign>().levels.len();
    for level in 0..count {
        let _ = assert_solvable(level);
    }
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn lives_left(app: &App) -> Option<u32> {
    app.world().resource::<Lives>().left
}

/// Moves to another level the way the game does: set the progress, then
/// re-enter `Playing`.
fn go_to_level(app: &mut App, level: usize) {
    app.world_mut().resource_mut::<Progress>().level = level;
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Playing);
    run(app, 2);
}

#[test]
fn lives_unlock_at_level_4() {
    assert_eq!(lives_left(&game(2)), None, "level 3 has no lives yet");
    assert_eq!(lives_left(&game(3)), Some(5));
}

#[test]
fn restarting_costs_a_life_and_running_out_restarts_the_level_in_world_1() {
    let mut app = game(3);
    for expected in (1..5).rev() {
        tap(&mut app, KeyCode::KeyR);
        assert_eq!(lives_left(&app), Some(expected));
    }
    tap(&mut app, KeyCode::KeyR);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::GameOver);

    tap(&mut app, KeyCode::Enter);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Playing);
    assert_eq!(app.world().resource::<Progress>().level, 3);
    assert_eq!(lives_left(&app), Some(5));
}

#[test]
fn running_out_in_world_2_returns_to_its_first_level() {
    let mut app = game(7);
    assert_eq!(lives_left(&app), Some(4));
    for _ in 0..4 {
        tap(&mut app, KeyCode::KeyR);
    }
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::GameOver);
    tap(&mut app, KeyCode::Enter);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<Progress>().level, 6);
    assert_eq!(lives_left(&app), Some(4));
}

#[test]
fn lives_carry_over_within_a_world_and_refill_in_the_next() {
    let mut app = game(3);
    tap(&mut app, KeyCode::KeyR);
    go_to_level(&mut app, 4);
    assert_eq!(lives_left(&app), Some(4), "same world keeps the budget");
    go_to_level(&mut app, 9);
    assert_eq!(lives_left(&app), Some(3), "a new world refills it");
}

#[test]
fn progress_is_saved_and_restored_in_the_next_session() {
    let store = std::sync::Arc::new(crate::save::MemoryStore::default());
    let mut first = game_with_store(0, Box::new(store.clone()));
    assert!(store.load().is_none(), "loading alone writes nothing");
    go_to_level(&mut first, 5);
    assert!(store.load().is_some(), "the new level was saved");

    // A new session: build without forcing a level, as the real game does.
    let mut second = App::new();
    second
        .add_plugins((MinimalPlugins, StatesPlugin))
        .init_resource::<crate::i18n::Language>()
        .insert_resource(crate::save::SaveStore(Box::new(store)))
        .add_plugins((crate::campaign::CampaignPlugin, crate::save::SavePlugin));
    assert_eq!(second.world().resource::<Progress>().level, 5);
    assert!(second.world().resource::<crate::save::SaveFound>().0);
}

/// Types text the way a keyboard would: one pressed `KeyboardInput` per key.
fn type_keys(app: &mut App, keys: &[Key]) {
    for key in keys {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: key.clone(),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        run(app, 1);
    }
}

#[test]
fn level_5_asks_for_the_name_and_the_texts_use_it() {
    let mut app = game(3);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Card);
    run(&mut app, 2);
    tap(&mut app, KeyCode::Enter);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Prompt);

    let ada = ["A", "d", "a"].map(|c| Key::Character(c.into()));
    type_keys(&mut app, &ada);
    type_keys(&mut app, &[Key::Enter]);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Playing);
    run(&mut app, 2);
    let tiles = app
        .world_mut()
        .query::<&crate::level::Tile>()
        .iter(app.world())
        .count();
    assert!(
        tiles > 0,
        "the level's tiles are spawned after the question"
    );
    player_position(&mut app); // panics unless exactly one player exists

    let progress = app.world().resource::<Progress>().clone();
    assert_eq!(progress.level, 4);
    assert_eq!(progress.profile.name.as_deref(), Some("Ada"));
    let campaign = app.world().resource::<Campaign>();
    let example = &campaign.levels[4].data.card.example;
    let text = crate::campaign::fill(example.get(Language::En), &progress.profile, Language::En);
    assert!(text.starts_with("Ada "), "{text}");
}

#[test]
fn the_question_is_not_asked_again_once_answered() {
    let mut app = game(3);
    app.world_mut().resource_mut::<Progress>().profile.name = Some("Ada".into());
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Card);
    run(&mut app, 2);
    tap(&mut app, KeyCode::Enter);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Playing);
}

fn best_time(app: &App, level: usize) -> Option<f32> {
    let id = &app.world().resource::<Campaign>().levels[level].id;
    app.world()
        .resource::<Progress>()
        .best_times
        .get(id)
        .copied()
}

#[test]
fn the_timer_records_best_times_once_unlocked() {
    let app = assert_solvable(5);
    assert_eq!(best_time(&app, 5), None, "level 6 has no timer yet");

    let app = assert_solvable(6);
    let time = best_time(&app, 6).expect("level 7 records a time");
    assert!(time > 1.0 && time < 60.0, "plausible time: {time}");
    let last = app
        .world()
        .resource::<crate::timer::LastRun>()
        .0
        .expect("a run");
    assert!(last.is_record());
}

#[test]
fn solved_levels_go_into_the_journal_which_j_opens_from_the_card() {
    let mut app = assert_solvable(0);
    assert_eq!(app.world().resource::<Progress>().completed.len(), 1);
    // Level 1 has no journal yet: J does nothing on its card.
    run(&mut app, 1);
    tap(&mut app, KeyCode::KeyJ);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Card);

    let mut app = assert_solvable(2);
    run(&mut app, 1);
    tap(&mut app, KeyCode::KeyJ);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Journal);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::Escape);
    run(&mut app, 1);
    assert_eq!(state(&app), AppState::Card, "ESC returns to the card");
}

#[test]
fn the_steelman_reinforces_the_other_lens_platform_once_per_level() {
    use crate::gadgets::Charges;
    let cell =
        |app: &App, col: usize, row: usize| app.world().resource::<LevelGrid>().cells[row][col];

    // Level 8 has no gadget yet.
    let mut app = game(7);
    tap(&mut app, KeyCode::KeyE);
    assert_eq!(
        app.world().resource::<Charges>().0,
        0,
        "locked before level 9"
    );

    // Level 9: walk under the "B" platform at row 9, columns 16-17.
    let mut app = game(8);
    run(&mut app, 30);
    hold(&mut app, KeyCode::ArrowRight);
    while player_position(&mut app).x < 16.0 {
        run(&mut app, 1);
    }
    release(&mut app, KeyCode::ArrowRight);
    assert_eq!(cell(&app, 16, 9), Cell::LensOnly(crate::lens::Lens::Second));

    tap(&mut app, KeyCode::KeyE);
    run(&mut app, 1);
    assert_eq!(cell(&app, 16, 9), Cell::Steel);
    assert_eq!(cell(&app, 17, 9), Cell::Steel, "the whole platform");
    let painted = app
        .world_mut()
        .query::<(&crate::level::GridPos, &Sprite)>()
        .iter(app.world())
        .find(|(pos, _)| pos.0 == 16 && pos.1 == 9)
        .map(|(_, sprite)| sprite.color);
    assert_eq!(painted, Some(crate::lens::STEEL), "the steel is visible");
    assert_eq!(app.world().resource::<Charges>().0, 0);

    tap(&mut app, KeyCode::KeyE);
    assert_eq!(
        cell(&app, 19, 8),
        Cell::LensOnly(crate::lens::Lens::Second),
        "no charge left for a second platform"
    );

    tap(&mut app, KeyCode::KeyR);
    run(&mut app, 1);
    assert_eq!(cell(&app, 16, 9), Cell::LensOnly(crate::lens::Lens::Second));
    assert_eq!(
        app.world().resource::<Charges>().0,
        1,
        "a respawn resets the level"
    );
}
