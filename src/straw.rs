//! Straw ground, for the straw man fallacy: it looks as solid as real ground
//! (in the "win the debate" lens), but as soon as the player puts weight on it
//! the whole straw structure creaks and collapses. It comes back when the
//! player respawns.

use bevy::prelude::*;

use crate::AppState;
use crate::lens::ActiveLens;
use crate::level::{Cell, LevelGrid, Tile};
use crate::player::{Player, Respawned};

/// Seconds between stepping on straw and the collapse. Long enough to notice
/// the creak, too short to run across a long straw bridge.
const COLLAPSE_DELAY: f32 = 0.25;

/// Colour of straw while it creaks, in every lens: the truth shows just
/// before it gives way.
const CREAKING: Color = Color::srgb(0.65, 0.45, 0.20);

/// Time left before the straw collapses, while it is creaking.
#[derive(Resource, Default)]
struct Collapse(Option<f32>);

pub struct StrawPlugin;

impl Plugin for StrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collapse>()
            .add_systems(OnEnter(AppState::Playing), |mut c: ResMut<Collapse>| {
                c.0 = None
            })
            .add_systems(
                Update,
                (step_on_straw, collapse_straw, restore_straw)
                    .chain()
                    .after(crate::player::move_player)
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn step_on_straw(
    grid: Res<LevelGrid>,
    player: Query<(&Player, &Transform)>,
    mut collapse: ResMut<Collapse>,
    mut tiles: Query<(&Tile, &mut Sprite)>,
) {
    let Ok((player, transform)) = player.single() else {
        return;
    };
    if collapse.0.is_some() || !player.on_ground {
        return;
    }
    // A thin box just under the player's feet.
    let feet = transform.translation.truncate() - Vec2::new(0.0, Player::HALF_SIZE.y + 1.0);
    let on_straw = grid
        .cells_in_box(feet, Vec2::new(Player::HALF_SIZE.x, 0.5))
        .iter()
        .any(|&(_, _, cell)| cell == Cell::Straw);
    if on_straw {
        collapse.0 = Some(COLLAPSE_DELAY);
        for (tile, mut sprite) in &mut tiles {
            if tile.0 == Cell::Straw {
                sprite.color = CREAKING;
            }
        }
    }
}

fn collapse_straw(
    time: Res<Time>,
    mut collapse: ResMut<Collapse>,
    mut grid: ResMut<LevelGrid>,
    mut tiles: Query<(&Tile, &mut Visibility)>,
) {
    let Some(left) = collapse.0.as_mut() else {
        return;
    };
    if *left > 0.0 {
        *left -= time.delta_secs();
        return;
    }
    for cell in grid.cells.iter_mut().flatten() {
        if *cell == Cell::Straw {
            *cell = Cell::Empty;
        }
    }
    for (tile, mut visibility) in &mut tiles {
        if tile.0 == Cell::Straw {
            *visibility = Visibility::Hidden;
        }
    }
}

fn restore_straw(
    mut respawned: MessageReader<Respawned>,
    mut collapse: ResMut<Collapse>,
    mut grid: ResMut<LevelGrid>,
    mut lens: ResMut<ActiveLens>,
    mut tiles: Query<(&Tile, &mut Visibility)>,
) {
    if respawned.read().count() == 0 {
        return;
    }
    collapse.0 = None;
    grid.cells = grid.original.clone();
    for (tile, mut visibility) in &mut tiles {
        if tile.0 == Cell::Straw {
            *visibility = Visibility::Inherited;
        }
    }
    // Repaint every tile, which also clears the creaking colour.
    lens.set_changed();
}
