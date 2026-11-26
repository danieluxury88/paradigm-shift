//! Gadgets: tools the traveler gains later in the journey, each unlocked by
//! a level in `levels/worlds.ron` and shown in the HUD's gadget slot.
//!
//! The first is the Steelman (`Feature::Steelman`): E reinforces the nearest
//! platform of the other lens, so it holds in both. It is the principle of
//! charity as a mechanic: take the other view at its strongest. It has one
//! charge per level; respawning resets the level, which brings back both the
//! charge and the platform's original state. Gadgets are optional, so the
//! solver tests never use them and still prove every level needs SHIFT.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Current, Feature, unlocked};
use crate::hud::{HudSlot, Slot, new_slot};
use crate::i18n::{Language, Ui, ui};
use crate::lens::{ActiveLens, Lens};
use crate::level::{Cell, GridPos, LevelGrid, Tile};
use crate::player::{Player, Respawned};

/// Charges of the Steelman per level.
pub const STEELMAN_CHARGES: u32 = 1;
/// How far from the player's centre a platform can be reinforced, in tiles.
const REACH: f32 = 3.0;

/// Steelman charges left in this level.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Charges(pub u32);

/// Sent when a platform was reinforced, for sound.
#[derive(Message)]
pub struct Reinforced;

/// Sent when E found nothing to reinforce (or no charge was left).
#[derive(Message)]
pub struct ReinforceFailed;

#[derive(Component)]
struct ChargesText;

pub struct GadgetsPlugin;

impl Plugin for GadgetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Charges>()
            .add_message::<Reinforced>()
            .add_message::<ReinforceFailed>()
            .add_systems(OnEnter(AppState::Playing), refill)
            .add_systems(
                Update,
                (restore_on_respawn, use_steelman, spawn_widget, show_charges)
                    .chain()
                    .after(crate::player::respawn_if_fallen)
                    .run_if(in_state(AppState::Playing))
                    .run_if(unlocked(Feature::Steelman)),
            );
    }
}

/// Each level starts with a full charge once the gadget is unlocked, and
/// none before.
fn refill(current: Current, mut charges: ResMut<Charges>) {
    let unlocked = current
        .campaign
        .is_unlocked(Feature::Steelman, current.progress.level);
    charges.0 = if unlocked { STEELMAN_CHARGES } else { 0 };
}

/// The cells of the other lens's platform nearest to `position`, within
/// reach: the whole horizontal run, so a platform is reinforced at once.
pub fn steelman_target(grid: &LevelGrid, position: Vec2, lens: Lens) -> Vec<(usize, usize)> {
    let wanted = Cell::LensOnly(lens.other());
    let reach = REACH * crate::TILE;
    let nearest = (0..grid.height)
        .flat_map(|row| (0..grid.width).map(move |col| (col, row)))
        .filter(|&(col, row)| grid.cells[row][col] == wanted)
        .map(|(col, row)| (col, row, grid.cell_center(col, row).distance(position)))
        .filter(|&(_, _, distance)| distance <= reach)
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((col, row, _)) = nearest else {
        return Vec::new();
    };
    let cells = &grid.cells[row];
    let start = (0..=col)
        .rev()
        .take_while(|&c| cells[c] == wanted)
        .last()
        .unwrap_or(col);
    let end = (col..grid.width)
        .take_while(|&c| cells[c] == wanted)
        .last()
        .unwrap_or(col);
    (start..=end).map(|c| (c, row)).collect()
}

/// What using a gadget reports, for sound and feedback.
#[derive(bevy::ecs::system::SystemParam)]
struct Outcome<'w> {
    done: MessageWriter<'w, Reinforced>,
    failed: MessageWriter<'w, ReinforceFailed>,
}

fn use_steelman(
    keys: Res<ButtonInput<KeyCode>>,
    mut lens: ResMut<ActiveLens>,
    player: Query<&Transform, With<Player>>,
    mut grid: ResMut<LevelGrid>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut charges: ResMut<Charges>,
    mut outcome: Outcome,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok(transform) = player.single() else {
        return;
    };
    let target = steelman_target(&grid, transform.translation.truncate(), lens.0);
    if charges.0 == 0 || target.is_empty() {
        outcome.failed.write(ReinforceFailed);
        return;
    }
    charges.0 -= 1;
    for &(col, row) in &target {
        grid.cells[row][col] = Cell::Steel;
    }
    for (pos, mut tile) in &mut tiles {
        if target.contains(&(pos.0, pos.1)) {
            tile.0 = Cell::Steel;
        }
    }
    // Repaint: tiles are recoloured when the lens resource changes.
    lens.set_changed();
    outcome.done.write(Reinforced);
}

/// A respawn resets the level: steel turns back into the other lens's
/// platform (the straw plugin restores the grid itself), and the charge
/// returns.
fn restore_on_respawn(
    mut respawned: MessageReader<Respawned>,
    grid: Res<LevelGrid>,
    mut tiles: Query<(&GridPos, &mut Tile)>,
    mut charges: ResMut<Charges>,
    mut lens: ResMut<ActiveLens>,
) {
    if respawned.read().count() == 0 {
        return;
    }
    charges.0 = STEELMAN_CHARGES;
    let mut changed = false;
    for (pos, mut tile) in &mut tiles {
        if tile.0 == Cell::Steel {
            tile.0 = grid.original[pos.1][pos.0];
            changed = true;
        }
    }
    if changed {
        lens.set_changed(); // repaint
    }
}

fn spawn_widget(mut commands: Commands, slots: Query<(Entity, &HudSlot), Added<HudSlot>>) {
    let Some(slot) = new_slot(&slots, Slot::Gadgets) else {
        return;
    };
    commands.entity(slot).with_children(|parent| {
        parent.spawn((
            Text::new(""),
            crate::hud::text_size(16.0),
            TextColor(Color::srgb(0.55, 0.70, 0.95)),
            ChargesText,
        ));
    });
}

fn show_charges(
    charges: Res<Charges>,
    language: Res<Language>,
    mut text: Query<(&mut Text, &mut TextColor), With<ChargesText>>,
) {
    for (mut text, mut color) in &mut text {
        text.0 = format!("[E] {} × {}", ui(*language, Ui::Steelman), charges.0);
        color.0 = if charges.0 > 0 {
            Color::srgb(0.55, 0.70, 0.95)
        } else {
            Color::srgba(0.55, 0.70, 0.95, 0.35)
        };
    }
}
