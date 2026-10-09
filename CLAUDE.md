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
| Mini-boss floors (partway up) | a balcony curving round the tower | a shallow arc along the lower part of the screen |
| The roof (tower boss) | straight down onto the round tower top | a full circle round the whole arena |

The paddle always lies along the tangent of its path (decided 2026-10-09). On the straight path
that means horizontal, as in Arkanoid. On the arc and the circle it tilts with the curve, so
where you meet the ball sends it off at different angles. That change in dynamics is the point
of the boss fights: a new skill to learn, arriving just as the bosses get hard.

The mini-boss arc is a shallow curve, well short of a half circle (decided 2026-10-09). Its
exact sweep is set by playtesting so it fits the field and plays well; start around 90 degrees.

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

## Catalog neighbours

- **mtg** (github.com/modern-terminal-games/mtg, MIT) has a classic brick breaker, `breaker`,
  that David expects to be in the catalog alongside Brickspire. They are different games
  (a straight classic versus the climb and the curved paths), so Brickspire should not
  borrow mtg's look, so the two stay clearly apart side by side.

## Reference games (read 2026-10-09)

David found nine terminal brick breakers; their source was read, none was run. No code is
taken from any of them. None does curved or circular paddle paths, and none corrects for
terminal cells being taller than they are wide, so that part is ours to solve.

- **mtg `breaker`** (MIT), a physics checklist worth keeping:
  - keep each velocity component at 18% or more of the speed, so the ball never runs almost
    straight up or almost sideways (no endless wall loops);
  - pick a brick's bounce side by penetration depth, settle corner hits by the ball's
    direction, let one brick count per step, push the ball out if it ends up inside a brick;
  - a test proving the ball can't clear a whole column of bricks in one pass;
  - fixed 60 Hz step with sub-steps (we go further with a proper sweep).
- **bricktick** (Unlicense): the paddle's own movement nudges the ball on a hit; timed
  power-ups; levels as a small data table.
- **Brick-Breaker-Terminal-Based-Game v2** (Python, no licence: ideas only): a UFO boss that
  follows the paddle, drops bombs, and rebuilds a brick shield once hurt. A starting point
  for a first boss, not a design to copy.
- **arcanoid.sh** (bolknote/shellgames, no licence: ideas only): levels written as
  `x y type count` runs; bricks that weaken and change colour when hit.
- **bashciiout** (no licence: ideas only): adapts the field to the terminal size; scrolling
  star background.
- Nothing to take: arkanoid-tui (no time step, tunnels), ENGG1340-Breakout and ASCII-Breakout
  (near-copies, use-after-free on respawn), nbsdgames (no brick breaker in it).

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
- Losing the ball: below the paddle line on floors, below the arc on mini-boss floors, out of the
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

- [ ] How many towers, and a theme per tower?
- [ ] Two-player co-op or versus over iroh? Doh It Again had both.
- [ ] A level editor? Doh It Again had one.
- [ ] Offer a "paddle-centric" view on the roof (the arena turns and the paddle stays at the
      bottom, as Circanoid allows) as an option?

## Running

```bash
cargo run --release                         # play
cargo run --release -- --path arc           # preview the mini-boss arc on the same field
cargo run --release -- --path circle        # preview the roof ring (a small test block)
cargo run --release -- --floor 3            # start on floor 3
cargo run --release -- --floor 5            # the Gargoyle's balcony (first mini-boss)
cargo run --release -- --floor 6            # the Weathercock's roof (first tower boss)
cargo run --release -- --glyphs             # pipe graphics (g flips in the game)
RUST_LOG=debug cargo run --release 2>brickspire.log
cargo run --release -- --snapshot out.ppm --size 100x30 --secs 6 [--path arc] [--turn 0.5]   # headless frame
cargo test --release
```

## Code map

- `physics.rs`: `V2`, the paddle's `Path` (line, arc, circle; arc length `s`, +s is right
  at the bottom), and swept collisions (capsules, boxes as four edge capsules, planes).
- `game.rs`: the rules at a fixed 120 Hz step; `Arena` per mode, bricks on a grid,
  paddle bat (normal turned up to 60 degrees by where it hits), the 18% minimum share
  on straight floors, lives, score, particles, and the autoplay `Pilot`.
- `levels.rs`: floors as text grids, each a kind: plain (straight paddle), balcony
  (arc, Gargoyle) or roof (ring, Weathercock).
- `boss.rs`: the Gargoyle (movement, spitting, hits, when it lays a shield) and the
  Weathercock (turning, gusts, spin-ups, hits).
- `roof.rs`: the camera tilting up onto the roof, shared by both renderers (what a
  screen point sees: roof top, parapet, tower side or beyond), and the roof's contents.
- `tower.rs`: the tower's geometry, shared by both renderers: which floor's face, and
  where on it, lies under a screen point for a camera angle; the bricks of nearby floors.
- `render.rs`: pixel mode, per-pixel half-block shading of the world, scaled to any terminal.
- `glyph.rs`: glyph mode, pipe graphics (box-drawing lines, no half-blocks): bricks are
  lengths of pipe `├──┤` (steel `╞══╡`, gilt `╠══╣`), boxes when taller than a row; the
  paddle a heavy bar `╺━━╸` that tilts with its path; the ball a pipe ring at its
  true size (`()`, `╭─╮ ╰─╯`, wider on big terminals). Scaled so a brick is whole cells
  (4x1 at the smallest, a 67x19 field that fits 80x24; 8x2 next), grid aligned to cells.
- `input.rs`: keys (exact or timing), mouse chase and click, gamepad (stick aims on the roof).

## Status

Build step 4 done (2026-10-09): circle mode on the roof and the first tower boss. The
first tower is now Ground Floor, The Spire, Chequers, Twin Lanterns, the Gargoyle's
Balcony, and the Weathercock's Roof; after the roof the climb starts again (next tower:
step 5). Beating the Gargoyle doesn't turn the tower: the camera rises and tilts to look
straight down (2.6 s). The roof shows as an ellipse that opens into a circle, the tower's
side below it foreshortens away with its faces' tops following the cylinder's curve,
and the sky gives way to the ground far below. The roof is flagstones in rings, a
crenellated parapet (24 merlons), the paddle's ring (radius 25) and a ring of bricks.
The Weathercock (16 hits) stands in the middle: its arrow turns slowly and bats the ball
away (it can't be hurt), the rooster on its hub is the target; every 6 s (faster as it
is hurt) it shudders, then blows a 1.6 s gust along the arrow that bends the ball's path
in flight; every fourth hit it spins up and sweeps round fast for 2.5 s. Beating it
climbs the tower: a 5000 bonus. Particles have no gravity looking straight down.
Autoplay beats it in 60 to 120 s without losing a life, but autoplay tracks the ball
perfectly round the ring; balance is a first guess, as with the Gargoyle.

Step 3 (same day): arc mode and the Gargoyle on floor 5. The balcony is part of the
floor's face, the camera pulls back a little (pixels), the Gargoyle (12 hits) spits
fireballs that stun the paddle for 1 s, blinks 0.9 s after a hit, lays a stone shield
every third hit. David played it: the arc plays really differently, and the balance
felt right.

Step 2 (same day): the turning tower. Four faces to a turn, each 14 units higher than
the last; the front face is the flat field, walls as pilasters, a lintel, a stone ledge;
neighbours peek round the curve; clearing a floor holds 0.6 s, then turns over 1.8 s.

Step 1 (same day): flat floors playable with keys, mouse and gamepad; ordinary, steel
and gilt bricks, three lives, score, pixel and pipe graphics. Next: step 5, content
(more floors and towers, brick types, power-ups, bosses).
