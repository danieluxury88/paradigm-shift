//! A level solver for the tests: a best-first search over the game's own
//! physics (`player::step`, `lens::try_shift`) that finds a sequence of
//! inputs reaching the goal. The simulation tests then replay that plan in
//! the real Bevy app, so "every level is solvable" is proven, not assumed.
//!
//! Straw is treated as empty ground: it collapses a quarter of a second after
//! the player lands on it, so a route must never depend on it.

use std::collections::{BinaryHeap, HashSet};

use bevy::prelude::*;

use crate::lens::{Lens, try_shift};
use crate::level::{Cell, LevelGrid};
use crate::player::{MoveInput, Player, step, touches_goal};

/// Frames each search action is held for. Shorter finds tighter routes but
/// searches more states.
pub const FRAMES_PER_ACTION: usize = 4;

/// One search action, held for `FRAMES_PER_ACTION` frames. Jump and SHIFT
/// act on the first of those frames only, like a key tap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub direction: i8,
    pub jump: bool,
    pub shift: bool,
}

/// The state the search explores: exactly what the game would have.
#[derive(Clone)]
pub struct State {
    pub position: Vec2,
    pub player: Player,
    pub lens: Lens,
}

/// Coarse key for the visited set. Two states with the same key are treated
/// as equivalent; the plan itself always uses exact positions.
fn key(state: &State) -> (i32, i32, i32, bool, bool) {
    (
        (state.position.x / 3.0).round() as i32,
        (state.position.y / 3.0).round() as i32,
        (state.player.velocity.y / 60.0).round() as i32,
        state.player.on_ground,
        state.lens == Lens::First,
    )
}

/// Applies one action. Returns `None` when the player fell out of the map.
/// Sets `reached` if the goal was touched on any of the frames.
fn apply(
    grid: &LevelGrid,
    state: &State,
    action: Action,
    delta: f32,
    reached: &mut bool,
) -> Option<State> {
    let mut next = state.clone();
    for frame in 0..FRAMES_PER_ACTION {
        let first = frame == 0;
        if first && action.shift {
            // A refused shift changes nothing, as in the game.
            if let Some(lens) = try_shift(grid, next.position, next.lens) {
                next.lens = lens;
            }
        }
        let input = MoveInput {
            direction: action.direction,
            jump: first && action.jump,
        };
        step(
            &mut next.position,
            &mut next.player,
            input,
            grid,
            next.lens,
            delta,
        );
        if next.position.y < grid.bottom() {
            return None;
        }
        if touches_goal(grid, next.position) {
            *reached = true;
            return Some(next);
        }
    }
    Some(next)
}

struct Node {
    score: i64,
    index: usize,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Node {
    // `BinaryHeap` is a max-heap: a lower score must compare as greater.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.score.cmp(&self.score)
    }
}

/// Finds a plan from `start` to the goal, or `None` if the search space is
/// exhausted. `delta` is the frame time the replay will use.
pub fn solve(grid: &LevelGrid, start: State, delta: f32) -> Option<Vec<Action>> {
    search(grid, start, delta, true)
}

/// Like `solve`, but never pressing SHIFT: proves a level needs both lenses.
pub fn solve_without_shift(grid: &LevelGrid, start: State, delta: f32) -> Option<Vec<Action>> {
    search(grid, start, delta, false)
}

fn search(grid: &LevelGrid, start: State, delta: f32, allow_shift: bool) -> Option<Vec<Action>> {
    // A copy with straw removed: a route must not depend on straw.
    let mut grid = grid.clone();
    for cell in grid.cells.iter_mut().flatten() {
        if *cell == Cell::Straw {
            *cell = Cell::Empty;
        }
    }
    let goal = grid.find(Cell::Goal).expect("validated at startup");

    let mut actions = Vec::new();
    for direction in [1, 0, -1] {
        for jump in [false, true] {
            for shift in [false, allow_shift] {
                let action = Action {
                    direction,
                    jump,
                    shift,
                };
                if !actions.contains(&action) {
                    actions.push(action);
                }
            }
        }
    }

    // Nodes are stored once; the heap holds indices. `parent` rebuilds the plan.
    let mut states: Vec<(State, Option<(usize, Action)>, usize)> = vec![(start.clone(), None, 0)];
    let mut visited = HashSet::new();
    visited.insert(key(&start));
    let mut heap = BinaryHeap::new();
    heap.push(Node { score: 0, index: 0 });

    while let Some(Node { index, .. }) = heap.pop() {
        let depth = states[index].2;
        for &action in &actions {
            let mut reached = false;
            let Some(next) = apply(&grid, &states[index].0, action, delta, &mut reached) else {
                continue;
            };
            if reached {
                let mut plan = vec![action];
                let mut at = index;
                while let Some((parent, step)) = states[at].1 {
                    plan.push(step);
                    at = parent;
                }
                plan.reverse();
                return Some(plan);
            }
            if !visited.insert(key(&next)) {
                continue;
            }
            // Greedy towards the goal, with a small cost per step so the
            // search does not wander forever.
            let distance = next.position.distance(goal);
            let score = (distance * 4.0) as i64 + depth as i64 * 8;
            states.push((next, Some((index, action)), depth + 1));
            heap.push(Node {
                score,
                index: states.len() - 1,
            });
        }
        if states.len() > 400_000 {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_a_flat_walk() {
        let grid = LevelGrid::parse(&["P...G".to_string(), "#####".to_string()]).unwrap();
        let start = State {
            position: grid.find(Cell::Start).unwrap(),
            player: Player::default(),
            lens: Lens::First,
        };
        let plan = solve(&grid, start, 1.0 / 60.0).expect("a plan");
        assert!(plan.iter().all(|a| a.direction >= 0));
    }

    #[test]
    fn reports_an_impossible_level() {
        // The goal sits behind a wall far taller than any jump.
        let mut map = vec!["......#..".to_string(); 8];
        map.push("P.....#.G".to_string());
        map.push("#########".to_string());
        let grid = LevelGrid::parse(&map).unwrap();
        let start = State {
            position: grid.find(Cell::Start).unwrap(),
            player: Player::default(),
            lens: Lens::First,
        };
        assert!(solve(&grid, start, 1.0 / 60.0).is_none());
    }
}
