//! Heads-up display while playing: level title, active lens, language and a
//! hint, plus empty slots that feature plugins fill with their own widgets
//! (lives, timer...). The HUD owns the layout; a widget only fills its slot.
//! The whole HUD is rebuilt whenever the language changes, and the slots are
//! spawned again with it, so a widget fills its slot from an `Added<HudSlot>`
//! query and works the same on the first frame and after a rebuild.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::Current;
use crate::i18n::{Fonts, Language, Ui, ui};
use crate::lens::{ActiveLens, ShiftBlocked};

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct LensLabel;

/// A place in the HUD where one feature shows its widget. Slots are laid out
/// top-right, in this order; an empty slot takes no space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Traveler,
    Lives,
    Timer,
    Gadgets,
}

impl Slot {
    const ALL: [Slot; 4] = [Slot::Traveler, Slot::Lives, Slot::Timer, Slot::Gadgets];
}

/// Marks the UI node of a slot. Widgets spawn as its children.
#[derive(Component, Debug)]
pub struct HudSlot(pub Slot);

/// The entity of a slot spawned this frame, for a widget plugin to fill.
/// Use it with `Query<(Entity, &HudSlot), Added<HudSlot>>`.
pub fn new_slot(slots: &Query<(Entity, &HudSlot), Added<HudSlot>>, wanted: Slot) -> Option<Entity> {
    slots
        .iter()
        .find_map(|(entity, slot)| (slot.0 == wanted).then_some(entity))
}

/// Font size helper shared by the HUD and its widgets.
pub fn text_size(px: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(px),
        ..default()
    }
}

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

fn spawn_hud(mut commands: Commands, current: Current, language: Res<Language>, fonts: Res<Fonts>) {
    let level = &current.level().data;
    let lang = *language;
    let size = text_size;
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
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Px(24.0),
                    ..default()
                },
                children![
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        children![
                            (
                                Text::new(current.world().name.get(lang)),
                                TextFont {
                                    font: fonts.bold.clone().into(),
                                    ..size(15.0)
                                },
                                TextColor(Color::srgb(0.65, 0.70, 0.70)),
                            ),
                            (
                                Text::new(format!(
                                    "{} · {}",
                                    current.number(),
                                    current.text(&level.title, lang)
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
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            row_gap: Val::Px(6.0),
                            ..default()
                        },
                        Children::spawn(SpawnIter(Slot::ALL.into_iter().map(|slot| {
                            (
                                Node {
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(6.0),
                                    ..default()
                                },
                                HudSlot(slot),
                            )
                        }))),
                    )
                ],
            ),
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(24.0),
                    ..default()
                },
                children![
                    (
                        Text::new(current.text(&level.hint, lang)),
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
    current: Current,
    fonts: Res<Fonts>,
) {
    if !language.is_changed() || language.is_added() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_hud(commands, current, language, fonts);
}

fn update_lens_label(
    time: Res<Time>,
    current: Current,
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
    let level = &current.level().data;
    let lang = *language;
    let active = level.lens_name(lens.0, lang);
    let other = level.lens_name(lens.0.other(), lang);
    if warning.0 > 0.0 {
        text.0 = format!("{} \"{other}\".", ui(lang, Ui::NoRoom));
        color.0 = Color::srgb(0.95, 0.40, 0.35);
    } else {
        text.0 = format!("{}: {active}   [SHIFT] {other}", ui(lang, Ui::Lens));
        color.0 = lens.0.color();
    }
}
