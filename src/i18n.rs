//! Languages: English, German and Spanish. Level texts carry all three in
//! their `.ron` files; the few interface strings live in `ui()` below.
//! L cycles the language at any time. Also installs the game font, which
//! (unlike Bevy's built-in one) covers accents, umlauts and ß.

use bevy::prelude::*;
use bevy::text::Font;
use serde::Deserialize;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/AtkinsonHyperlegible-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/AtkinsonHyperlegible-Bold.ttf");

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    De,
    Es,
}

impl Language {
    #[cfg(test)]
    pub const ALL: [Language; 3] = [Language::En, Language::De, Language::Es];

    fn next(self) -> Language {
        match self {
            Language::En => Language::De,
            Language::De => Language::Es,
            Language::Es => Language::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::En => "EN",
            Language::De => "DE",
            Language::Es => "ES",
        }
    }
}

/// A text in every supported language. English is required; a missing German
/// or Spanish text falls back to English (and fails `level::tests`).
#[derive(Debug, Clone, Deserialize)]
pub struct Tr {
    pub en: String,
    #[serde(default)]
    pub de: String,
    #[serde(default)]
    pub es: String,
}

impl Tr {
    pub fn get(&self, language: Language) -> &str {
        let text = match language {
            Language::En => &self.en,
            Language::De => &self.de,
            Language::Es => &self.es,
        };
        if text.is_empty() { &self.en } else { text }
    }

    #[cfg(test)]
    pub fn is_complete(&self) -> bool {
        Language::ALL.iter().all(|&l| match l {
            Language::En => !self.en.is_empty(),
            Language::De => !self.de.is_empty(),
            Language::Es => !self.es.is_empty(),
        })
    }
}

/// Interface strings that are not part of any level.
#[derive(Debug, Clone, Copy)]
pub enum Ui {
    Lens,
    NoRoom,
    ConceptUnlocked,
    Example,
    TryIt,
    NextLevel,
    PlayAgain,
    LanguageKey,
    OutOfLives,
    BackToLevel,
    BackToWorld,
    TryAgain,
}

pub fn ui(language: Language, key: Ui) -> &'static str {
    use Language::*;
    match (key, language) {
        (Ui::Lens, En) => "Lens",
        (Ui::Lens, De) => "Linse",
        (Ui::Lens, Es) => "Lente",
        (Ui::NoRoom, En) => "No room here in",
        (Ui::NoRoom, De) => "Hier kein Platz in",
        (Ui::NoRoom, Es) => "No hay sitio aquí en",
        (Ui::ConceptUnlocked, En) => "CONCEPT UNLOCKED",
        (Ui::ConceptUnlocked, De) => "KONZEPT FREIGESCHALTET",
        (Ui::ConceptUnlocked, Es) => "CONCEPTO DESBLOQUEADO",
        (Ui::Example, En) => "Example",
        (Ui::Example, De) => "Beispiel",
        (Ui::Example, Es) => "Ejemplo",
        (Ui::TryIt, En) => "Try it",
        (Ui::TryIt, De) => "Probier es aus",
        (Ui::TryIt, Es) => "Pruébalo",
        (Ui::NextLevel, En) => "next level",
        (Ui::NextLevel, De) => "nächstes Level",
        (Ui::NextLevel, Es) => "siguiente nivel",
        (Ui::PlayAgain, En) => "play again",
        (Ui::PlayAgain, De) => "nochmal spielen",
        (Ui::PlayAgain, Es) => "jugar otra vez",
        (Ui::LanguageKey, En) => "language",
        (Ui::LanguageKey, De) => "Sprache",
        (Ui::LanguageKey, Es) => "idioma",
        (Ui::OutOfLives, En) => "Out of lives",
        (Ui::OutOfLives, De) => "Keine Leben mehr",
        (Ui::OutOfLives, Es) => "Sin vidas",
        (Ui::BackToLevel, En) => "This level starts over.",
        (Ui::BackToLevel, De) => "Dieses Level beginnt von vorn.",
        (Ui::BackToLevel, Es) => "Este nivel empieza de nuevo.",
        (Ui::BackToWorld, En) => "Back to the first level of this world.",
        (Ui::BackToWorld, De) => "Zurück zum ersten Level dieser Welt.",
        (Ui::BackToWorld, Es) => "Vuelves al primer nivel de este mundo.",
        (Ui::TryAgain, En) => "try again",
        (Ui::TryAgain, De) => "nochmal versuchen",
        (Ui::TryAgain, Es) => "intentarlo de nuevo",
    }
}

/// The bold face for titles. The regular face replaces Bevy's default font,
/// so every text without an explicit font already uses it.
#[derive(Resource)]
pub struct Fonts {
    pub bold: Handle<Font>,
}

pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
        fonts
            .insert(AssetId::default(), Font::from_bytes(FONT_REGULAR.to_vec()))
            .expect("the default font slot accepts a font");
        let bold = fonts.add(Font::from_bytes(FONT_BOLD.to_vec()));

        app.insert_resource(Fonts { bold })
            .init_resource::<Language>()
            .add_systems(Update, cycle_language);
    }
}

fn cycle_language(keys: Res<ButtonInput<KeyCode>>, mut language: ResMut<Language>) {
    if keys.just_pressed(KeyCode::KeyL) {
        *language = language.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_translation_falls_back_to_english() {
        let text = Tr {
            en: "Hello".into(),
            de: String::new(),
            es: "Hola".into(),
        };
        assert_eq!(text.get(Language::De), "Hello");
        assert_eq!(text.get(Language::Es), "Hola");
        assert!(!text.is_complete());
    }
}
