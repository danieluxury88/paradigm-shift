//! Sound: effects for what the player does, and music once a level unlocks
//! it (`Feature::Music` in `levels/worlds.ron`).
//!
//! Every sound comes from the `SoundBank`, which `build_bank` fills. Today it
//! synthesizes them (`synth.rs`); to use recorded files instead, load them
//! there with the `AssetServer` (and enable the matching Bevy audio format
//! feature): nothing else changes.
//!
//! The audio listens to messages the game already sends (jump, shift,
//! respawn, goal), so no gameplay module knows about sound. Each world's
//! music has two versions, one per lens, that play in sync; SHIFT crossfades
//! between them, so the same song is heard another way. M mutes everything,
//! and the choice is saved.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

use crate::AppState;
use crate::campaign::{Current, Feature, unlocked};
use crate::lens::{ActiveLens, Lens, ShiftBlocked, Shifted};
use crate::player::{Jumped, LevelCompleted, Respawned};
use crate::save::Settings;
use crate::synth::{self, Effect};

/// Loudest music volume; effects play at full volume over it.
const MUSIC_VOLUME: f32 = 0.6;
/// How fast the music crossfades on SHIFT, in volume per second.
const CROSSFADE_SPEED: f32 = 4.0;

/// Every sound the game can play.
#[derive(Resource)]
pub struct SoundBank {
    effects: HashMap<Effect, Handle<AudioSource>>,
    /// Per world: the music for the first and the second lens.
    music: Vec<[Handle<AudioSource>; 2]>,
}

/// The one place that decides where sounds come from.
fn build_bank(sources: &mut Assets<AudioSource>, worlds: usize) -> SoundBank {
    let mut add = |samples: Vec<f32>| {
        sources.add(AudioSource {
            bytes: Arc::from(synth::wav(&samples)),
        })
    };
    let effects = Effect::ALL
        .iter()
        .map(|&e| (e, add(synth::effect(e))))
        .collect();
    let music = (0..worlds)
        .map(|w| [add(synth::music(w, false)), add(synth::music(w, true))])
        .collect();
    SoundBank { effects, music }
}

/// A playing music loop: which world and which lens version it is.
#[derive(Component)]
struct MusicTrack {
    world: usize,
    lens: Lens,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_bank)
            .add_systems(Update, play_effects.run_if(in_state(AppState::Playing)))
            .add_systems(
                OnEnter(AppState::Playing),
                start_world_music.run_if(unlocked(Feature::Music)),
            )
            .add_systems(Update, (toggle_mute, crossfade_music).chain());
    }
}

fn setup_bank(
    mut commands: Commands,
    mut sources: ResMut<Assets<AudioSource>>,
    campaign: Res<crate::campaign::Campaign>,
) {
    commands.insert_resource(build_bank(&mut sources, campaign.worlds.len()));
}

/// The game messages that make a sound.
#[derive(bevy::ecs::system::SystemParam)]
struct SoundCues<'w, 's> {
    jumped: MessageReader<'w, 's, Jumped>,
    shifted: MessageReader<'w, 's, Shifted>,
    blocked: MessageReader<'w, 's, ShiftBlocked>,
    respawned: MessageReader<'w, 's, Respawned>,
    completed: MessageReader<'w, 's, LevelCompleted>,
}

impl SoundCues<'_, '_> {
    /// The effects due this frame.
    fn effects(&mut self) -> Vec<Effect> {
        [
            (Effect::Jump, self.jumped.read().count()),
            (Effect::Shift, self.shifted.read().count()),
            (Effect::Blocked, self.blocked.read().count()),
            (Effect::Fall, self.respawned.read().count()),
            (Effect::Goal, self.completed.read().count()),
        ]
        .into_iter()
        .filter(|&(_, count)| count > 0)
        .map(|(effect, _)| effect)
        .collect()
    }
}

fn play_effects(
    mut commands: Commands,
    bank: Res<SoundBank>,
    settings: Res<Settings>,
    mut cues: SoundCues,
) {
    let effects = cues.effects();
    if settings.muted {
        return;
    }
    for effect in effects {
        commands.spawn((
            AudioPlayer(bank.effects[&effect].clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}

/// Starts both versions of the current world's music, unless they are
/// already playing: music carries on across the levels of a world.
fn start_world_music(
    mut commands: Commands,
    bank: Res<SoundBank>,
    current: Current,
    tracks: Query<(Entity, &MusicTrack)>,
) {
    let world = current.level().world;
    if tracks.iter().any(|(_, track)| track.world == world) {
        return;
    }
    for (entity, _) in &tracks {
        commands.entity(entity).despawn();
    }
    for (index, lens) in [Lens::First, Lens::Second].into_iter().enumerate() {
        commands.spawn((
            AudioPlayer(bank.music[world][index].clone()),
            // Start silent; `crossfade_music` brings the active lens in.
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
            MusicTrack { world, lens },
        ));
    }
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut settings: ResMut<Settings>,
) {
    // While typing an answer, M is just a letter.
    if *state.get() != AppState::Prompt && keys.just_pressed(KeyCode::KeyM) {
        settings.muted = !settings.muted;
    }
}

fn crossfade_music(
    time: Res<Time>,
    lens: Res<ActiveLens>,
    settings: Res<Settings>,
    mut tracks: Query<(&MusicTrack, &mut AudioSink)>,
) {
    let step = CROSSFADE_SPEED * time.delta_secs();
    for (track, mut sink) in &mut tracks {
        let target = if settings.muted || track.lens != lens.0 {
            0.0
        } else {
            MUSIC_VOLUME
        };
        let now = sink.volume().to_linear();
        let next = now + (target - now).clamp(-step, step);
        sink.set_volume(Volume::Linear(next));
    }
}
