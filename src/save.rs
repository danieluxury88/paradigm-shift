//! Saving: the player's `Progress` and language survive closing the game.
//!
//! The save is a small RON text in a `Store`: the browser's `localStorage` on
//! the web build, a file in the platform's data folder natively, memory in
//! tests. It is loaded while the app is built, so the first frame already
//! has it, and written whenever the progress or the language changes. A save
//! that cannot be read (old version, broken text, a level that no longer
//! exists) is ignored rather than stopping the game.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::campaign::{Campaign, Progress};
use crate::i18n::Language;

/// Bump when the format changes in a way old saves cannot be read.
const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct SaveFile {
    version: u32,
    progress: Progress,
    language: Language,
}

/// Where the save text lives.
pub trait Store: Send + Sync + 'static {
    fn load(&self) -> Option<String>;
    fn save(&self, text: &str);
}

/// The store in use, as a resource.
#[derive(Resource)]
pub struct SaveStore(pub Box<dyn Store>);

/// A store kept in memory: for tests, and when no other store is available.
#[derive(Default)]
pub struct MemoryStore(std::sync::Mutex<Option<String>>);

impl Store for MemoryStore {
    fn load(&self) -> Option<String> {
        self.0.lock().ok()?.clone()
    }
    fn save(&self, text: &str) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(text.to_string());
        }
    }
}

/// Lets several apps share one store, e.g. two sessions in a test.
impl<T: Store> Store for std::sync::Arc<T> {
    fn load(&self) -> Option<String> {
        (**self).load()
    }
    fn save(&self, text: &str) {
        (**self).save(text);
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    //! The browser's `localStorage`, under one key.
    const KEY: &str = "paradigm-shift-save";

    pub struct BrowserStore;

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    impl super::Store for BrowserStore {
        fn load(&self) -> Option<String> {
            storage()?.get_item(KEY).ok()?
        }
        fn save(&self, text: &str) {
            // Private windows may refuse storage; the game still plays.
            if let Some(storage) = storage() {
                let _ = storage.set_item(KEY, text);
            }
        }
    }

    /// `localStorage` when the browser allows it; otherwise (some private
    /// windows) memory, so the game plays without saving.
    pub fn default_store() -> Box<dyn super::Store> {
        if storage().is_some() {
            Box::new(BrowserStore)
        } else {
            Box::new(super::MemoryStore::default())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    //! A file in the platform's per-user data folder.
    use std::path::PathBuf;

    pub struct FileStore(pub PathBuf);

    impl super::Store for FileStore {
        fn load(&self) -> Option<String> {
            std::fs::read_to_string(&self.0).ok()
        }
        fn save(&self, text: &str) {
            if let Some(dir) = self.0.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(error) = std::fs::write(&self.0, text) {
                bevy::log::warn!("could not save to {}: {error}", self.0.display());
            }
        }
    }

    /// `%APPDATA%` on Windows, `~/Library/Application Support` on macOS,
    /// `$XDG_DATA_HOME` or `~/.local/share` elsewhere.
    fn data_dir() -> Option<PathBuf> {
        let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
        if cfg!(windows) {
            env("APPDATA")
        } else if cfg!(target_os = "macos") {
            env("HOME").map(|home| home.join("Library/Application Support"))
        } else {
            env("XDG_DATA_HOME").or_else(|| env("HOME").map(|home| home.join(".local/share")))
        }
    }

    pub fn default_store() -> Box<dyn super::Store> {
        match data_dir() {
            Some(dir) => Box::new(FileStore(dir.join("paradigm-shift").join("save.ron"))),
            None => Box::new(super::MemoryStore::default()),
        }
    }
}

/// Reads a save; `None` if it is missing, from another version or invalid
/// for this campaign.
fn parse(text: &str, campaign: &Campaign) -> Option<(Progress, Language)> {
    let file: SaveFile = ron::from_str(text).ok()?;
    if file.version != VERSION || file.progress.level >= campaign.levels.len() {
        return None;
    }
    Some((file.progress, file.language))
}

fn serialize(progress: &Progress, language: Language) -> String {
    let file = SaveFile {
        version: VERSION,
        progress: progress.clone(),
        language,
    };
    ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
        .expect("plain data always serializes")
}

/// Whether a saved game was found at startup, for the title screen.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SaveFound(pub bool);

/// Loads the save from the store. Add after `CampaignPlugin` and
/// `I18nPlugin`; insert a `SaveStore` first to use another store.
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<SaveStore>() {
            app.insert_resource(SaveStore(platform::default_store()));
        }
        let world = app.world();
        let loaded = world
            .resource::<SaveStore>()
            .0
            .load()
            .and_then(|text| parse(&text, world.resource::<Campaign>()));
        app.insert_resource(SaveFound(loaded.is_some()));
        if let Some((progress, language)) = loaded {
            app.insert_resource(progress).insert_resource(language);
        }
        app.add_systems(Last, write_on_change);
    }
}

fn write_on_change(store: Res<SaveStore>, progress: Res<Progress>, language: Res<Language>) {
    let changed = progress.is_changed() || language.is_changed();
    let first_frame = progress.is_added() && language.is_added();
    if changed && !first_frame {
        store.0.save(&serialize(&progress, *language));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_round_trips() {
        let campaign = Campaign::load();
        let progress = Progress { level: 4 };
        let text = serialize(&progress, Language::Es);
        assert_eq!(parse(&text, &campaign), Some((progress, Language::Es)));
    }

    #[test]
    fn ignores_broken_old_or_out_of_range_saves() {
        let campaign = Campaign::load();
        assert_eq!(parse("not ron", &campaign), None);
        let old = serialize(&Progress::default(), Language::En).replace("version: 1", "version: 0");
        assert_eq!(parse(&old, &campaign), None);
        let beyond = Progress {
            level: campaign.levels.len(),
        };
        assert_eq!(parse(&serialize(&beyond, Language::En), &campaign), None);
    }
}
