//! A tiny synthesizer: the game's sound effects and music, generated as WAV
//! bytes at startup instead of loaded from files. It keeps the web bundle
//! small and the licensing simple until real audio replaces it.
//!
//! Nothing outside `audio::build_bank` calls this module, so swapping in
//! recorded files only changes that one function.

use std::f32::consts::TAU;

/// Samples per second. Plenty for chiptune-like tones, half the memory of CD
/// quality.
pub const RATE: u32 = 22_050;

#[derive(Debug, Clone, Copy)]
enum Wave {
    Sine,
    Triangle,
    Square,
}

impl Wave {
    /// One sample at `phase` (in cycles, 0..1).
    fn at(self, phase: f32) -> f32 {
        let p = phase.fract();
        match self {
            Wave::Sine => (p * TAU).sin(),
            Wave::Triangle => 1.0 - 4.0 * (p - 0.5).abs(),
            Wave::Square => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

/// A note: a pitch glide from `from` to `to` Hz with a fast attack and an
/// exponential decay, written into `out` starting at `start` seconds.
#[derive(Debug, Clone, Copy)]
struct Note {
    from: f32,
    to: f32,
    start: f32,
    length: f32,
    wave: Wave,
    volume: f32,
    /// How fast the note fades; higher is shorter.
    decay: f32,
}

impl Note {
    fn render(&self, out: &mut [f32]) {
        let first = (self.start * RATE as f32) as usize;
        let count = (self.length * RATE as f32) as usize;
        let mut phase = 0.0f32;
        for i in 0..count {
            let Some(sample) = out.get_mut(first + i) else {
                break;
            };
            let t = i as f32 / RATE as f32;
            let progress = i as f32 / count as f32;
            let freq = self.from + (self.to - self.from) * progress;
            phase += freq / RATE as f32;
            let attack = (t / 0.005).min(1.0);
            // Fade the last 10 ms so a cut note never clicks.
            let release = ((count - i) as f32 / (0.01 * RATE as f32)).min(1.0);
            let envelope = attack * (-self.decay * t).exp() * release;
            *sample += self.wave.at(phase) * self.volume * envelope;
        }
    }
}

fn silence(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * RATE as f32) as usize]
}

fn midi(note: i32) -> f32 {
    440.0 * 2f32.powf((note - 69) as f32 / 12.0)
}

/// Mono 16-bit PCM WAV bytes for the samples (clamped to -1..1).
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    out
}

/// The sound effects, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Effect {
    Jump,
    Shift,
    Blocked,
    Fall,
    Goal,
    Reinforce,
}

impl Effect {
    pub const ALL: [Effect; 6] = [
        Effect::Jump,
        Effect::Shift,
        Effect::Blocked,
        Effect::Fall,
        Effect::Goal,
        Effect::Reinforce,
    ];
}

/// Samples of one sound effect.
pub fn effect(effect: Effect) -> Vec<f32> {
    let note = |from: f32, to: f32, start: f32, length: f32, wave: Wave, decay: f32| Note {
        from,
        to,
        start,
        length,
        wave,
        volume: 0.35,
        decay,
    };
    let notes: Vec<Note> = match effect {
        Effect::Jump => vec![note(330.0, 660.0, 0.0, 0.12, Wave::Square, 14.0)],
        Effect::Shift => vec![
            note(523.0, 523.0, 0.0, 0.18, Wave::Sine, 10.0),
            note(784.0, 784.0, 0.04, 0.18, Wave::Sine, 10.0),
        ],
        Effect::Blocked => vec![note(140.0, 110.0, 0.0, 0.16, Wave::Square, 8.0)],
        Effect::Fall => vec![note(600.0, 120.0, 0.0, 0.4, Wave::Triangle, 4.0)],
        Effect::Reinforce => vec![
            note(220.0, 440.0, 0.0, 0.25, Wave::Square, 6.0),
            note(880.0, 880.0, 0.05, 0.35, Wave::Sine, 5.0),
        ],
        Effect::Goal => [72, 76, 79, 84]
            .iter()
            .enumerate()
            .map(|(i, &m)| note(midi(m), midi(m), i as f32 * 0.08, 0.3, Wave::Triangle, 6.0))
            .collect(),
    };
    let length = notes.iter().map(|n| n.start + n.length).fold(0.0, f32::max);
    let mut out = silence(length);
    for n in &notes {
        n.render(&mut out);
    }
    out
}

/// A chord: root note (MIDI) and whether it is minor.
type Chord = (i32, bool);

/// Four chords per world, one bar each: a different key and mood per world.
fn progression(world: usize) -> [Chord; 4] {
    match world % 3 {
        // A minor: Am F C G, the first crossing.
        0 => [(57, true), (53, false), (60, false), (55, false)],
        // D minor: Dm G C Am, the listening city.
        1 => [(50, true), (55, false), (60, false), (57, true)],
        // F major: F Dm Bb C, the common ground.
        _ => [(53, false), (50, true), (58, false), (60, false)],
    }
}

/// Seconds per beat for a world; later worlds are a little quicker.
fn beat(world: usize) -> f32 {
    60.0 / (92.0 + 8.0 * (world % 3) as f32)
}

/// Length of a world's music loop, in seconds (4 bars of 4 beats).
pub fn loop_length(world: usize) -> f32 {
    16.0 * beat(world)
}

/// One version of a world's music loop. Both versions share the bass and
/// harmony, so the game can crossfade between them on SHIFT: `second` plays
/// the same chords with another timbre, an octave higher and off the beat.
pub fn music(world: usize, second: bool) -> Vec<f32> {
    let beat = beat(world);
    let mut out = silence(loop_length(world));
    for (bar, &(root, minor)) in progression(world).iter().enumerate() {
        let bar_start = bar as f32 * 4.0 * beat;
        let tones = [0, if minor { 3 } else { 4 }, 7, 12];
        // Bass on every beat, the same in both versions.
        for b in 0..4 {
            Note {
                from: midi(root - 12),
                to: midi(root - 12),
                start: bar_start + b as f32 * beat,
                length: beat,
                wave: Wave::Sine,
                volume: 0.28,
                decay: 3.0,
            }
            .render(&mut out);
        }
        // Arpeggio in eighths: up in the first version, a broken pattern an
        // octave higher in the second.
        for step in 0..8 {
            let (tone, octave, wave, offset) = if second {
                let pattern = [3, 1, 2, 0, 3, 2, 1, 2];
                (tones[pattern[step]], 12, Wave::Triangle, beat / 4.0)
            } else {
                (tones[step % 4], 0, Wave::Sine, 0.0)
            };
            Note {
                from: midi(root + tone + octave),
                to: midi(root + tone + octave),
                start: bar_start + step as f32 * beat / 2.0 + offset,
                length: beat / 2.0,
                wave,
                volume: if second { 0.12 } else { 0.16 },
                decay: 5.0,
            }
            .render(&mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_the_data() {
        let bytes = wav(&[0.0, 0.5, -0.5]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 44 + 3 * 2);
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        assert_eq!(data_len, 6);
    }

    #[test]
    fn every_effect_is_short_audible_and_unclipped() {
        for e in Effect::ALL {
            let samples = effect(e);
            let seconds = samples.len() as f32 / RATE as f32;
            assert!(seconds > 0.05 && seconds < 1.0, "{e:?}: {seconds}s");
            let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.05 && peak <= 1.0, "{e:?}: peak {peak}");
        }
    }

    #[test]
    fn both_music_versions_loop_with_the_same_length_and_no_clipping() {
        for world in 0..3 {
            let a = music(world, false);
            let b = music(world, true);
            assert_eq!(
                a.len(),
                b.len(),
                "world {world}: versions must stay in sync"
            );
            for samples in [&a, &b] {
                let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                assert!(peak > 0.1 && peak <= 1.0, "world {world}: peak {peak}");
                // Quiet at the seam, so the loop does not click.
                assert!(samples.last().unwrap().abs() < 0.05);
            }
        }
    }
}
