//! The concept card shown after a level: it names the idea the level made the
//! player experience. ENTER continues to the next level; L changes language.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Campaign, Current, Progress};
use crate::i18n::{Fonts, Language, Ui, ui};

#[derive(Component)]
struct CardRoot;

pub struct CardPlugin;

impl Plugin for CardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Card), spawn_card)
            .add_systems(
                Update,
                (rebuild_on_language_change, continue_on_enter).run_if(in_state(AppState::Card)),
            );
    }
}

fn spawn_card(
    mut commands: Commands,
    current: Current,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let lang = *language;
    let level = &current.level().data;
    let card = &level.card;
    let size = |px: f32| TextFont {
        font_size: FontSize::Px(px),
        ..default()
    };
    let soft = TextColor(Color::srgb(0.72, 0.74, 0.70));
    let next = ui(
        lang,
        if current.progress.is_last(&current.campaign) {
            Ui::PlayAgain
        } else {
            Ui::NextLevel
        },
    );

    commands.spawn((
        CardRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.94)),
        DespawnOnExit(AppState::Card),
        children![(
            Node {
                max_width: Val::Px(760.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                padding: UiRect::all(Val::Px(28.0)),
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
                    Text::new(ui(lang, Ui::ConceptUnlocked)),
                    TextFont {
                        font: fonts.bold.clone().into(),
                        ..size(15.0)
                    },
                    TextColor(Color::srgb(0.95, 0.85, 0.30)),
                ),
                (
                    Text::new(card.term.get(lang)),
                    TextFont {
                        font: fonts.bold.clone().into(),
                        ..size(42.0)
                    },
                ),
                (Text::new(card.origin.get(lang)), size(16.0), soft),
                (Text::new(card.definition.get(lang)), size(21.0)),
                (
                    Text::new(format!(
                        "{}: {}",
                        ui(lang, Ui::Example),
                        card.example.get(lang)
                    )),
                    size(17.0),
                    soft
                ),
                (
                    Text::new(format!(
                        "{}: {}",
                        ui(lang, Ui::TryIt),
                        card.practice.get(lang)
                    )),
                    size(17.0),
                    TextColor(Color::srgb(0.45, 0.80, 0.70)),
                ),
                (
                    Text::new(format!(
                        "ENTER: {next}   ·   [L] {} · {}",
                        ui(lang, Ui::LanguageKey),
                        lang.code()
                    )),
                    size(15.0),
                    soft,
                ),
            ]
        )],
    ));
}

fn rebuild_on_language_change(
    mut commands: Commands,
    language: Res<Language>,
    roots: Query<Entity, With<CardRoot>>,
    current: Current,
    fonts: Res<Fonts>,
) {
    if !language.is_changed() || language.is_added() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_card(commands, current, language, fonts);
}

fn continue_on_enter(
    keys: Res<ButtonInput<KeyCode>>,
    campaign: Res<Campaign>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
        progress.advance(&campaign);
        next.set(AppState::Playing);
    }
}
