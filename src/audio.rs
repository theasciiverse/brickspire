//! All sound is synthesised here; Brickspire ships no audio files (the approach
//! Panzer and SIGPIPE use, `games/sigpipe/src/audio.rs`).
//!
//! - Effects are rendered once at start-up into sample buffers.
//! - Each brick broken in a row (since the ball last left the paddle) plays the next
//!   note up a pentatonic scale, so a run through the bricks climbs.
//! - The rules ask for sounds as `game::Sound`s each step; `play` turns them into
//!   effects, skipping an effect that played a moment ago.
//!
//! Without an audio device, or with `--mute`, every call does nothing.

use crate::game::Sound;
use rodio::buffer::SamplesBuffer;
use rodio::MixerDeviceSink;
use std::f32::consts::TAU;
use std::num::NonZero;
use std::time::{Duration, Instant};

const SR: u32 = 44_100;
const SRF: f32 = SR as f32;

/// Effects, in the bank's order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sfx {
    Launch,
    Paddle,
    Wall,
    Clink,
    Gilt,
    BossHit,
    BossDown,
    Spit,
    Struck,
    Thunder,
    Toll,
    Swoop,
    Lob,
    Tick,
    Power,
    Chisel,
    Lost,
    Clear,
    Climbed,
    GameOver,
    Go,
}

const ALL_SFX: [Sfx; 21] = [
    Sfx::Launch,
    Sfx::Paddle,
    Sfx::Wall,
    Sfx::Clink,
    Sfx::Gilt,
    Sfx::BossHit,
    Sfx::BossDown,
    Sfx::Spit,
    Sfx::Struck,
    Sfx::Thunder,
    Sfx::Toll,
    Sfx::Swoop,
    Sfx::Lob,
    Sfx::Tick,
    Sfx::Power,
    Sfx::Chisel,
    Sfx::Lost,
    Sfx::Clear,
    Sfx::Climbed,
    Sfx::GameOver,
    Sfx::Go,
];

/// Notes for bricks broken in a row: a pentatonic scale over three octaves.
const SCALE: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
const NOTES: usize = 15;

pub struct Audio {
    // the sink must outlive everything playing on its mixer
    sink: Option<MixerDeviceSink>,
    bank: Vec<SamplesBuffer>,
    notes: Vec<SamplesBuffer>,
    pub muted: bool,
    last: [Option<Instant>; ALL_SFX.len()],
}

impl Audio {
    pub fn new(muted: bool) -> Audio {
        let sink = if muted {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        let (bank, notes) = if sink.is_some() {
            (ALL_SFX.iter().map(|&s| buffer(render_sfx(s))).collect(), (0..NOTES).map(|i| buffer(render_note(i))).collect())
        } else {
            (Vec::new(), Vec::new())
        };
        Audio { sink, bank, notes, muted, last: [None; ALL_SFX.len()] }
    }

    /// Play what the rules asked for.
    pub fn play(&mut self, sound: Sound) {
        if self.muted || self.sink.is_none() {
            return;
        }
        let sfx = match sound {
            Sound::Break(n) => {
                let i = (n.max(1) as usize - 1).min(NOTES - 1);
                if let Some(sink) = &self.sink {
                    sink.mixer().add(self.notes[i].clone());
                }
                return;
            }
            Sound::Launch => Sfx::Launch,
            Sound::Paddle => Sfx::Paddle,
            Sound::Wall => Sfx::Wall,
            Sound::Clink => Sfx::Clink,
            Sound::Gilt => Sfx::Gilt,
            Sound::BossHit => Sfx::BossHit,
            Sound::BossDown => Sfx::BossDown,
            Sound::Spit => Sfx::Spit,
            Sound::Struck => Sfx::Struck,
            Sound::Thunder => Sfx::Thunder,
            Sound::Toll => Sfx::Toll,
            Sound::Swoop => Sfx::Swoop,
            Sound::Lob => Sfx::Lob,
            Sound::Tick => Sfx::Tick,
            Sound::Power => Sfx::Power,
            Sound::Chisel => Sfx::Chisel,
            Sound::Lost => Sfx::Lost,
            Sound::Clear => Sfx::Clear,
            Sound::Climbed => Sfx::Climbed,
            Sound::GameOver => Sfx::GameOver,
            Sound::Go => Sfx::Go,
        };
        let i = ALL_SFX.iter().position(|&s| s == sfx).unwrap_or(0);
        let now = Instant::now();
        if self.last[i].is_some_and(|t| now.duration_since(t) < Duration::from_millis(35)) {
            return;
        }
        self.last[i] = Some(now);
        if let Some(sink) = &self.sink {
            sink.mixer().add(self.bank[i].clone());
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }
}

fn buffer(samples: Vec<f32>) -> SamplesBuffer {
    SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(SR).unwrap(), samples)
}

// ------------------------------------------------------------------ helpers

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn square(ph: f32) -> f32 {
    if ph - ph.floor() < 0.5 { 1.0 } else { -1.0 }
}

fn tri(ph: f32) -> f32 {
    let p = ph - ph.floor();
    4.0 * (p - 0.5).abs() - 1.0
}

/// `dur` seconds of samples from `f(t, noise)`.
fn synth(dur: f32, mut f: impl FnMut(f32, &mut Noise) -> f32) -> Vec<f32> {
    let mut n = Noise(0x1234_5678);
    (0..(dur * SRF) as usize).map(|i| f(i as f32 / SRF, &mut n)).collect()
}

/// A little tune: (midi note, seconds), played on `wave` with a soft attack.
fn tune(notes: &[(f32, f32)], gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for &(n, d) in notes {
        let len = (d * SRF) as usize;
        for i in 0..len {
            let t = i as f32 / SRF;
            if n > 0.0 {
                ph += midi(n) / SRF;
            }
            let env = (t / 0.008).min(1.0) * ((d - t) / 0.03).clamp(0.0, 1.0);
            let x = if n > 0.0 { wave(ph) * env } else { 0.0 };
            lp += (x - lp) * 0.3;
            out.push(lp * gain);
        }
    }
    out
}

/// Several sounds played together.
fn chord(parts: &[Vec<f32>]) -> Vec<f32> {
    let len = parts.iter().map(Vec::len).max().unwrap_or(0);
    (0..len).map(|i| parts.iter().map(|p| p.get(i).copied().unwrap_or(0.0)).sum()).collect()
}

/// Noise through a low-pass whose cutoff sweeps from `from` to `to` Hz.
fn rumble(dur: f32, from: f32, to: f32, gain: f32, seed: u32) -> Vec<f32> {
    let mut n = Noise(seed);
    let mut lp = 0.0f32;
    let mut lp2 = 0.0f32;
    (0..(dur * SRF) as usize)
        .map(|i| {
            let t = i as f32 / SRF;
            let k = t / dur;
            let c = (TAU * (from + (to - from) * k) / SRF).min(0.9);
            lp += (n.next() - lp) * c;
            lp2 += (lp - lp2) * c;
            lp2 * (t / 0.01).min(1.0) * (1.0 - k).powf(1.5) * gain
        })
        .collect()
}

/// A bell: inharmonic partials over `f`, each dying away at its own rate.
fn bell(f: f32, dur: f32, gain: f32) -> Vec<f32> {
    const PARTIALS: [(f32, f32, f32); 6] = [(0.5, 0.5, 1.2), (1.0, 1.0, 1.6), (1.19, 0.6, 2.2), (1.5, 0.45, 2.6), (2.0, 0.4, 3.0), (2.74, 0.25, 4.0)];
    synth(dur, |t, _| {
        let strike = (t / 0.003).min(1.0);
        PARTIALS.iter().map(|&(m, a, d)| (TAU * f * m * t).sin() * a * (-t * d).exp()).sum::<f32>() * strike * gain
    })
}

/// A pitch gliding from `from` to `to` Hz on `wave`, fading out.
fn glide(dur: f32, from: f32, to: f32, gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut ph = 0.0f32;
    synth(dur, |t, _| {
        let k = t / dur;
        ph += (from + (to - from) * k) / SRF;
        wave(ph) * (t / 0.004).min(1.0) * (1.0 - k) * gain
    })
}

fn sine(ph: f32) -> f32 {
    (ph * TAU).sin()
}

fn render_note(i: usize) -> Vec<f32> {
    let n = 72.0 + SCALE[i % 5] + 12.0 * (i / 5) as f32;
    let f = midi(n);
    synth(0.18, |t, _| {
        let env = (t / 0.003).min(1.0) * (-t * 20.0).exp();
        ((TAU * f * t).sin() * 0.6 + tri(f * 2.0 * t) * 0.15 + (TAU * 3.0 * f * t).sin() * 0.08) * env * 0.22
    })
}

fn render_sfx(s: Sfx) -> Vec<f32> {
    match s {
        // the ball springing off: a quick rising boing
        Sfx::Launch => glide(0.14, 280.0, 640.0, 0.22, sine),
        // a warm wooden tock
        Sfx::Paddle => synth(0.09, |t, n| {
            let env = (-t * 55.0).exp();
            ((TAU * 210.0 * t).sin() * 0.8 + (TAU * 420.0 * t).sin() * 0.2 + n.next() * 0.15 * (-t * 300.0).exp()) * env * 0.3
        }),
        Sfx::Wall => synth(0.035, |t, _| (TAU * 1250.0 * t).sin() * (-t * 160.0).exp() * 0.12),
        // a hit that didn't break it: a metal ping
        Sfx::Clink => synth(0.2, |t, _| {
            let env = (t / 0.002).min(1.0) * (-t * 22.0).exp();
            ((TAU * 1830.0 * t).sin() * 0.5 + (TAU * 2655.0 * t).sin() * 0.35 + (TAU * 4120.0 * t).sin() * 0.15) * env * 0.2
        }),
        Sfx::Gilt => bell(1560.0, 0.4, 0.09),
        // a crunchy thump
        Sfx::BossHit => chord(&[rumble(0.2, 2500.0, 300.0, 0.9, 5), glide(0.2, 160.0, 70.0, 0.3, sine)]),
        // the boss falls apart: a long crumbling rumble under a falling cry
        Sfx::BossDown => chord(&[rumble(1.4, 1800.0, 80.0, 1.1, 9), glide(0.9, 700.0, 90.0, 0.12, square)]),
        Sfx::Spit => glide(0.11, 900.0, 300.0, 0.1, tri),
        // a buzzing zap
        Sfx::Struck => synth(0.32, |t, n| {
            let k = t / 0.32;
            (square(85.0 * t) * 0.5 + n.next() * 0.3) * (t / 0.005).min(1.0) * (1.0 - k) * 0.16
        }),
        // a crack, then the rumble rolling away
        Sfx::Thunder => {
            let crack = synth(0.12, |t, n| n.next() * (-t * 30.0).exp() * 0.45);
            chord(&[crack, rumble(1.6, 600.0, 60.0, 1.6, 21)])
        }
        Sfx::Toll => bell(196.0, 2.2, 0.16),
        // wings rushing past: noise swelling and passing
        Sfx::Swoop => {
            let dur = 0.8;
            let mut n = Noise(77);
            let (mut lp, mut bp) = (0.0f32, 0.0f32);
            (0..(dur * SRF) as usize)
                .map(|i| {
                    let t = i as f32 / SRF;
                    let k = t / dur;
                    let f = 400.0 + 1400.0 * (k * std::f32::consts::PI).sin();
                    let c = (TAU * f / SRF).min(0.9);
                    lp += c * bp;
                    let hp = n.next() - lp - 0.4 * bp;
                    bp += c * hp;
                    bp * (k * std::f32::consts::PI).sin() * 0.35
                })
                .collect()
        }
        Sfx::Lob => glide(0.3, 500.0, 1100.0, 0.06, sine),
        // tick, tock
        Sfx::Tick => {
            let tick = |f: f32| synth(0.025, move |t, _| (TAU * f * t).sin() * (-t * 250.0).exp() * 0.25);
            let mut v = tick(3100.0);
            v.extend(vec![0.0; (0.09 * SRF) as usize]);
            v.extend(tick(2300.0));
            v
        }
        Sfx::Power => tune(&[(72.0, 0.06), (76.0, 0.06), (79.0, 0.06), (84.0, 0.14)], 0.18, square),
        Sfx::Chisel => glide(0.08, 2100.0, 700.0, 0.12, square),
        Sfx::Lost => tune(&[(67.0, 0.16), (63.0, 0.16), (60.0, 0.16), (55.0, 0.45)], 0.18, tri),
        Sfx::Clear => tune(&[(72.0, 0.08), (76.0, 0.08), (79.0, 0.08), (84.0, 0.3)], 0.18, tri),
        Sfx::Climbed => chord(&[
            tune(&[(72.0, 0.12), (72.0, 0.12), (79.0, 0.12), (84.0, 0.6)], 0.15, square),
            tune(&[(0.0, 0.36), (76.0, 0.6)], 0.1, tri),
            tune(&[(0.0, 0.36), (67.0, 0.6)], 0.1, tri),
        ]),
        Sfx::GameOver => tune(&[(64.0, 0.3), (62.0, 0.3), (60.0, 0.3), (59.0, 0.3), (57.0, 0.9)], 0.17, tri),
        Sfx::Go => rumble(0.6, 300.0, 3000.0, 0.5, 33),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_render_and_stay_in_range() {
        for s in ALL_SFX {
            let v = render_sfx(s);
            assert!(!v.is_empty(), "{s:?} is empty");
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(v.iter().all(|x| x.is_finite()), "{s:?} not finite");
            assert!(peak > 0.01 && peak <= 1.0, "{s:?} peak {peak}");
        }
        for i in 0..NOTES {
            let peak = render_note(i).iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.05 && peak <= 1.0, "note {i} peak {peak}");
        }
    }
}
