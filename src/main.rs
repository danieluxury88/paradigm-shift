//! Paradigm Shift: a puzzle platformer about seeing the same world through
//! different paradigms. Entry point: builds the Bevy app and wires the plugins.

mod audio;
#[cfg(feature = "brp")]
mod brp;
mod campaign;
mod card;
mod gadgets;
mod hud;
mod i18n;
mod journal;
mod lens;
mod level;
mod lives;
mod player;
mod prompt;
mod save;
#[cfg(test)]
mod sim_tests;
#[cfg(test)]
mod solver;
mod straw;
mod synth;
mod timer;
mod title;

use bevy::prelude::*;

/// Size of one map cell in world units (pixels at 1:1 zoom).
pub const TILE: f32 = 32.0;

/// The screens of the game: the title, then the loop of playing a level and
/// reading the concept card that names what the level taught. `GameOver`
/// opens when the player runs out of lives, once lives are unlocked.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Title,
    Playing,
    Card,
    GameOver,
    /// A question before a level (the player's name, their city).
    Prompt,
    /// The concepts collected so far, opened from the card or the title.
    Journal,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Paradigm Shift".into(),
            // On the web, render into the <canvas id="bevy"> from index.html
            // and follow the browser window size.
            canvas: Some("#bevy".into()),
            fit_canvas_to_parent: true,
            ..default()
        }),
        ..default()
    }))
    .init_state::<AppState>()
    // The core loop: content, saving, screens, the level and its mechanics.
    .add_plugins((
        // First: it replaces the default font that the other plugins' texts use.
        i18n::I18nPlugin,
        campaign::CampaignPlugin,
        // After the campaign and the language: the save restores both.
        save::SavePlugin,
        title::TitlePlugin,
        level::LevelPlugin,
        lens::LensPlugin,
        player::PlayerPlugin,
        straw::StrawPlugin,
        hud::HudPlugin,
        card::CardPlugin,
        prompt::PromptPlugin,
    ))
    // Features the campaign unlocks level by level (`unlocks:` in
    // levels/worlds.ron); each fills its own HUD or card slot.
    .add_plugins((
        audio::AudioPlugin,
        journal::JournalPlugin,
        lives::LivesPlugin,
        timer::TimerPlugin,
        gadgets::GadgetsPlugin,
    ))
    .add_systems(Startup, spawn_camera);

    #[cfg(feature = "brp")]
    app.add_plugins(brp::BrpPlugin);

    // Development shortcut, left out of release builds: N solves the level.
    #[cfg(debug_assertions)]
    app.add_systems(Update, skip_level.run_if(in_state(AppState::Playing)));

    app.run();
}

#[cfg(debug_assertions)]
fn skip_level(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::KeyN) {
        next.set(AppState::Card);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
