//! Lives, unlocked by a level in `levels/worlds.ron` (`Feature::Lives`).
//!
//! Each world has a budget of lives that carries over between its levels and
//! is refilled when a world starts. Every respawn costs one, whether the
//! player fell or pressed R, so restarting is never a free way out of a bad
//! jump. Running out opens a short screen, then applies the world's rule:
//! start the level again, or go back to the world's first level.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Campaign, Current, Feature, OutOfLives, Progress, unlocked};
use crate::hud::{HudSlot, Slot, new_slot};
use crate::i18n::{Fonts, Language, Ui, ui};
use crate::player::Respawned;

/// Lives left in the current world. `None` until the feature is first
/// unlocked, or after a world change, so the next level refills it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Lives {
    pub left: Option<u32>,
    /// The world the budget belongs to.
    world: usize,
}

/// One pip of the lives widget; `.0` is its index from the left.
#[derive(Component)]
struct LifePip(u32);

#[derive(Component)]
struct GameOverRoot;

const FULL: Color = Color::srgb(0.95, 0.40, 0.35);
const EMPTY: Color = Color::srgba(0.95, 0.40, 0.35, 0.18);

pub struct LivesPlugin;

impl Plugin for LivesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lives>()
            .add_systems(
                OnEnter(AppState::Playing),
                refill_on_new_world.run_if(unlocked(Feature::Lives)),
            )
            .add_systems(
                Update,
                (lose_life_on_respawn, spawn_widget, paint_widget)
                    .chain()
                    // Same frame as the respawn, so the HUD never lags behind.
                    .after(crate::player::respawn_if_fallen)
                    .run_if(in_state(AppState::Playing))
                    .run_if(unlocked(Feature::Lives)),
            )
            .add_systems(OnEnter(AppState::GameOver), spawn_game_over)
            .add_systems(
                Update,
                continue_after_game_over.run_if(in_state(AppState::GameOver)),
            );
    }
}

fn refill_on_new_world(current: Current, mut lives: ResMut<Lives>) {
    let world = current.campaign.levels[current.progress.level].world;
    if lives.left.is_none() || lives.world != world {
        *lives = Lives {
            left: Some(current.world().lives),
            world,
        };
    }
}

fn lose_life_on_respawn(
    mut respawned: MessageReader<Respawned>,
    mut lives: ResMut<Lives>,
    mut next: ResMut<NextState<AppState>>,
) {
    for _ in respawned.read() {
        let Some(left) = lives.left.as_mut() else {
            return;
        };
        *left = left.saturating_sub(1);
        if *left == 0 {
            next.set(AppState::GameOver);
            return;
        }
    }
}

fn spawn_widget(
    mut commands: Commands,
    slots: Query<(Entity, &HudSlot), Added<HudSlot>>,
    current: Current,
) {
    let Some(slot) = new_slot(&slots, Slot::Lives) else {
        return;
    };
    commands.entity(slot).with_children(|parent| {
        for index in 0..current.world().lives {
            parent.spawn((
                LifePip(index),
                Node {
                    width: Val::Px(14.0),
                    height: Val::Px(14.0),
                    border_radius: BorderRadius::all(Val::Px(7.0)),
                    ..default()
                },
                BackgroundColor(FULL),
            ));
        }
    });
}

fn paint_widget(lives: Res<Lives>, mut pips: Query<(&LifePip, &mut BackgroundColor)>) {
    let left = lives.left.unwrap_or(0);
    for (pip, mut color) in &mut pips {
        color.0 = if pip.0 < left { FULL } else { EMPTY };
    }
}

fn spawn_game_over(
    mut commands: Commands,
    current: Current,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let lang = *language;
    let rule = match current.world().out_of_lives {
        OutOfLives::RestartLevel => ui(lang, Ui::BackToLevel),
        OutOfLives::RestartWorld => ui(lang, Ui::BackToWorld),
    };
    commands.spawn((
        GameOverRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.94)),
        DespawnOnExit(AppState::GameOver),
        children![
            (
                Text::new(ui(lang, Ui::OutOfLives)),
                TextFont {
                    font: fonts.bold.clone().into(),
                    font_size: FontSize::Px(40.0),
                    ..default()
                },
                TextColor(FULL),
            ),
            (
                Text::new(rule),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
            ),
            (
                Text::new(format!("ENTER: {}", ui(lang, Ui::TryAgain))),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.74, 0.70)),
            ),
        ],
    ));
}

fn continue_after_game_over(
    keys: Res<ButtonInput<KeyCode>>,
    campaign: Res<Campaign>,
    mut progress: ResMut<Progress>,
    mut lives: ResMut<Lives>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        return;
    }
    apply_game_over(&campaign, &mut progress, &mut lives);
    next.set(progress.level_entry(&campaign));
}

/// Applies the world's out-of-lives rule and refills the budget.
pub fn apply_game_over(campaign: &Campaign, progress: &mut Progress, lives: &mut Lives) {
    let world = campaign.world_of(progress.level);
    if world.out_of_lives == OutOfLives::RestartWorld {
        progress.level = world.first_level;
    }
    lives.left = Some(world.lives);
}
