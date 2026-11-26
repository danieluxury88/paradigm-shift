//! The journal: every concept the player has collected, unlocked by a level
//! in `levels/worlds.ron` (`Feature::Journal`).
//!
//! J opens it from the concept card or the title screen and returns there;
//! it is not reachable mid-level, because leaving `Playing` rebuilds the
//! level. The left column lists the collected concepts by world; UP and DOWN
//! choose one, and the right side shows its whole card, with the player's
//! name and city filled in.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Campaign, Feature, Progress, fill};
use crate::card::CardSlot;
use crate::i18n::{Fonts, Language, Ui, ui};

/// The screen to go back to, and the entry being read.
#[derive(Resource, Debug)]
pub struct JournalView {
    pub back_to: AppState,
    pub selected: usize,
}

impl Default for JournalView {
    fn default() -> Self {
        JournalView {
            back_to: AppState::Title,
            selected: 0,
        }
    }
}

#[derive(Component)]
struct JournalRoot;

pub struct JournalPlugin;

impl Plugin for JournalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JournalView>()
            .add_systems(
                Update,
                (open_journal, show_hint_on_card)
                    .run_if(in_state(AppState::Card).or_else(in_state(AppState::Title))),
            )
            .add_systems(OnEnter(AppState::Journal), spawn_journal)
            .add_systems(
                Update,
                (navigate, rebuild)
                    .chain()
                    .run_if(in_state(AppState::Journal)),
            );
    }
}

/// Indices into `Campaign::levels` of the collected concepts, in play order.
pub fn entries(campaign: &Campaign, progress: &Progress) -> Vec<usize> {
    (0..campaign.levels.len())
        .filter(|&i| progress.completed.contains(&campaign.levels[i].id))
        .collect()
}

/// The journal is available once unlocked at the level reached and at least
/// one concept is collected.
fn available(campaign: &Campaign, progress: &Progress) -> bool {
    campaign.is_unlocked(Feature::Journal, progress.level) && !progress.completed.is_empty()
}

fn open_journal(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    mut view: ResMut<JournalView>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::KeyJ) && available(&campaign, &progress) {
        // Open on the newest concept: the one just collected on the card.
        *view = JournalView {
            back_to: *state.get(),
            selected: entries(&campaign, &progress).len().saturating_sub(1),
        };
        next.set(AppState::Journal);
    }
}

/// Adds "J: journal" to the concept card once the journal is available.
fn show_hint_on_card(
    mut commands: Commands,
    slots: Query<Entity, Added<CardSlot>>,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    language: Res<Language>,
) {
    let Ok(slot) = slots.single() else {
        return;
    };
    if !available(&campaign, &progress) {
        return;
    }
    let count = entries(&campaign, &progress).len();
    commands.entity(slot).with_children(|parent| {
        parent.spawn((
            Text::new(format!(
                "J: {} · {count} {}",
                ui(*language, Ui::Journal),
                ui(*language, Ui::Concepts)
            )),
            crate::hud::text_size(15.0),
            TextColor(Color::srgb(0.95, 0.85, 0.30)),
        ));
    });
}

fn spawn_journal(
    mut commands: Commands,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    view: Res<JournalView>,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let lang = *language;
    let list = entries(&campaign, &progress);
    let size = crate::hud::text_size;
    let soft = Color::srgb(0.72, 0.74, 0.70);
    let accent = Color::srgb(0.95, 0.85, 0.30);
    let text = |tr: &crate::i18n::Tr| fill(tr.get(lang), &progress.profile, lang);

    commands
        .spawn((
            JournalRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(32.0)),
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
            DespawnOnExit(AppState::Journal),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "{} · {} {}",
                    ui(lang, Ui::Journal).to_uppercase(),
                    list.len(),
                    ui(lang, Ui::Concepts)
                )),
                TextFont {
                    font: fonts.bold.clone().into(),
                    ..size(16.0)
                },
                TextColor(accent),
            ));
            root.spawn(Node {
                column_gap: Val::Px(40.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|columns| {
                // Left: the concepts, grouped under their world's name.
                columns
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        min_width: Val::Px(260.0),
                        ..default()
                    })
                    .with_children(|left| {
                        let mut last_world = None;
                        for (position, &level) in list.iter().enumerate() {
                            let entry = &campaign.levels[level];
                            if last_world != Some(entry.world) {
                                last_world = Some(entry.world);
                                left.spawn((
                                    Text::new(campaign.worlds[entry.world].name.get(lang)),
                                    TextFont {
                                        font: fonts.bold.clone().into(),
                                        ..size(14.0)
                                    },
                                    TextColor(soft),
                                    Node {
                                        margin: UiRect::top(Val::Px(10.0)),
                                        ..default()
                                    },
                                ));
                            }
                            let chosen = position == view.selected;
                            left.spawn((
                                Text::new(format!(
                                    "{} {}",
                                    if chosen { ">" } else { " " },
                                    text(&entry.data.card.term)
                                )),
                                size(18.0),
                                TextColor(if chosen { accent } else { Color::WHITE }),
                            ));
                        }
                    });
                // Right: the chosen concept's card.
                if let Some(&level) = list.get(view.selected) {
                    let card = &campaign.levels[level].data.card;
                    columns
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(12.0),
                            max_width: Val::Px(680.0),
                            ..default()
                        })
                        .with_children(|right| {
                            right.spawn((
                                Text::new(text(&card.term)),
                                TextFont {
                                    font: fonts.bold.clone().into(),
                                    ..size(36.0)
                                },
                            ));
                            right.spawn((
                                Text::new(text(&card.origin)),
                                size(15.0),
                                TextColor(soft),
                            ));
                            right.spawn((Text::new(text(&card.definition)), size(19.0)));
                            right.spawn((
                                Text::new(format!(
                                    "{}: {}",
                                    ui(lang, Ui::Example),
                                    text(&card.example)
                                )),
                                size(16.0),
                                TextColor(soft),
                            ));
                            right.spawn((
                                Text::new(format!(
                                    "{}: {}",
                                    ui(lang, Ui::TryIt),
                                    text(&card.practice)
                                )),
                                size(16.0),
                                TextColor(Color::srgb(0.45, 0.80, 0.70)),
                            ));
                        });
                }
            });
            root.spawn((
                Text::new(format!(
                    "UP / DOWN   ·   J / ESC: {}   ·   [L] {} · {}",
                    ui(lang, Ui::Back),
                    ui(lang, Ui::LanguageKey),
                    lang.code()
                )),
                size(15.0),
                TextColor(soft),
            ));
        });
}

fn navigate(
    keys: Res<ButtonInput<KeyCode>>,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    mut view: ResMut<JournalView>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.any_just_pressed([KeyCode::KeyJ, KeyCode::Escape]) {
        next.set(view.back_to);
        return;
    }
    let count = entries(&campaign, &progress).len();
    if count == 0 {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        view.selected = (view.selected + 1) % count;
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        view.selected = (view.selected + count - 1) % count;
    }
}

/// Redraws the journal when the selection or the language changes.
fn rebuild(
    mut commands: Commands,
    roots: Query<Entity, With<JournalRoot>>,
    view: Res<JournalView>,
    language: Res<Language>,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    fonts: Res<Fonts>,
) {
    let changed = (view.is_changed() || language.is_changed()) && !view.is_added();
    if !changed || roots.is_empty() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_journal(commands, campaign, progress, view, language, fonts);
}
