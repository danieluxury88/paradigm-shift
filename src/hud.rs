//! Heads-up display while playing: level title, active lens, language and a
//! hint. Rebuilt whenever the language changes.

use bevy::prelude::*;

use crate::AppState;
use crate::i18n::{Fonts, Language, Ui, ui};
use crate::lens::{ActiveLens, ShiftBlocked};
use crate::level::Levels;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct LensLabel;

/// Seconds left showing the "no room" warning after a refused shift.
#[derive(Resource, Default)]
struct BlockedWarning(f32);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BlockedWarning>()
            .add_systems(OnEnter(AppState::Playing), spawn_hud)
            .add_systems(
                Update,
                (rebuild_on_language_change, update_lens_label)
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

fn spawn_hud(
    mut commands: Commands,
    levels: Res<Levels>,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let level = levels.current();
    let lang = *language;
    let size = |px: f32| TextFont {
        font_size: FontSize::Px(px),
        ..default()
    };
    commands.spawn((
        HudRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        },
        DespawnOnExit(AppState::Playing),
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                children![
                    (
                        Text::new(format!(
                            "{} · {}",
                            levels.current + 1,
                            level.title.get(lang)
                        )),
                        TextFont {
                            font: fonts.bold.clone().into(),
                            ..size(28.0)
                        },
                    ),
                    (Text::new(""), size(19.0), LensLabel),
                ]
            ),
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(24.0),
                    ..default()
                },
                children![
                    (
                        Text::new(level.hint.get(lang)),
                        size(17.0),
                        TextColor(Color::srgb(0.75, 0.75, 0.72))
                    ),
                    (
                        Text::new(format!(
                            "[L] {} · {}",
                            ui(lang, Ui::LanguageKey),
                            lang.code()
                        )),
                        size(15.0),
                        TextColor(Color::srgb(0.55, 0.57, 0.55)),
                    ),
                ]
            ),
        ],
    ));
}

/// Changing the language rebuilds the HUD rather than patching each text.
fn rebuild_on_language_change(
    mut commands: Commands,
    language: Res<Language>,
    roots: Query<Entity, With<HudRoot>>,
    levels: Res<Levels>,
    fonts: Res<Fonts>,
) {
    if !language.is_changed() || language.is_added() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_hud(commands, levels, language, fonts);
}

fn update_lens_label(
    time: Res<Time>,
    levels: Res<Levels>,
    lens: Res<ActiveLens>,
    language: Res<Language>,
    mut blocked: MessageReader<ShiftBlocked>,
    mut warning: ResMut<BlockedWarning>,
    mut label: Query<(&mut Text, &mut TextColor), With<LensLabel>>,
) {
    if blocked.read().count() > 0 {
        warning.0 = 1.2;
    }
    warning.0 = (warning.0 - time.delta_secs()).max(0.0);

    // Right after a rebuild the new label exists only from the next frame on.
    let Ok((mut text, mut color)) = label.single_mut() else {
        return;
    };
    let level = levels.current();
    let lang = *language;
    let current = level.lens_name(lens.0, lang);
    let other = level.lens_name(lens.0.other(), lang);
    if warning.0 > 0.0 {
        text.0 = format!("{} \"{other}\".", ui(lang, Ui::NoRoom));
        color.0 = Color::srgb(0.95, 0.40, 0.35);
    } else {
        text.0 = format!("{}: {current}   [SHIFT] {other}", ui(lang, Ui::Lens));
        color.0 = lens.0.color();
    }
}
