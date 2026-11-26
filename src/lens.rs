//! Lenses: the two ways of seeing a level. SHIFT toggles between them, which
//! changes which tiles are solid and how the scene looks.

use bevy::prelude::*;

use crate::AppState;
use crate::level::{Cell, LevelGrid, Tile};
use crate::player::Player;

/// One of the two lenses a level offers. Their names come from the level data
/// (for example "Order" / "Freedom"); the code only needs to know which one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Lens {
    #[default]
    First,
    Second,
}

impl Lens {
    pub fn other(self) -> Lens {
        match self {
            Lens::First => Lens::Second,
            Lens::Second => Lens::First,
        }
    }

    /// Main colour of the tiles that belong to this lens.
    pub fn color(self) -> Color {
        match self {
            Lens::First => Color::srgb(0.25, 0.70, 0.75),
            Lens::Second => Color::srgb(0.95, 0.55, 0.25),
        }
    }

    /// Background tint, so the player always feels which lens is active.
    fn background(self) -> Color {
        match self {
            Lens::First => Color::srgb(0.07, 0.11, 0.14),
            Lens::Second => Color::srgb(0.14, 0.09, 0.07),
        }
    }
}

/// The lens currently in use. A resource: one value shared by the whole game.
#[derive(Resource, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct ActiveLens(pub Lens);

/// Sent (as a Bevy message) when switching was refused because the player
/// would end up inside a wall. The HUD uses it to give feedback.
#[derive(Message)]
pub struct ShiftBlocked;

pub struct LensPlugin;

impl Plugin for LensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLens>()
            .add_message::<ShiftBlocked>()
            .add_systems(OnEnter(AppState::Playing), reset_lens)
            .add_systems(
                Update,
                // SHIFT is applied before movement, so the frame's physics
                // always uses the lens the player asked for; the solver
                // relies on this order.
                (switch_lens.before(crate::player::move_player), paint_tiles)
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn reset_lens(mut lens: ResMut<ActiveLens>) {
    lens.0 = Lens::First;
}

fn switch_lens(
    keys: Res<ButtonInput<KeyCode>>,
    grid: Res<LevelGrid>,
    mut lens: ResMut<ActiveLens>,
    player: Query<&Transform, With<Player>>,
    mut blocked: MessageWriter<ShiftBlocked>,
) {
    if !(keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::ShiftRight)) {
        return;
    }
    let Ok(transform) = player.single() else {
        return;
    };
    match try_shift(&grid, transform.translation.truncate(), lens.0) {
        Some(next) => lens.0 = next,
        None => {
            blocked.write(ShiftBlocked);
        }
    }
}

/// The lens after a SHIFT at `position`, or `None` if it is refused.
/// Rule: you cannot shift into a wall. The other view has to have room for you.
pub fn try_shift(grid: &LevelGrid, position: Vec2, current: Lens) -> Option<Lens> {
    let next = current.other();
    (!grid.overlaps_solid(position, Player::HALF_SIZE, next)).then_some(next)
}

/// Recolours tiles and the background whenever the active lens changes.
fn paint_tiles(
    lens: Res<ActiveLens>,
    mut clear: ResMut<ClearColor>,
    mut tiles: Query<(&Tile, &mut Sprite)>,
) {
    // `is_changed` is true on the first frame and after every switch.
    if !lens.is_changed() {
        return;
    }
    clear.0 = lens.0.background();
    for (tile, mut sprite) in &mut tiles {
        sprite.color = match tile.0 {
            Cell::Solid => Color::srgb(0.80, 0.80, 0.78),
            Cell::Goal => Color::srgb(0.95, 0.85, 0.30),
            // Straw passes for real ground in the first lens; the second
            // lens shows it for what it is.
            Cell::Straw if lens.0 == Lens::First => Color::srgb(0.80, 0.80, 0.78),
            Cell::Straw => Color::srgba(0.85, 0.70, 0.35, 0.55),
            // Tiles of the active lens are solid and bright; the others are
            // only a faint outline of what another view would see.
            Cell::LensOnly(owner) if owner == lens.0 => owner.color(),
            Cell::LensOnly(owner) => owner.color().with_alpha(0.12),
            Cell::Empty | Cell::Start => Color::NONE,
        };
    }
}
