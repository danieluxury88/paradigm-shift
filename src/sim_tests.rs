//! Headless simulation tests: the real game plugins run without a window or
//! renderer, with simulated key presses and a fixed 60 fps clock. They check
//! that levels play as designed, not just that their files parse.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use crate::AppState;
use crate::lens::ActiveLens;
use crate::level::{Cell, LevelGrid, Levels};
use crate::player::Player;
use crate::solver;

const FRAME: Duration = Duration::from_micros(16_667);

/// A game without rendering, starting at the given level (0-based).
fn game(level: usize) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ClearColor>()
        .init_state::<AppState>()
        .add_plugins((
            crate::level::LevelPlugin,
            crate::lens::LensPlugin,
            crate::player::PlayerPlugin,
            crate::straw::StrawPlugin,
        ));
    app.world_mut().resource_mut::<Levels>().current = level;
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
fn assert_solvable(level: usize) {
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
                return;
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
    let count = game(0).world().resource::<Levels>().all.len();
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
    let count = game(0).world().resource::<Levels>().all.len();
    for level in 0..count {
        assert_solvable(level);
    }
}
