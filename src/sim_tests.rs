//! Headless simulation tests: the real game plugins run without a window or
//! renderer, with simulated key presses and a fixed 60 fps clock. They check
//! that levels play as designed, not just that their files parse.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use crate::AppState;
use crate::level::{Cell, LevelGrid, Levels};
use crate::player::Player;

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

#[test]
fn straw_level_is_solvable_on_the_stepping_stones() {
    let mut app = game(2);
    run(&mut app, 30);
    // Switch to "Understand the other", where the stepping stones are solid.
    hold(&mut app, KeyCode::ShiftLeft);
    run(&mut app, 1);
    release(&mut app, KeyCode::ShiftLeft);

    // Run right and jump at each stone: a simple scripted player.
    hold(&mut app, KeyCode::ArrowRight);
    for frame in 0..600 {
        if frame % 20 == 0 {
            hold(&mut app, KeyCode::Space);
        }
        run(&mut app, 1);
        release(&mut app, KeyCode::Space);
        if *app.world().resource::<State<AppState>>().get() == AppState::Card {
            return;
        }
    }
    panic!(
        "did not reach the goal; ended at {}",
        player_position(&mut app)
    );
}

/// One step of a scripted solution.
enum Step {
    /// Walk right until x reaches the value, then press SHIFT.
    ShiftAt(f32),
    /// Walk right until x reaches `takeoff` while on the ground, jump (and
    /// SHIFT right after takeoff if `shift`), keep right for `air` frames,
    /// then wait to land.
    Hop {
        takeoff: f32,
        air: usize,
        shift: bool,
    },
}

fn on_ground(app: &mut App) -> bool {
    let mut query = app.world_mut().query::<&Player>();
    query.single(app.world()).expect("one player").on_ground
}

fn in_card(app: &App) -> bool {
    *app.world().resource::<State<AppState>>().get() == AppState::Card
}

fn tap(app: &mut App, key: KeyCode) {
    hold(app, key);
    run(app, 1);
    release(app, key);
}

/// Plays a level with a script, then walks right; panics with the step and
/// position if the goal is not reached.
fn solve(level: usize, script: &[Step]) {
    let mut app = game(level);
    run(&mut app, 30);
    let walk_until = |app: &mut App, x: f32, need_ground: bool, what: &str| {
        hold(app, KeyCode::ArrowRight);
        for _ in 0..600 {
            let p = player_position(app);
            if p.x >= x && (!need_ground || on_ground(app)) {
                return;
            }
            run(app, 1);
        }
        panic!(
            "level {}: never reached {what} at x={x}, stuck at {}",
            level + 1,
            player_position(app)
        );
    };
    for (i, step) in script.iter().enumerate() {
        match *step {
            Step::ShiftAt(x) => {
                walk_until(&mut app, x, false, &format!("step {i} (shift)"));
                tap(&mut app, KeyCode::ShiftLeft);
            }
            Step::Hop {
                takeoff,
                air,
                shift,
            } => {
                walk_until(&mut app, takeoff, true, &format!("step {i} (takeoff)"));
                tap(&mut app, KeyCode::Space);
                if shift {
                    tap(&mut app, KeyCode::ShiftLeft);
                }
                run(&mut app, air);
                release(&mut app, KeyCode::ArrowRight);
                for _ in 0..120 {
                    run(&mut app, 1);
                    if on_ground(&mut app) {
                        break;
                    }
                }
                let p = player_position(&mut app);
                assert!(
                    p.y > app.world().resource::<LevelGrid>().bottom(),
                    "level {}: fell at step {i} ({p})",
                    level + 1
                );
            }
        }
    }
    hold(&mut app, KeyCode::ArrowRight);
    for _ in 0..600 {
        if in_card(&app) {
            return;
        }
        run(&mut app, 1);
    }
    panic!(
        "level {}: goal not reached, ended at {}",
        level + 1,
        player_position(&mut app)
    );
}

#[test]
fn level_1_is_solvable() {
    solve(
        0,
        &[
            Step::ShiftAt(-48.0),
            Step::Hop {
                takeoff: 90.0,
                air: 19,
                shift: false,
            },
            Step::Hop {
                takeoff: 180.0,
                air: 33,
                shift: false,
            },
        ],
    );
}

#[test]
fn level_2_is_solvable() {
    solve(
        1,
        &[
            Step::Hop {
                takeoff: -300.0,
                air: 21,
                shift: false,
            },
            Step::Hop {
                takeoff: -200.0,
                air: 20,
                shift: true,
            },
            Step::Hop {
                takeoff: -104.0,
                air: 20,
                shift: true,
            },
            Step::Hop {
                takeoff: -8.0,
                air: 20,
                shift: true,
            },
            Step::Hop {
                takeoff: 88.0,
                air: 20,
                shift: true,
            },
            Step::Hop {
                takeoff: 184.0,
                air: 20,
                shift: true,
            },
        ],
    );
}

#[test]
fn level_4_is_solvable() {
    solve(
        3,
        &[
            Step::ShiftAt(-440.0),
            Step::Hop {
                takeoff: -10.0,
                air: 20,
                shift: false,
            },
            Step::Hop {
                takeoff: 85.0,
                air: 20,
                shift: false,
            },
        ],
    );
}
