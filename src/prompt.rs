//! The question screen: before a level that asks something (`ask:` in
//! `levels/worlds.ron`), the player types an answer that goes into the
//! profile, is saved, and fills `{name}` / `{city}` in later texts.
//!
//! Once the name is known, a small widget in the HUD's traveler slot shows
//! it (and the city, later on).
//!
//! Typing reads `KeyboardInput` messages, which carry the character the
//! keyboard layout produced, so accents and ñ work, on the web too. While
//! this screen is open the letter keys only type (L does not switch the
//! language).

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Campaign, ProfileField, Progress};
use crate::hud::{HudSlot, Slot, new_slot};
use crate::i18n::{Fonts, Language, Ui, ui};

/// Longest answer, in characters. Long enough for "Ciudad de México".
pub const MAX_CHARS: usize = 24;

/// The answer being typed.
#[derive(Resource, Debug, Default)]
pub struct Answer(pub String);

#[derive(Component)]
struct AnswerText;

pub struct PromptPlugin;

impl Plugin for PromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Answer>()
            .add_systems(OnEnter(AppState::Prompt), (clear_answer, spawn_prompt))
            .add_systems(
                Update,
                (type_answer, show_answer)
                    .chain()
                    .run_if(in_state(AppState::Prompt)),
            )
            .add_systems(Update, spawn_traveler_widget);
    }
}

fn clear_answer(mut answer: ResMut<Answer>, mut typed: MessageReader<KeyboardInput>) {
    answer.0.clear();
    // Drop the key that opened this screen (ENTER on the card), so it
    // cannot confirm or type anything here.
    typed.clear();
}

fn spawn_prompt(
    mut commands: Commands,
    campaign: Res<Campaign>,
    progress: Res<Progress>,
    language: Res<Language>,
    fonts: Res<Fonts>,
) {
    let lang = *language;
    let (question, why) = match progress.pending_question(&campaign) {
        Some(ProfileField::City) => (Ui::AskCity, Ui::AskCityWhy),
        _ => (Ui::AskName, Ui::AskNameWhy),
    };
    let size = crate::hud::text_size;
    commands.spawn((
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
        DespawnOnExit(AppState::Prompt),
        children![
            (
                Text::new(ui(lang, question)),
                TextFont {
                    font: fonts.bold.clone().into(),
                    ..size(34.0)
                },
            ),
            (
                Text::new(ui(lang, why)),
                size(18.0),
                TextColor(Color::srgb(0.72, 0.74, 0.70)),
            ),
            (
                Node {
                    min_width: Val::Px(360.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                    border: UiRect::bottom(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BorderColor::all(Color::srgb(0.25, 0.70, 0.75)),
                children![(Text::new("_"), size(30.0), AnswerText)],
            ),
            (
                Text::new(format!("ENTER: {}", ui(lang, Ui::Confirm))),
                size(15.0),
                TextColor(Color::srgb(0.55, 0.57, 0.55)),
            ),
        ],
    ));
}

/// Applies one key to the answer. Returns true when ENTER confirms a
/// non-empty answer. ENTER and BACKSPACE are recognised by their physical
/// code as well as their logical key, since some input sources (virtual
/// keyboards, remote control tools) only fill one of them.
pub fn apply_key(answer: &mut String, code: KeyCode, key: &Key) -> bool {
    let enter = matches!(key, Key::Enter) || matches!(code, KeyCode::Enter | KeyCode::NumpadEnter);
    let backspace = matches!(key, Key::Backspace) || code == KeyCode::Backspace;
    if enter {
        return !answer.trim().is_empty();
    }
    if backspace {
        answer.pop();
        return false;
    }
    match key {
        Key::Space => push(answer, " "),
        Key::Character(text) => push(answer, text),
        _ => {}
    }
    false
}

fn push(answer: &mut String, text: &str) {
    for c in text.chars().filter(|c| !c.is_control()) {
        // No leading spaces: an answer starts with a visible character.
        if c.is_whitespace() && answer.is_empty() {
            continue;
        }
        if answer.chars().count() < MAX_CHARS {
            answer.push(c);
        }
    }
}

fn type_answer(
    mut typed: MessageReader<KeyboardInput>,
    mut answer: ResMut<Answer>,
    campaign: Res<Campaign>,
    mut progress: ResMut<Progress>,
    mut next: ResMut<NextState<AppState>>,
) {
    for input in typed.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        if apply_key(&mut answer.0, input.key_code, &input.logical_key) {
            let Some(field) = progress.pending_question(&campaign) else {
                return;
            };
            let value = answer.0.trim().to_string();
            progress.profile.set(field, value);
            next.set(progress.level_entry(&campaign));
            return;
        }
    }
}

fn show_answer(answer: Res<Answer>, mut text: Query<&mut Text, With<AnswerText>>) {
    if !answer.is_changed() {
        return;
    }
    for mut text in &mut text {
        text.0 = format!("{}_", answer.0);
    }
}

/// Fills the HUD's traveler slot with "name · city", once there is a name.
fn spawn_traveler_widget(
    mut commands: Commands,
    slots: Query<(Entity, &HudSlot), Added<HudSlot>>,
    progress: Res<Progress>,
) {
    let Some(slot) = new_slot(&slots, Slot::Traveler) else {
        return;
    };
    let profile = &progress.profile;
    let Some(name) = profile.name.as_deref() else {
        return;
    };
    let label = match profile.city.as_deref() {
        Some(city) => format!("{name} · {city}"),
        None => name.to_string(),
    };
    commands.entity(slot).with_children(|parent| {
        parent.spawn((
            Text::new(label),
            crate::hud::text_size(16.0),
            TextColor(Color::srgb(0.85, 0.85, 0.80)),
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(keys: &[Key]) -> (String, bool) {
        let mut answer = String::new();
        let mut confirmed = false;
        for key in keys {
            confirmed = apply_key(&mut answer, KeyCode::KeyA, key);
        }
        (answer, confirmed)
    }

    #[test]
    fn enter_and_backspace_also_work_by_physical_code() {
        let mut answer = String::from("Ab");
        let newline = Key::Character("\n".into());
        assert!(!apply_key(&mut answer, KeyCode::Backspace, &newline));
        assert_eq!(answer, "A");
        assert!(apply_key(&mut answer, KeyCode::Enter, &newline));
        assert_eq!(answer, "A", "the control character is not typed");
    }

    fn ch(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn types_accents_spaces_and_deletes() {
        let keys = [
            ch("J"),
            ch("o"),
            ch("s"),
            ch("é"),
            Key::Space,
            ch("x"),
            Key::Backspace,
        ];
        assert_eq!(typed(&keys), ("José ".to_string(), false));
    }

    #[test]
    fn enter_confirms_only_a_non_empty_answer() {
        assert!(
            !typed(&[Key::Space, Key::Enter]).1,
            "a leading space is ignored"
        );
        assert!(typed(&[ch("A"), Key::Enter]).1);
    }

    #[test]
    fn answers_are_capped() {
        let keys = vec![ch("a"); MAX_CHARS + 5];
        assert_eq!(typed(&keys).0.chars().count(), MAX_CHARS);
    }
}
