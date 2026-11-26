//! The player: keyboard movement, gravity, jumping and tile collisions.
//!
//! Collisions are resolved one axis at a time: move horizontally and push out
//! of walls, then move vertically and push out of floors and ceilings. Simple,
//! and good enough for a tile-based platformer.

use bevy::prelude::*;

use crate::AppState;
use crate::lens::{ActiveLens, Lens};
use crate::level::{Cell, LevelGrid};

const RUN_SPEED: f32 = 220.0;
const JUMP_SPEED: f32 = 520.0;
const GRAVITY: f32 = 1400.0;
const MAX_FALL_SPEED: f32 = 900.0;

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct Player {
    pub velocity: Vec2,
    pub on_ground: bool,
}

/// What the player is asking for during one frame. Kept separate from the
/// keyboard so the same physics can be driven by tests and the solver.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoveInput {
    /// -1 left, 0 none, 1 right.
    pub direction: i8,
    /// Jump was pressed this frame (not held).
    pub jump: bool,
}

/// Sent when the player is put back at the start (fall or R), so other
/// systems can reset the level, e.g. rebuild collapsed straw.
#[derive(Message)]
pub struct Respawned;

/// Sent when the player touches the goal: the level was really solved (the
/// development skip key does not send it).
#[derive(Message)]
pub struct LevelCompleted;

impl Player {
    /// Half width and half height of the player's collision box.
    pub const HALF_SIZE: Vec2 = Vec2::new(11.0, 14.0);
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Respawned>()
            .add_message::<LevelCompleted>()
            .add_systems(
                OnEnter(AppState::Playing),
                spawn_player.after(crate::level::spawn_level),
            )
            .add_systems(
                Update,
                (move_player, respawn_if_fallen, reach_goal)
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn spawn_player(mut commands: Commands, grid: Res<LevelGrid>) {
    let start = grid.find(Cell::Start).expect("validated at startup");
    commands.spawn((
        Player::default(),
        Sprite::from_color(Color::srgb(0.95, 0.95, 0.95), Player::HALF_SIZE * 2.0),
        Transform::from_translation(start.extend(1.0)),
        DespawnOnExit(AppState::Playing),
    ));
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<LevelGrid>,
    lens: Res<ActiveLens>,
    mut query: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut transform)) = query.single_mut() else {
        return;
    };
    let left = keys.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]);
    let right = keys.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]);
    let input = MoveInput {
        direction: match (left, right) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        },
        jump: keys.any_just_pressed([KeyCode::Space, KeyCode::ArrowUp, KeyCode::KeyW]),
    };
    let mut position = transform.translation.truncate();
    step(
        &mut position,
        &mut player,
        input,
        &grid,
        lens.0,
        time.delta_secs(),
    );
    transform.translation = position.extend(transform.translation.z);
}

/// One frame of movement physics: pure, so the game, the tests and the level
/// solver all share exactly the same rules.
pub fn step(
    position: &mut Vec2,
    player: &mut Player,
    input: MoveInput,
    grid: &LevelGrid,
    lens: Lens,
    delta: f32,
) {
    // Cap the time step so a slow frame cannot make the player skip through a tile.
    let dt = delta.min(1.0 / 30.0);

    player.velocity.x = RUN_SPEED * f32::from(input.direction);
    if input.jump && player.on_ground {
        player.velocity.y = JUMP_SPEED;
    }
    player.velocity.y = (player.velocity.y - GRAVITY * dt).max(-MAX_FALL_SPEED);

    // Horizontal pass.
    let horizontal_direction = player.velocity.x;
    position.x += horizontal_direction * dt;
    let mut horizontal_collision: Option<f32> = None;
    for (col, row, cell) in grid.cells_in_box(*position, Player::HALF_SIZE) {
        if !cell.is_solid_in(lens) {
            continue;
        }
        let tile = grid.cell_center(col, row);
        let limit = crate::TILE / 2.0 + Player::HALF_SIZE.x;
        let contact = if horizontal_direction > 0.0 {
            tile.x - limit
        } else {
            tile.x + limit
        };
        horizontal_collision = Some(match horizontal_collision {
            Some(previous) if horizontal_direction > 0.0 => previous.min(contact),
            Some(previous) => previous.max(contact),
            None => contact,
        });
    }
    if let Some(contact) = horizontal_collision {
        position.x = contact;
        player.velocity.x = 0.0;
    }

    // Vertical pass.
    let previous_feet = position.y - Player::HALF_SIZE.y;
    let previous_head = position.y + Player::HALF_SIZE.y;
    let horizontal_position = position.x;
    position.y += player.velocity.y * dt;
    player.on_ground = false;
    for (col, row, cell) in grid.cells_in_box(*position, Player::HALF_SIZE) {
        if !cell.is_solid_in(lens) {
            continue;
        }
        let tile = grid.cell_center(col, row);
        let tile_top = tile.y + crate::TILE / 2.0;
        let tile_bottom = tile.y - crate::TILE / 2.0;
        if player.velocity.y <= 0.0 && previous_feet >= tile_top {
            position.y = tile_top + Player::HALF_SIZE.y;
            player.on_ground = true;
            player.velocity.y = 0.0;
        } else if player.velocity.y > 0.0 && previous_head <= tile_bottom {
            position.y = tile_bottom - Player::HALF_SIZE.y;
            player.velocity.y = 0.0;
        } else {
            // Vertical movement entered the side of a tile. Keep the player
            // out of the wall instead of treating it like a floor or ceiling.
            let side = crate::TILE / 2.0 + Player::HALF_SIZE.x;
            position.x = if horizontal_position <= tile.x {
                tile.x - side
            } else {
                tile.x + side
            };
            player.velocity.x = 0.0;
        }
    }
}

/// True when a player box at `position` touches the goal.
pub fn touches_goal(grid: &LevelGrid, position: Vec2) -> bool {
    grid.cells_in_box(position, Player::HALF_SIZE)
        .iter()
        .any(|&(_, _, cell)| cell == Cell::Goal)
}

/// Falling into the void, or pressing R, puts the player back at the start.
pub fn respawn_if_fallen(
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<LevelGrid>,
    mut query: Query<(&mut Player, &mut Transform)>,
    mut respawned: MessageWriter<Respawned>,
) {
    let Ok((mut player, mut transform)) = query.single_mut() else {
        return;
    };
    let fell = transform.translation.y < grid.bottom() - crate::TILE * 2.0;
    if fell || keys.just_pressed(KeyCode::KeyR) {
        let start = grid.find(Cell::Start).expect("validated at startup");
        transform.translation = start.extend(transform.translation.z);
        player.velocity = Vec2::ZERO;
        respawned.write(Respawned);
    }
}

pub fn reach_goal(
    grid: Res<LevelGrid>,
    query: Query<&Transform, With<Player>>,
    mut next: ResMut<NextState<AppState>>,
    mut completed: MessageWriter<LevelCompleted>,
) {
    let Ok(transform) = query.single() else {
        return;
    };
    if touches_goal(&grid, transform.translation.truncate()) {
        completed.write(LevelCompleted);
        next.set(AppState::Card);
    }
}
