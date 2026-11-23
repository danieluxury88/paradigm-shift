//! Remote control for coding agents, built only with the `brp` feature
//! (`make brp`). It serves the Bevy Remote Protocol on 127.0.0.1:15702 so an
//! agent using the `bevy_brp_mcp` server can query entities, read the active
//! lens, take screenshots and send key presses while the game runs.

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

use crate::lens::{ActiveLens, Lens};
use crate::player::Player;

pub struct BrpPlugin;

impl Plugin for BrpPlugin {
    fn build(&self, app: &mut App) {
        // BRP can only see types registered for reflection.
        app.add_plugins(BrpExtrasPlugin)
            .register_type::<Player>()
            .register_type::<ActiveLens>()
            .register_type::<Lens>();
    }
}
