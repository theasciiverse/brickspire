# Brickspire

Asciiverse brick breaker. Rust + Ratatui. Original IP: Arkanoid-style play on a turning,
Nebulus-style tower. Spiritual successor to the SNES Arkanoid games (Arkanoid: Doh It Again,
Taito 1997; Arkanoid: Revenge of Doh, Taito 1987), which David holds up as the best of the genre.

## Premise

You climb a tower one floor at a time. Each floor is a classic flat brick field, played on the
face of the tower turned towards the camera. Clear the floor and the tower turns and rises a
step to bring the next one round to the front.

The paddle itself never changes shape. What changes is the path it can travel along, depending
on where you are on the tower, and the camera explains each change:

| Where | What the camera sees | Paddle path |
|---|---|---|
| Ordinary floors | the tower wall, side-on | a straight horizontal line, as in Arkanoid |
| Mini-boss floors (partway up) | a balcony curving round the tower | an arc along the lower part of the screen |
| The roof (tower boss) | straight down onto the round tower top | a full circle round the whole arena |

The paddle always lies along the tangent of its path (decided 2026-10-09). On the straight path
that means horizontal, as in Arkanoid. On the arc and the circle it tilts with the curve, so
where you meet the ball sends it off at different angles. That change in dynamics is the point
of the boss fights: a new skill to learn, arriving just as the bosses get hard.

## Prior art (checked 2026-10-09)

- **Ballarena** (3FR Team, Acorn Archimedes, 1990): the paddle can travel round the bricks in
  the middle, not just along the bottom.
- **Circanoid** (Christian Wulff-Nilsen, 2022, free on itch.io): the paddle runs round a ring
  for the whole game; offers a "paddle-centric" view where the arena turns instead.
- **Vortex** (iPod): bricks in a circular layout, played with the click wheel.
- **Bricked Up**: a brick breaker that climbs a near-endless tower; flat paddle.
- **Shatter** (Sidhe, 2009): boss fights; David owns it and calls it a different kind of game,
  built around pulling and pushing the ball.

Nothing found combines a tower climb with a paddle that goes from flat, to an arc, to a full
circle. That combination, and the camera that motivates it, is what this game adds.

## Rights

- The brick-breaker rules (paddle, ball, bricks, falling power-up capsules, bosses) are genre
  and free to use.
- Don't use Taito's names (Arkanoid, Vaus, DOH) or Nebulus's (Pogo). Our own name, paddle,
  power-ups and bosses.
- Toppler (gitlab.com/roever/toppler, GPLv3) is a remake of Nebulus (John M. Phillips,
  Hewson, 1987). We take only the *idea* of a turning tower in the middle of the screen; no
  code or art, so its licence doesn't reach us.
- Name: **Brickspire** (bricks + spire), chosen 2026-10-09. Checked that day: no game of that
  name on Steam, itch.io or crates.io. Nearest: BrickSpite (rebmiami.itch.io, a brick breaker,
  one letter off), a paid Minecraft map called Brickspire (BuiltByBit), an empty GitHub repo
  DreamCastYT/brickspire, and Brickspire Contractors LLC (Florida construction). Trademark
  registers were not searched.

## Camera and rendering

- Half-block pixels at the catalog's 100x30 recording size, so 100x60 effective pixels.
  These are roughly square, so the roof's circle comes out round.
- The tower stands in the middle of the screen, as in Nebulus, with sky and stars either side.
- The tower is a cylinder with about 4 floor faces per turn, laid out as a spiral. The front
  face is drawn flat and full width; that is the play area, with ordinary flat physics.
- The neighbouring faces are drawn round the curve: squashed by `sin` of their angle, darkened
  by `cos`, and offset up or down by one spiral step. The floor you just cleared curves away
  round the left edge, slightly lower. The next floor peeks round the right edge, slightly
  higher, so you can see what is coming.
- Changing floors is a tween of the tower's turn angle and its height.
- Mini-boss floors: the camera pulls back to show a balcony curving round the tower. The roof: the
  camera tilts to look straight down onto the round tower top.

## Physics

- One continuous float simulation in world units, shared by all three modes; rendering maps
  world units to pixels.
- The paddle is a segment on a path: a line, an arc of radius R, or a full circle.
- The bounce uses the path's normal at the contact point, plus a Doh It Again-style deflection
  from how far off-centre the ball hits. Clamp the outgoing angle so shots can't run almost
  parallel to the paddle.
- Losing the ball: below the paddle line on floors, below the arc on boss floors, out of the
  ring on the roof.
- Sweep the ball each step (no tunnelling through thin bricks at high speed).

## Structure

- Several towers, Nebulus-style. About 10 floors each, a mini-boss partway up (paddle on an arc), and a
  tower boss on each roof (paddle on a full circle). The last tower's roof boss is the final boss.
- Power-up capsules follow the classic set (wider paddle, catch, laser, slow, split ball, extra
  life, skip to the next floor) under our own names.
- Bricks: ordinary, hard (several hits), indestructible, regenerating, sliding.

## Input

- Keyboard: left and right also move you along the arc or round the circle. Fire button for
  laser and catch.
- Mouse and gamepad.
- Held keys: adapt `entity/src/input.rs` (as Marble Recovery did). With the kitty keyboard
  protocol, presses and releases are exact. Otherwise a key counts as held while its OS
  repeats keep arriving, with the repeat timing learned from the events, and when both
  directions look held, the most recent one wins.

## Logging

`log` + `env_logger`, per the Asciiverse convention: every raw input event as it arrives,
state changes (floor start and clear, ball lost, power-up caught, boss phase, pause), and a
state snapshot about every 500 ms (paddle position and path, balls, active power-ups, floor).

## Build order

1. Flat physics prototype on a fixed field: paddle, ball, bricks, lives.
2. Tower renderer and the spiral transition between floors.
3. Arc mode and the first mini-boss.
4. Circle mode and the first tower boss.
5. Content: towers, brick types, power-ups, bosses.
6. Sound and the title screen.

## Open questions

- [ ] How much of a circle is the mini-boss arc: a half circle, or less?
- [ ] How many towers, and a theme per tower?
- [ ] Two-player co-op or versus over iroh? Doh It Again had both.
- [ ] A level editor? Doh It Again had one.
- [ ] Offer a "paddle-centric" view on the roof (the arena turns and the paddle stays at the
      bottom, as Circanoid allows) as an option?

## Status

Design/ideation stage only. No code yet. Came up in conversation with Claude, 2026-10-09.
