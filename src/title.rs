//! The title screen: the game's name, and either "start" or "continue" with
//! the saved level. BACKSPACE starts a new game when there is a save. On the
//! web it also gives the browser the key press it requires before any sound
//! may play.

use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Current, Progress};
use crate::i18n::{Fonts, Language, Ui, ui};
use crate::save::SaveFound;

#[derive(Component)]
struct TitleRoot;

pub struct TitlePlugin;

impl Plugin for TitlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Title), spawn_title)
            .add_systems(
                Update,
                (rebuild_on_language_change, choose).run_if(in_state(AppState::Title)),
            );
    }
}

/// Whether there is progress worth continuing from.
fn can_continue(found: SaveFound, progress: &Progress) -> bool {
    found.0 && *progress != Progress::default()
}

fn spawn_title(
    mut commands: Commands,
    current: Current,
    found: Res<SaveFound>,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let lang = *language;
    let soft = TextColor(Color::srgb(0.72, 0.74, 0.70));
    let size = crate::hud::text_size;
    let start = if can_continue(*found, &current.progress) {
        format!(
            "ENTER: {} · {} {} · {}\nBACKSPACE: {}",
            ui(lang, Ui::Continue),
            ui(lang, Ui::Level),
            current.number(),
            current.level().data.title.get(lang),
            ui(lang, Ui::NewGame),
        )
    } else {
        format!("ENTER: {}", ui(lang, Ui::Start))
    };
    commands.spawn((
        TitleRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(18.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.07, 0.08, 0.10)),
        DespawnOnExit(AppState::Title),
        children![
            (
                Text::new("Paradigm Shift"),
                TextFont {
                    font: fonts.bold.clone().into(),
                    ..size(64.0)
                },
            ),
            (
                Text::new(ui(lang, Ui::Tagline)),
                size(20.0),
                TextColor(Color::srgb(0.25, 0.70, 0.75)),
            ),
            (
                Text::new(start),
                size(18.0),
                TextLayout::justify(Justify::Center),
            ),
            (
                Text::new(format!(
                    "[L] {} · {}",
                    ui(lang, Ui::LanguageKey),
                    lang.code()
                )),
                size(15.0),
                soft,
            ),
        ],
    ));
}

fn rebuild_on_language_change(
    mut commands: Commands,
    language: Res<Language>,
    roots: Query<Entity, With<TitleRoot>>,
    current: Current,
    found: Res<SaveFound>,
    fonts: Res<Fonts>,
) {
    if !language.is_changed() || language.is_added() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    spawn_title(commands, current, found, language, fonts);
}

fn choose(
    keys: Res<ButtonInput<KeyCode>>,
    found: Res<SaveFound>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Backspace) && can_continue(*found, &progress) {
        *progress = Progress::default();
        next.set(AppState::Playing);
    } else if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]) {
        next.set(AppState::Playing);
    }
}
