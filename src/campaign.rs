//! The campaign: worlds and levels in play order, read from
//! `levels/worlds.ron`, and the player's progress through them.
//!
//! This is the layer the incremental features hang from. The content
//! (`Campaign`) never changes while playing; the player's state (`Progress`)
//! does, and is what gets saved. A level can unlock a `Feature`; a feature
//! plugin gates its systems with `run_if(unlocked(Feature::X))`, so the game
//! grows as the player advances without the plugins knowing about levels.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::Tr;
use crate::level::{LevelData, LevelGrid};

// `LEVEL_FILES`: every `levels/*.ron` except the manifest, embedded by build.rs.
include!(concat!(env!("OUT_DIR"), "/level_files.rs"));

const MANIFEST: &str = include_str!("../levels/worlds.ron");

/// Something the game gains as the player advances. Declared per level in
/// `levels/worlds.ron`; once unlocked it stays on for every later level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Feature {
    /// A per-world budget of lives; falling or restarting costs one.
    Lives,
}

/// What running out of lives costs, per world: the challenge grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum OutOfLives {
    /// Start the current level again.
    #[default]
    RestartLevel,
    /// Go back to the world's first level.
    RestartWorld,
}

fn default_lives() -> u32 {
    5
}

#[derive(Deserialize)]
struct Manifest {
    worlds: Vec<WorldEntry>,
}

#[derive(Deserialize)]
struct WorldEntry {
    name: Tr,
    #[serde(default = "default_lives")]
    lives: u32,
    #[serde(default)]
    out_of_lives: OutOfLives,
    levels: Vec<LevelEntry>,
}

#[derive(Deserialize)]
struct LevelEntry {
    file: String,
    #[serde(default)]
    unlocks: Vec<Feature>,
}

/// A world: a setting with its own question, holding consecutive levels.
#[derive(Debug, Clone)]
pub struct World {
    pub name: Tr,
    /// Lives for the whole world, once `Feature::Lives` is unlocked.
    pub lives: u32,
    pub out_of_lives: OutOfLives,
    /// Index of the world's first level in `Campaign::levels`.
    pub first_level: usize,
}

/// A level in campaign order, with its place in the progression.
#[derive(Debug, Clone)]
pub struct Level {
    /// File name without `.ron`, e.g. `01-two-ways-of-seeing`.
    pub id: String,
    /// Index into `Campaign::worlds`.
    pub world: usize,
    pub unlocks: Vec<Feature>,
    pub data: LevelData,
}

/// Every world and level, parsed and validated once at startup.
#[derive(Resource, Debug)]
pub struct Campaign {
    pub worlds: Vec<World>,
    pub levels: Vec<Level>,
}

impl Campaign {
    /// Parses the manifest and every level it names. Errors name the file.
    pub fn parse(manifest: &str, files: &[(&str, &str)]) -> Result<Campaign, String> {
        let manifest: Manifest = ron::from_str(manifest).map_err(|e| format!("worlds.ron: {e}"))?;
        let mut worlds = Vec::new();
        let mut levels: Vec<Level> = Vec::new();
        for (world_index, entry) in manifest.worlds.into_iter().enumerate() {
            if entry.levels.is_empty() {
                return Err(format!("worlds.ron: world {world_index} has no levels"));
            }
            if entry.lives == 0 {
                return Err(format!("worlds.ron: world {world_index} has 0 lives"));
            }
            worlds.push(World {
                name: entry.name,
                lives: entry.lives,
                out_of_lives: entry.out_of_lives,
                first_level: levels.len(),
            });
            for level in entry.levels {
                if levels.iter().any(|l| l.id == level.file) {
                    return Err(format!("worlds.ron: {} is listed twice", level.file));
                }
                let (_, source) = files
                    .iter()
                    .find(|(stem, _)| *stem == level.file)
                    .ok_or_else(|| format!("worlds.ron: no file levels/{}.ron", level.file))?;
                let data: LevelData =
                    ron::from_str(source).map_err(|e| format!("level {}: {e}", level.file))?;
                LevelGrid::parse(&data.map).map_err(|e| format!("level {}: {e}", level.file))?;
                levels.push(Level {
                    id: level.file,
                    world: world_index,
                    unlocks: level.unlocks,
                    data,
                });
            }
        }
        if let Some((stem, _)) = files
            .iter()
            .find(|(stem, _)| !levels.iter().any(|l| l.id == *stem))
        {
            return Err(format!("levels/{stem}.ron is not listed in worlds.ron"));
        }
        if levels.is_empty() {
            return Err("worlds.ron: no levels".into());
        }
        Ok(Campaign { worlds, levels })
    }

    /// The embedded campaign. A broken file stops the game at startup with a
    /// clear message, instead of failing later in the middle of a level.
    pub fn load() -> Campaign {
        Campaign::parse(MANIFEST, LEVEL_FILES).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn world_of(&self, level: usize) -> &World {
        &self.worlds[self.levels[level].world]
    }

    /// Whether `feature` is on while playing `level`: some level up to and
    /// including it unlocks the feature.
    pub fn is_unlocked(&self, feature: Feature, level: usize) -> bool {
        self.levels[..=level]
            .iter()
            .any(|l| l.unlocks.contains(&feature))
    }
}

/// The player's state across levels: what gets saved between sessions
/// (see `save.rs`). New fields need `#[serde(default)]` so older saves load.
#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Index into `Campaign::levels` of the level being played.
    pub level: usize,
}

impl Progress {
    /// Moves to the next level, wrapping around after the last one.
    pub fn advance(&mut self, campaign: &Campaign) {
        self.level = (self.level + 1) % campaign.levels.len();
    }

    pub fn is_last(&self, campaign: &Campaign) -> bool {
        self.level + 1 == campaign.levels.len()
    }
}

/// The level being played, its world and the player's progress: what most
/// screens need, in one system parameter.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Current<'w> {
    pub campaign: Res<'w, Campaign>,
    pub progress: Res<'w, Progress>,
}

impl Current<'_> {
    pub fn level(&self) -> &Level {
        &self.campaign.levels[self.progress.level]
    }

    pub fn world(&self) -> &World {
        self.campaign.world_of(self.progress.level)
    }

    /// 1-based number shown to the player.
    pub fn number(&self) -> usize {
        self.progress.level + 1
    }
}

/// Run condition: the feature is unlocked at the level being played.
pub fn unlocked(feature: Feature) -> impl Fn(Res<Campaign>, Res<Progress>) -> bool + Clone {
    move |campaign: Res<Campaign>, progress: Res<Progress>| {
        campaign.is_unlocked(feature, progress.level)
    }
}

pub struct CampaignPlugin;

impl Plugin for CampaignPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Campaign::load())
            .init_resource::<Progress>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: &str = r#"(
        title: (en: "T", de: "T", es: "T"),
        hint: (en: "H", de: "H", es: "H"),
        lenses: ((en: "A"), (en: "B")),
        map: ["P.G"],
        card: (
            term: (en: "x"), origin: (en: "x"), definition: (en: "x"),
            example: (en: "x"), practice: (en: "x"),
        ),
    )"#;

    fn world(levels: &str) -> String {
        format!("(worlds: [(name: (en: \"W\"), levels: [{levels}])])")
    }

    #[test]
    fn the_embedded_campaign_is_valid() {
        let campaign = Campaign::load();
        assert_eq!(campaign.levels.len(), LEVEL_FILES.len());
        assert_eq!(campaign.worlds[0].first_level, 0);
    }

    #[test]
    fn every_world_name_is_translated() {
        for world in Campaign::load().worlds {
            assert!(world.name.is_complete(), "{:?}", world.name.en);
        }
    }

    #[test]
    fn rejects_a_level_missing_from_the_manifest() {
        let files = [("a", LEVEL), ("b", LEVEL)];
        let error = Campaign::parse(&world("(file: \"a\")"), &files).unwrap_err();
        assert!(error.contains("levels/b.ron is not listed"), "{error}");
    }

    #[test]
    fn rejects_a_manifest_entry_without_a_file() {
        let files = [("a", LEVEL)];
        let error = Campaign::parse(&world("(file: \"a\"), (file: \"zz\")"), &files).unwrap_err();
        assert!(error.contains("no file levels/zz.ron"), "{error}");
    }

    #[test]
    fn rejects_a_level_listed_twice() {
        let files = [("a", LEVEL)];
        let error = Campaign::parse(&world("(file: \"a\"), (file: \"a\")"), &files).unwrap_err();
        assert!(error.contains("listed twice"), "{error}");
    }

    #[test]
    fn progress_wraps_after_the_last_level() {
        let campaign = Campaign::load();
        let mut progress = Progress {
            level: campaign.levels.len() - 1,
        };
        assert!(progress.is_last(&campaign));
        progress.advance(&campaign);
        assert_eq!(progress.level, 0);
    }
}
