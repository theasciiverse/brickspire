//! The mini-bosses. The first is the Gargoyle: a stone gargoyle come alive on the
//! balcony floor. It flaps to and fro above the bricks, spits fireballs at the paddle
//! (a hit stuns the paddle for a moment), and every few hits lays a shield of stone
//! bricks beneath itself. Those stay where they are laid as it drifts on, so the
//! field clutters up as the fight goes on. It speeds up as it is hurt.

use crate::physics::V2;
use crate::game::WORLD_W;

/// Half the Gargoyle's body (its hit box), wingtip to wingtip and horn to claw.
pub const HALF: V2 = V2::new(7.0, 3.0);
pub const SHOT_R: f32 = 1.0;
const SHOT_SPEED: f32 = 34.0;
const MAX_HP: u8 = 12;
/// Hits it takes between laying shields.
const SHIELD_EVERY: u8 = 3;
/// Seconds it can't be hurt again after a hit.
const HURT_TIME: f32 = 0.9;
/// Seconds its eyes glow before it spits.
pub const AIM_TIME: f32 = 0.6;
const SWAY: f32 = 30.0;
const HOVER_Y: f32 = 12.5;

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub p: V2,
    pub v: V2,
}

/// What a hit did.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Hurt {
    /// Still glowing from the last hit: nothing.
    Shrugged,
    Hurt,
    /// Hurt, and lays a stone shield.
    Shield,
    Dead,
}

#[derive(Clone, Debug)]
pub struct Gargoyle {
    pub p: V2,
    pub hp: u8,
    pub max: u8,
    /// Seconds of hurt flash left.
    pub hurt: f32,
    /// Seconds alive, for the wingbeat.
    pub t: f32,
    /// Where it is in its sway, separately from `t` so speeding up doesn't jump.
    phase: f32,
    /// Seconds until it next spits.
    pub reload: f32,
    since_shield: u8,
    shots_fired: u32,
}

impl Gargoyle {
    pub fn new() -> Gargoyle {
        Gargoyle {
            p: V2::new(WORLD_W / 2.0, HOVER_Y),
            hp: MAX_HP,
            max: MAX_HP,
            hurt: 0.0,
            t: 0.0,
            phase: 0.0,
            reload: 3.0,
            since_shield: 0,
            shots_fired: 0,
        }
    }

    /// 1 when fresh, nearly 2 when nearly beaten.
    fn rage(&self) -> f32 {
        1.0 + (1.0 - self.hp as f32 / self.max as f32) * 0.9
    }

    pub fn bounds(&self) -> (V2, V2) {
        (self.p - HALF, self.p + HALF)
    }

    /// Wing angle for drawing: -1 (up) to 1 (down).
    pub fn flap(&self) -> f32 {
        (self.t * 6.0 * self.rage()).sin()
    }

    /// Is it about to spit (eyes glowing)?
    pub fn aiming(&self) -> bool {
        self.reload < AIM_TIME
    }

    /// Move for `dt`; while `live`, spit at `target` when reloaded.
    pub fn update(&mut self, dt: f32, live: bool, target: V2, shots: &mut Vec<Shot>) {
        self.t += dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.phase += dt * 0.45 * self.rage();
        self.p = V2::new(WORLD_W / 2.0 + SWAY * self.phase.sin(), HOVER_Y + 1.5 * (self.t * 1.3).sin());
        if !live {
            return;
        }
        self.reload -= dt;
        if self.reload <= 0.0 {
            let mouth = self.p + V2::new(0.0, 1.0);
            shots.push(Shot { p: mouth, v: (target - mouth).norm() * SHOT_SPEED });
            self.shots_fired += 1;
            // a little uneven, so the rhythm can't be learned to the frame
            let wobble = [0.0, 0.5, -0.3, 0.8, 0.2][self.shots_fired as usize % 5];
            self.reload = (2.4 + wobble) / self.rage();
            log::info!("gargoyle spits at {:.1},{:.1}", target.x, target.y);
        }
    }

    pub fn hit(&mut self) -> Hurt {
        if self.hurt > 0.0 {
            return Hurt::Shrugged;
        }
        self.hp = self.hp.saturating_sub(1);
        self.hurt = HURT_TIME;
        if self.hp == 0 {
            return Hurt::Dead;
        }
        self.since_shield += 1;
        if self.since_shield >= SHIELD_EVERY {
            self.since_shield = 0;
            return Hurt::Shield;
        }
        Hurt::Hurt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_between_the_walls() {
        let mut g = Gargoyle::new();
        let mut shots = Vec::new();
        for _ in 0..20_000 {
            g.update(1.0 / 120.0, true, V2::new(50.0, 50.0), &mut shots);
            let (min, max) = g.bounds();
            assert!(min.x >= crate::game::FIELD_X0 && max.x <= crate::game::FIELD_X1 && min.y >= crate::game::FIELD_TOP);
        }
        assert!(shots.len() > 20);
    }

    #[test]
    fn shields_every_third_hit_and_dies() {
        let mut g = Gargoyle::new();
        let mut seen = Vec::new();
        while g.hp > 0 {
            g.hurt = 0.0;
            seen.push(g.hit());
        }
        assert_eq!(seen.iter().filter(|&&h| h == Hurt::Shield).count(), 3);
        assert_eq!(seen.last(), Some(&Hurt::Dead));
        g.hp = 5;
        g.hurt = 0.1;
        assert_eq!(g.hit(), Hurt::Shrugged);
    }
}
