//! Levels: the data in each RON file, the map as a grid of cells, and
//! spawning tiles. Which levels exist and in what order is the campaign's
//! business (`campaign.rs`); files are embedded in the binary by build.rs, so
//! the web build needs no asset loading.

use bevy::prelude::*;
use serde::Deserialize;

use crate::campaign::Current;
use crate::i18n::{Language, Tr};
use crate::lens::Lens;
use crate::{AppState, TILE};

/// The concept a level teaches, shown on a card once the level is solved.
#[derive(Debug, Clone, Deserialize)]
pub struct ConceptCard {
    pub term: Tr,
    pub origin: Tr,
    pub definition: Tr,
    pub example: Tr,
    pub practice: Tr,
}

/// One level exactly as written in its `.ron` file.
#[derive(Debug, Clone, Deserialize)]
pub struct LevelData {
    pub title: Tr,
    pub hint: Tr,
    pub lenses: (Tr, Tr),
    pub map: Vec<String>,
    pub card: ConceptCard,
}

impl LevelData {
    pub fn lens_name(&self, lens: Lens, language: Language) -> &str {
        match lens {
            Lens::First => self.lenses.0.get(language),
            Lens::Second => self.lenses.1.get(language),
        }
    }

    /// Every translatable text of the level, for the completeness test.
    #[cfg(test)]
    fn texts(&self) -> [&Tr; 9] {
        let c = &self.card;
        [
            &self.title,
            &self.hint,
            &self.lenses.0,
            &self.lenses.1,
            &c.term,
            &c.origin,
            &c.definition,
            &c.example,
            &c.practice,
        ]
    }
}

/// What a single map cell contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Solid,
    LensOnly(Lens),
    /// Looks like solid ground but the whole straw structure collapses
    /// shortly after the player stands on it (see `straw.rs`).
    Straw,
    Start,
    Goal,
}

impl Cell {
    fn from_char(c: char) -> Result<Cell, String> {
        Ok(match c {
            '.' => Cell::Empty,
            '#' => Cell::Solid,
            'A' => Cell::LensOnly(Lens::First),
            'B' => Cell::LensOnly(Lens::Second),
            'S' => Cell::Straw,
            'P' => Cell::Start,
            'G' => Cell::Goal,
            other => return Err(format!("unknown map character {other:?}")),
        })
    }

    pub fn is_solid_in(self, lens: Lens) -> bool {
        match self {
            Cell::Solid | Cell::Straw => true,
            Cell::LensOnly(owner) => owner == lens,
            _ => false,
        }
    }
}

/// The current level as a grid of cells, for fast collision lookups.
/// Row 0 is the top of the map, as written in the file.
#[derive(Resource, Default, Clone)]
pub struct LevelGrid {
    pub cells: Vec<Vec<Cell>>,
    /// The cells as loaded, to undo collapses when the player respawns.
    pub original: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
}

impl LevelGrid {
    pub fn parse(map: &[String]) -> Result<LevelGrid, String> {
        let width = map.first().map(|row| row.chars().count()).unwrap_or(0);
        let mut cells = Vec::with_capacity(map.len());
        for (index, row) in map.iter().enumerate() {
            if row.chars().count() != width {
                return Err(format!("row {index} has a different width than row 0"));
            }
            cells.push(
                row.chars()
                    .map(Cell::from_char)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        let grid = LevelGrid {
            original: cells.clone(),
            cells,
            width,
            height: map.len(),
        };
        for required in [Cell::Start, Cell::Goal] {
            let count = grid
                .cells
                .iter()
                .flatten()
                .filter(|&&c| c == required)
                .count();
            if count != 1 {
                return Err(format!("expected exactly one {required:?}, found {count}"));
            }
        }
        Ok(grid)
    }

    /// World position of the centre of a cell. The map is centred on the origin.
    pub fn cell_center(&self, col: usize, row: usize) -> Vec2 {
        Vec2::new(
            (col as f32 + 0.5) * TILE - self.width as f32 * TILE / 2.0,
            (self.height as f32 / 2.0 - row as f32 - 0.5) * TILE,
        )
    }

    /// The cell that contains a world position, if it is inside the map.
    fn cell_at(&self, point: Vec2) -> Option<(usize, usize)> {
        let col = ((point.x + self.width as f32 * TILE / 2.0) / TILE).floor();
        let row = ((self.height as f32 * TILE / 2.0 - point.y) / TILE).floor();
        if col < 0.0 || row < 0.0 || col >= self.width as f32 || row >= self.height as f32 {
            return None;
        }
        Some((col as usize, row as usize))
    }

    pub fn find(&self, wanted: Cell) -> Option<Vec2> {
        self.cells.iter().enumerate().find_map(|(row, cells)| {
            cells
                .iter()
                .position(|&c| c == wanted)
                .map(|col| self.cell_center(col, row))
        })
    }

    /// Lowest world y of the map; falling below it means the player fell out.
    pub fn bottom(&self) -> f32 {
        -(self.height as f32) * TILE / 2.0
    }

    /// Every cell touched by a box, as (col, row, cell).
    pub fn cells_in_box(&self, center: Vec2, half: Vec2) -> Vec<(usize, usize, Cell)> {
        // Shrink the box slightly so that touching an edge does not count as overlap.
        let epsilon = Vec2::splat(0.01);
        let (Some(a), Some(b)) = (
            self.cell_at_clamped(center - half + epsilon),
            self.cell_at_clamped(center + half - epsilon),
        ) else {
            return Vec::new();
        };
        // `a` is the bottom-left corner, `b` the top-right; rows grow downwards.
        self.cells[b.1..=a.1]
            .iter()
            .zip(b.1..)
            .flat_map(|(cells, row)| {
                cells[a.0..=b.0]
                    .iter()
                    .zip(a.0..)
                    .map(move |(&cell, col)| (col, row, cell))
            })
            .collect()
    }

    fn cell_at_clamped(&self, point: Vec2) -> Option<(usize, usize)> {
        let min = Vec2::new(-(self.width as f32) * TILE / 2.0, self.bottom());
        let max = -min - Vec2::splat(0.001);
        self.cell_at(point.clamp(min, max))
    }

    pub fn overlaps_solid(&self, center: Vec2, half: Vec2, lens: Lens) -> bool {
        self.cells_in_box(center, half)
            .iter()
            .any(|&(_, _, cell)| cell.is_solid_in(lens))
    }
}

/// Marks a spawned tile entity with the cell it represents.
#[derive(Component)]
pub struct Tile(pub Cell);

/// The (column, row) of a tile in the map.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos(pub usize, pub usize);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelGrid>()
            .add_systems(OnEnter(AppState::Playing), spawn_level);
    }
}

pub fn spawn_level(mut commands: Commands, current: Current, mut grid: ResMut<LevelGrid>) {
    // Already validated when the campaign loaded, so this cannot fail here.
    *grid = LevelGrid::parse(&current.level().data.map).expect("validated at startup");

    for (row, cells) in grid.cells.iter().enumerate() {
        for (col, &cell) in cells.iter().enumerate() {
            if matches!(cell, Cell::Empty | Cell::Start) {
                continue;
            }
            let size = if cell == Cell::Goal {
                TILE * 0.6
            } else {
                TILE - 2.0
            };
            commands.spawn((
                Tile(cell),
                GridPos(col, row),
                // The colour is set by `lens::paint_tiles`.
                Sprite::from_color(Color::WHITE, Vec2::splat(size)),
                Transform::from_translation(grid.cell_center(col, row).extend(0.0)),
                // Removed automatically when the level ends.
                DespawnOnExit(AppState::Playing),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::Campaign;

    #[test]
    fn every_level_text_is_translated() {
        for level in Campaign::load().levels {
            for text in level.data.texts() {
                assert!(
                    text.is_complete(),
                    "level {}: missing translation for {:?}",
                    level.id,
                    text.en
                );
            }
        }
    }

    #[test]
    fn rejects_rows_of_different_width() {
        let map = vec!["P.G".to_string(), "##".to_string()];
        assert!(LevelGrid::parse(&map).is_err());
    }

    #[test]
    fn lens_tiles_are_solid_only_in_their_lens() {
        assert!(Cell::LensOnly(Lens::First).is_solid_in(Lens::First));
        assert!(!Cell::LensOnly(Lens::First).is_solid_in(Lens::Second));
        assert!(Cell::Solid.is_solid_in(Lens::Second));
        assert!(Cell::Straw.is_solid_in(Lens::First) && Cell::Straw.is_solid_in(Lens::Second));
    }
}
