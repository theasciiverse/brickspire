//! Every way to steer the paddle, merged into one `Steer` per step.
//!
//! Keyboard: two tiers, adapted from Marble Recovery's (itself from Entity's). With
//! the kitty keyboard protocol presses and releases are real; otherwise a key counts
//! as held while its OS repeats keep arriving, with the repeat timing learned from the
//! events, and when both directions look held the most recent one wins.
//! Mouse: the paddle chases the pointer; a click launches. Gamepad: stick or d-pad
//! along the path; on the roof the stick points at where round the ring to go.
//!
//! gilrs counts any device with buttons and two axes as a gamepad, so only devices
//! that matched SDL's controller database are used; `BRICKSPIRE_ANY_PAD=1` accepts all.

use crate::game::Steer;
use crate::physics::V2;
use log::debug;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

const PRE_REPEAT_CAP: Duration = Duration::from_millis(220);
const REPEAT_DELAY_DEFAULT: Duration = Duration::from_millis(500);
const REPEAT_INTERVAL_DEFAULT: Duration = Duration::from_millis(33);
const RELEASE_MIN: Duration = Duration::from_millis(60);
const RELEASE_MAX: Duration = Duration::from_millis(160);
#[cfg(feature = "gamepad")]
const STICK_DEADZONE: f32 = 0.2;

#[derive(Clone, Copy, Debug)]
struct Repeat {
    delay: Duration,
    interval: Duration,
}

#[derive(Clone, Copy, Default, Debug)]
struct Hold {
    last: Option<Instant>,
    events: u32,
}

impl Hold {
    fn event(&mut self, now: Instant, rep: &mut Repeat) {
        if let Some(last) = self.last {
            let gap = now.duration_since(last);
            let continues = if self.events == 1 {
                gap >= rep.delay.mul_f32(0.5) && gap <= rep.delay.mul_f32(1.6)
            } else {
                gap <= RELEASE_MAX.max(rep.interval * 4)
            };
            if continues {
                self.events += 1;
                let blend = |old: Duration, new: Duration| (old.mul_f32(0.5) + new.mul_f32(0.5)).max(Duration::from_millis(10));
                if self.events == 2 {
                    rep.delay = blend(rep.delay, gap.clamp(Duration::from_millis(100), Duration::from_millis(1000)));
                } else if gap < Duration::from_millis(200) {
                    rep.interval = blend(rep.interval, gap);
                }
                self.last = Some(now);
                return;
            }
        }
        self.last = Some(now);
        self.events = 1;
    }

    fn held(&self, now: Instant, rep: &Repeat) -> bool {
        let Some(last) = self.last else { return false };
        let timeout = if self.events >= 2 {
            (rep.interval * 5 / 2).clamp(RELEASE_MIN, RELEASE_MAX)
        } else {
            PRE_REPEAT_CAP.min(rep.delay + rep.interval * 2)
        };
        now.duration_since(last) <= timeout
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(not(feature = "gamepad"), allow(dead_code))]
pub enum Source {
    Keys,
    Mouse,
    Pad,
}

pub struct Input {
    exact: bool,
    /// 0 left, 1 right.
    down: [bool; 2],
    holds: [Hold; 2],
    /// Which direction was pressed last, for when both look held.
    latest: usize,
    rep: Repeat,
    pub mouse_at: (u16, u16),
    pad: (f32, f32),
    pub last_source: Source,
    pub quit: bool,
    pub restart: bool,
    pub pause: bool,
    pub skip: bool,
    /// Flip between pixel and glyph graphics.
    pub graphics: bool,
    /// Sound on or off.
    pub mute: bool,
    /// Launch, let go of a caught ball, fire the chisels. Taken by the game each step.
    pub fire: bool,
    #[cfg(feature = "gamepad")]
    gilrs: Option<gilrs::Gilrs>,
    pub pad_name: Option<String>,
}

fn dir_of(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('h') => Some(0),
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('l') => Some(1),
        _ => None,
    }
}

#[cfg(feature = "gamepad")]
fn usable_pad(p: &gilrs::Gamepad) -> bool {
    p.mapping_source() == gilrs::MappingSource::SdlMappings || std::env::var("BRICKSPIRE_ANY_PAD").is_ok_and(|v| v == "1")
}

impl Input {
    pub fn new(exact: bool) -> Input {
        #[cfg(feature = "gamepad")]
        let (gilrs, pad_name) = match gilrs::Gilrs::new() {
            Ok(g) => {
                let all: Vec<_> = g.gamepads().map(|(_, p)| (p.name().to_string(), usable_pad(&p))).collect();
                log::info!("gamepads (name, used): {all:?}");
                let name = all.into_iter().find(|(_, used)| *used).map(|(n, _)| n);
                (Some(g), name)
            }
            Err(e) => {
                log::warn!("gamepad support unavailable: {e}");
                (None, None)
            }
        };
        #[cfg(not(feature = "gamepad"))]
        let pad_name = None;
        Input {
            exact,
            down: [false; 2],
            holds: [Hold::default(); 2],
            latest: 1,
            rep: Repeat { delay: REPEAT_DELAY_DEFAULT, interval: REPEAT_INTERVAL_DEFAULT },
            mouse_at: (0, 0),
            pad: (0.0, 0.0),
            last_source: Source::Keys,
            quit: false,
            restart: false,
            pause: false,
            skip: false,
            graphics: false,
            mute: false,
            fire: false,
            #[cfg(feature = "gamepad")]
            gilrs,
            pad_name,
        }
    }

    pub fn keys_note(&self) -> &'static str {
        if self.exact { "keys: exact" } else { "keys: timing" }
    }

    pub fn on_key(&mut self, k: KeyEvent, now: Instant) {
        debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
        let down = k.kind != KeyEventKind::Release;
        let press = k.kind == KeyEventKind::Press;
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if let Some(d) = dir_of(k.code) {
            self.last_source = Source::Keys;
            if press {
                self.latest = d;
            }
            if self.exact {
                self.down[d] = down;
            } else if down {
                self.holds[d].event(now, &mut self.rep);
                // a press of the opposite direction ends that one at once
                self.holds[d ^ 1] = Hold::default();
            }
            return;
        }
        if !press {
            return;
        }
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('r') => self.restart = true,
            KeyCode::Char('p') => self.pause = true,
            KeyCode::Char('n') => self.skip = true,
            KeyCode::Char('g') => self.graphics = true,
            KeyCode::Char('m') => self.mute = true,
            KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('k') => self.fire = true,
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                debug!("mouse click at {},{}", m.column, m.row);
                self.fire = true;
                self.last_source = Source::Mouse;
            }
            MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                if (m.column, m.row) != self.mouse_at {
                    debug!("mouse at {},{}", m.column, m.row);
                    self.last_source = Source::Mouse;
                }
            }
            _ => {}
        }
        self.mouse_at = (m.column, m.row);
    }

    /// Drain gamepad events (call once per frame).
    pub fn poll_pad(&mut self) {
        #[cfg(feature = "gamepad")]
        if let Some(g) = &mut self.gilrs {
            use gilrs::{Axis, Button, EventType};
            while let Some(ev) = g.next_event() {
                let pad = g.gamepad(ev.id);
                if !usable_pad(&pad) {
                    continue;
                }
                match ev.event {
                    EventType::Connected => {
                        self.pad_name = Some(pad.name().to_string());
                        log::info!("gamepad connected: {:?}", self.pad_name);
                    }
                    EventType::ButtonPressed(b, _) => {
                        debug!("pad button {b:?}");
                        match b {
                            Button::South | Button::East | Button::RightTrigger => self.fire = true,
                            Button::Start => self.pause = true,
                            Button::Select => self.restart = true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
                let axis = |a: Axis| pad.axis_data(a).map_or(0.0, |d| d.value());
                let btn = |b: Button| pad.is_pressed(b) as i32 as f32;
                let mut x = axis(Axis::LeftStickX) + btn(Button::DPadRight) - btn(Button::DPadLeft);
                // stick up is positive; screen up is negative
                let mut y = -axis(Axis::LeftStickY) + btn(Button::DPadDown) - btn(Button::DPadUp);
                let len = (x * x + y * y).sqrt();
                if len < STICK_DEADZONE {
                    (x, y) = (0.0, 0.0);
                } else if len > 1.0 {
                    (x, y) = (x / len, y / len);
                }
                if (x, y) != self.pad {
                    debug!("pad stick {x:.2},{y:.2}");
                    self.pad = (x, y);
                    if len >= STICK_DEADZONE {
                        self.last_source = Source::Pad;
                    }
                }
            }
        }
    }

    /// What the paddle should do. `to_world` turns a terminal cell into a world point.
    pub fn steer(&self, now: Instant, to_world: impl Fn(u16, u16) -> V2) -> Steer {
        match self.last_source {
            Source::Mouse => Steer::Toward(to_world(self.mouse_at.0, self.mouse_at.1)),
            Source::Pad => Steer::Aim(V2::new(self.pad.0, self.pad.1)),
            Source::Keys => {
                let held = |d: usize| if self.exact { self.down[d] } else { self.holds[d].held(now, &self.rep) };
                Steer::Move(match (held(0), held(1)) {
                    (true, true) => if self.latest == 0 { -1.0 } else { 1.0 },
                    (true, false) => -1.0,
                    (false, true) => 1.0,
                    _ => 0.0,
                })
            }
        }
    }
}
