//! The level clock, unlocked by a level in `levels/worlds.ron`
//! (`Feature::Timer`).
//!
//! It counts the time spent playing the current level. Falling does not stop
//! it, so mistakes cost time; entering the level again (a new level, or after
//! a game over) starts it from zero. When the goal is reached, the time is
//! compared with the best one in the save and shown on the concept card.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Campaign, Feature, Progress, unlocked};
use crate::card::CardSlot;
use crate::hud::{HudSlot, Slot, new_slot};
use crate::i18n::{Language, Ui, ui};
use crate::player::LevelCompleted;

/// Seconds spent in the current level.
#[derive(Resource, Debug, Default)]
pub struct LevelClock(pub f32);

/// The last completed run, for the concept card. Cleared when a level starts,
/// so a level left without reaching the goal shows no time.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct LastRun(pub Option<Run>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Run {
    pub time: f32,
    /// The best time before this run, if there was one.
    pub previous_best: Option<f32>,
}

impl Run {
    pub fn is_record(&self) -> bool {
        self.previous_best.is_none_or(|best| self.time < best)
    }
}

#[derive(Component)]
struct ClockText;

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelClock>()
            .init_resource::<LastRun>()
            .add_systems(OnEnter(AppState::Playing), start_clock)
            .add_systems(
                Update,
                (tick, record_run, spawn_widget, show_clock)
                    .chain()
                    .after(crate::player::reach_goal)
                    .run_if(in_state(AppState::Playing))
                    .run_if(unlocked(Feature::Timer)),
            )
            .add_systems(Update, show_run_on_card.run_if(in_state(AppState::Card)));
    }
}

fn start_clock(mut clock: ResMut<LevelClock>, mut last: ResMut<LastRun>) {
    clock.0 = 0.0;
    last.0 = None;
}

fn tick(time: Res<Time>, mut clock: ResMut<LevelClock>) {
    clock.0 += time.delta_secs();
}

fn record_run(
    mut completed: MessageReader<LevelCompleted>,
    clock: Res<LevelClock>,
    campaign: Res<Campaign>,
    mut progress: ResMut<Progress>,
    mut last: ResMut<LastRun>,
) {
    if completed.read().count() == 0 {
        return;
    }
    let id = campaign.levels[progress.level].id.clone();
    let run = Run {
        time: clock.0,
        previous_best: progress.best_times.get(&id).copied(),
    };
    if run.is_record() {
        progress.best_times.insert(id, run.time);
    }
    last.0 = Some(run);
}

/// `m:ss.t`, e.g. `1:04.3`.
pub fn format_time(seconds: f32) -> String {
    let tenths = (seconds * 10.0).floor() as u32;
    format!("{}:{:02}.{}", tenths / 600, (tenths / 10) % 60, tenths % 10)
}

fn spawn_widget(mut commands: Commands, slots: Query<(Entity, &HudSlot), Added<HudSlot>>) {
    let Some(slot) = new_slot(&slots, Slot::Timer) else {
        return;
    };
    commands.entity(slot).with_children(|parent| {
        parent.spawn((
            Text::new(format_time(0.0)),
            crate::hud::text_size(18.0),
            TextColor(Color::srgb(0.95, 0.85, 0.30)),
            ClockText,
        ));
    });
}

fn show_clock(clock: Res<LevelClock>, mut text: Query<&mut Text, With<ClockText>>) {
    for mut text in &mut text {
        text.0 = format_time(clock.0);
    }
}

fn show_run_on_card(
    mut commands: Commands,
    slots: Query<Entity, Added<CardSlot>>,
    last: Res<LastRun>,
    language: Res<Language>,
) {
    let (Ok(slot), Some(run)) = (slots.single(), last.0) else {
        return;
    };
    let lang = *language;
    let line = match run.previous_best {
        Some(_) if run.is_record() => format!(
            "{}: {} · {}",
            ui(lang, Ui::Time),
            format_time(run.time),
            ui(lang, Ui::NewRecord)
        ),
        Some(best) => format!(
            "{}: {} · {}: {}",
            ui(lang, Ui::Time),
            format_time(run.time),
            ui(lang, Ui::Best),
            format_time(best)
        ),
        None => format!("{}: {}", ui(lang, Ui::Time), format_time(run.time)),
    };
    commands.entity(slot).with_children(|parent| {
        parent.spawn((
            Text::new(line),
            crate::hud::text_size(17.0),
            TextColor(Color::srgb(0.95, 0.85, 0.30)),
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_minutes_seconds_and_tenths() {
        assert_eq!(format_time(0.0), "0:00.0");
        assert_eq!(format_time(64.37), "1:04.3");
        assert_eq!(format_time(600.0), "10:00.0");
    }

    #[test]
    fn a_first_run_or_a_faster_one_is_a_record() {
        let first = Run {
            time: 30.0,
            previous_best: None,
        };
        assert!(first.is_record());
        let slower = Run {
            time: 31.0,
            previous_best: Some(30.0),
        };
        assert!(!slower.is_record());
    }
}
