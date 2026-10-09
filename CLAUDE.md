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

- Five towers, Nebulus-style (decided 2026-10-09; more can be added later if it feels
  short). About 10 floors each, a mini-boss partway up (paddle on an arc), and a tower
  boss on each roof (paddle on a full circle). The last tower's roof boss is the final
  boss. Roof bosses get harder in order.
- Each tower has a theme: its own look, one new brick type, and bosses that fit it
  (decided 2026-10-09; the last tower mixes every brick type):

  | # | Tower | Look | New brick | Mini-boss | Roof boss |
  |---|---|---|---|---|---|
  | 1 | The Old Keep | grey stone, night sky, fields below | steel, gilt | the Gargoyle | the Storm Cloud |
  | 2 | The Ivy Tower | mossy stone in a forest, dusk | vine bricks that slide along their row | a giant spider on its web | the Rook's Nest (a giant crow in a nest with a twig wall the ball must break; it hops round and swoops across the roof) |
  | 3 | The Bell Tower | warm sandstone, sunset | clockwork bricks that tick back in after a while | the clock's jack, the figure that strikes the bell on the hour | the Great Bell (swings in the middle; each ring sends a slow shock ring outward that stuns the paddle unless dodged) |
  | 4 | The Sea Tower | a lighthouse on a cliff, storm clouds, waves below | ice bricks the ball skids off at a shallower angle | a sea serpent rising beside the balcony | the Weathercock |
  | 5 | The Brickspire | above the clouds, starry sky | all of them | the Mason's apprentice (provisional, to confirm) | the Mason, who built the tower, rebuilding the field as you break it |

- Boss design rule (learned 2026-10-09): keep the ball readable. A boss's threat
  goes at the paddle (marks, stuns, things to dodge), not at the ball's flight. The
  Weathercock breaks this (gusts bend the ball, the arrow bats it), which is why it
  was too hard as the first roof boss and moved to tower 4.
- Power-up capsules follow the classic set under our own names (built 2026-10-09):
  Plank (P, a longer paddle), Mortar (M, catch: the ball sticks, space lets it go),
  Chisel (C, laser: space fires a pair of chisels), Hourglass (H, slow), Flock (F,
  three balls), Spare (+, extra life), Stair (S, on to the next floor; not on boss
  floors).
- Bricks: ordinary, hard (several hits), indestructible, plus one new type per tower
  (sliding vine, regenerating clockwork, ice).

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

- [x] How many towers, and a theme per tower? Five, themed (see Structure).
- [ ] Tower 5's mini-boss. Built provisionally (overnight, 2026-10-09) as the Mason's
      apprentice on a scaffold, throwing bricks that chip the paddle short; David to
      confirm or pick another.
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
cargo run --release -- --floor 10           # the Storm Cloud's roof (first tower boss)
cargo run --release -- --floor 11           # the Ivy Tower (floors 11 to 20: spider on 15, rook on 20)
cargo run --release -- --floor 21           # the Bell Tower (floors 21 to 30: the Jack on 25, the Great Bell on 30)
cargo run --release -- --floor 31           # the Sea Tower (floors 31 to 40: the serpent on 35, the Weathercock on 40)
cargo run --release -- --floor 41           # the Brickspire (floors 41 to 50: the apprentice on 45, the Mason on 50)
cargo run --release -- --floor 10 --roof weathercock   # any roof boss on any roof: storm, rook, bell, weathercock, mason
cargo run --release -- --glyphs             # pipe graphics (g flips in the game)
cargo run --release -- --mute               # no sound (m flips in the game)
RUST_LOG=debug cargo run --release 2>brickspire.log
cargo run --release -- --snapshot out.ppm --size 100x30 --secs 6 [--path arc] [--turn 0.5] [--map 4 [--leave 0.5]]   # headless frame (--map T: the map after this tower, T s in; --leave L: L s into going on)
cargo test --release
```

## Code map

- `physics.rs`: `V2`, the paddle's `Path` (line, arc, circle; arc length `s`, +s is right
  at the bottom), and swept collisions (capsules, boxes as four edge capsules, planes).
- `game.rs`: the rules at a fixed 120 Hz step; `Arena` per mode, bricks on a grid,
  paddle bat (normal turned up to 60 degrees by where it hits), the 18% minimum share
  on straight floors, lives, score, particles, and the autoplay `Pilot`.
- `levels.rs`: the towers, each a theme (sky, stone, ground, ivy), its mini-boss and
  its floors; floors as text grids, each a kind: plain (straight paddle), balcony
  (arc) or roof (ring, and which roof boss). Floors are numbered straight through
  all the towers.
- `boss.rs`: the mini-bosses, one shape of fight in three kinds (the Gargoyle spits
  fire and lays stone, the Spider spits web and spins web bricks, the Jack strikes
  volleys of chimes and winds clockwork, the sea serpent sways on its neck, spits
  freezing spray and freezes ice, the apprentice swings on his scaffold and throws
  bricks), the Storm Cloud (drifting, marking the
  ring, lightning, rain), the Rook (twig nest, hopping, crouch and swoop, patching),
  the Great Bell (swinging, tolling rings of sound loud along its swing, lurching),
  the Mason (working in his fort of bricks, putting them back, lobbing bricks)
  and the Weathercock (turning, gusts, spin-ups, hits).
- `power.rs`: the power-up capsules: kinds, names, colours, letters, how often each
  falls. Their rules (falling, catching, mortar, chisels) are in `game.rs`.
- `roof.rs`: the camera tilting up onto the roof, shared by both renderers (what a
  screen point sees: roof top, parapet, tower side or beyond), and the roof's contents.
- `map.rs`: the map between towers, shared by both renderers: the five towers on a
  dusk landscape (shapes, where they stand), which are climbed, the trail.
- `tower.rs`: the tower's geometry, shared by both renderers: which floor's face, and
  where on it, lies under a screen point for a camera angle; the bricks of nearby floors.
- `render.rs`: pixel mode, per-pixel half-block shading of the world, scaled to any terminal.
- `glyph.rs`: glyph mode, pipe graphics (box-drawing lines, no half-blocks): bricks are
  lengths of pipe `├──┤` (steel `╞══╡`, gilt `╠══╣`), boxes when taller than a row; the
  paddle a heavy bar `╺━━╸` that tilts with its path; the ball a pipe ring at its
  true size (`()`, `╭─╮ ╰─╯`, wider on big terminals). Scaled so a brick is whole cells
  (4x1 at the smallest, a 67x19 field that fits 80x24; 8x2 next), grid aligned to cells.
- `audio.rs`: every sound synthesised at start-up (rodio, no audio files), after
  SIGPIPE's `audio.rs`; the rules ask for `game::Sound`s each step.
- `title.rs`: the BRICKSPIRE logo, spelled in bricks (3 by 5 letters), drawn over
  the map a new game opens on.
- `input.rs`: keys (exact or timing), mouse chase and click, gamepad (stick aims on the roof).

## Status

Title (2026-10-10, overnight): a new game opens on the map with BRICKSPIRE spelled
in bricks across the top, each letter in a brick colour with a sheen sweeping
across (bevelled half-block squares as big as fit; `├┤` pipes on their colour in
pipe mode), "AN ASCIIVERSE BRICK BREAKER · FIVE TOWERS TO CLIMB" under it, then
"FIRST: THE OLD KEEP · space to climb". Step 6 still lacks music and a saved best
score.

Sound (2026-10-09, overnight; step 6 begun): synthesised effects, no files. The
paddle's wooden tock, a tick off the walls, each brick broken in a row since the
ball left the paddle a note higher up a pentatonic scale (a run climbs), a metal
ping for a hit that didn't break (steel, ice), a higher bell off gilt; bosses
thump when hit and crumble when beaten, spit with a short chirp; the paddle
buzzes when stunned, webbed, iced or chipped; thunder for the Storm Cloud's bolt,
a deep bell for the Great Bell's toll, rushing wings for the Rook's swoop, a
whistle for the Mason's lob, tick-tock as clockwork winds back in; jingles for a
power-up, a cleared floor, a lost ball, a climbed tower, game over; a rush going
on from the map. No music yet. Not yet heard on a real device overnight (only
checked that every sound renders in range): needs a listen.

Ten floors a tower (2026-10-09, overnight): four floors, the mini-boss's balcony,
four more, the roof; 50 floors in all. Beating a mini-boss now turns the tower to
the next floor like any other (the balcony curves away round the left edge); only
the floor under the roof tilts up onto it. Steel takes 2 hits, one more every 12
floors, at most 5. A test clears every plain floor with the autopilot (between
about 40 s and 3 minutes each).

Power-ups (2026-10-09, overnight): a broken brick lets a capsule fall one time in
seven (one at a time, and not while several balls are in play); on the roof it
drifts outward to the ring instead. A pill in its colour with its letter (the
letter sits in the cell at its middle in pixel mode, `(M)` in pipes); catching one
says what it does for 2 s, and the lasting one shows in the HUD. Plank, Mortar
and Chisel last until another of them is caught or a life is lost; the paddle
shows them (mortar along its top, red chisel ends). Mortar holds a ball where it
landed (it rides with the paddle) until space, or 3 s; Chisel fires two chisels
up from the paddle's ends (along its normal, so inward on the roof), every 0.3 s
at most, each breaking the first brick it meets; a boss stops them unhurt (only
the ball hurts bosses). Flock turns every ball into three.

The Brickspire (2026-10-09, overnight): above the clouds, a full night sky, violet
stone, a sea of cloud far below the roof. Its floors mix every brick so far (vine,
clockwork, ice, steel, gilt). The Apprentice's Scaffold (provisional mini-boss,
see open questions): the Mason's apprentice (12 hits) stands on a plank slung from
the lintel on two ropes, a hod of bricks beside him, swinging to and fro; he
raises a brick and throws it at the paddle, which chips it (60% of its length for
3 s); every third hit he lays a row of red bricks. The Mason's Roof, the final
boss: the Mason (20 hits, a straw hat over blue shoulders seen from above) works
inside a little fort of bricks in the middle of the roof, putting back a brick
where his roof had one every 3 s (sooner when hurt, three at once every fourth
hit). He lobs bricks at the paddle: the ring is marked where one will land 1.3 s
later, and a paddle still there is stunned; below half health he lobs two at a
time, the second wide to one side. Beating him is worth 10000. The autopilot
takes about 155 s and loses two lives, the hardest roof, as it should be.

With all five towers built, the map's "every tower climbed" after the last roof is
now true, and the climb starts again from the Old Keep.

The Sea Tower (2026-10-09, overnight): a lighthouse in a storm, grey-white stone,
storm clouds rolling and rain streaking the sky. Ice bricks (pale, glassy, a glint
across): two hits, cracked after the first, and the ball skids off them, keeping
only 55% of its speed away from the ice, so it leaves at a shallower angle. The
Serpent's Balcony: the sea serpent (12 hits), its scaled neck rising over the
lintel from the sea below, its head (red fins, yellow eyes) swaying over the
bricks; it opens its jaws and spits freezing spray, which ices the paddle over for
2.5 s: it slides, slow to get going and slow to stop. Every third hit it freezes a
row of ice. Its roof has the Weathercock. Roof times for the autopilot: the Storm
Cloud about 70 s, the Rook about 105, the Great Bell (16 hits, tolling every 4.5 s)
and the Weathercock about 70.

The Bell Tower (2026-10-09, overnight): warm sandstone at sunset. Clockwork bricks
(brass, a cog turning in each): broken, one leaves a dotted outline that fills
from the left as it winds back in over 9 s (it waits if a ball is on its cell); a
floor is clear once every brick is down at the same moment, so the clockwork has
to go together at the end. The Jack's Balcony: the Jack (12 hits), an iron
striker in a helmet, marches to and fro in ticks like a clock's hand; on the hour
it raises its hammer and strikes the little bell at its side, a chime flying at
the paddle (stuns), one at first and volleys of up to three as it is hurt; every
third hit it winds a row of clockwork. The Great Bell's Roof: the bell (14 hits)
hangs in the middle, seen from above (bronze rings, the oak headstock across),
swinging along an axis that slowly turns. It shivers, then tolls: a ring of sound
spreads out, loud only in two arcs along its swing (front and back), and stuns
the paddle where a loud arc crosses it; the paddle's ring is marked with pink
dashes where the arcs will land, so the paddle steps into the quiet between them.
Every third hit it lurches a quarter turn and tolls twice. The autopilot beats
the Jack and the Bell in under a minute each.

Build step 5 started (2026-10-09): towers. The game now has two of the five
towers, numbered straight through (`--floor 7` is the Ivy Tower's ground floor);
after the last tower the climb starts again. Each tower has its own theme: the Old
Keep's night sky and blue-grey stone, the Ivy Tower's dusk sky, mossy stone with ivy
over the walls (thickest hanging from the lintels) and forest below. Only the
current tower's floors show on the tower: below its first floor is plinth.

Between towers, the map (test version, same day, David's idea): after the roof's
3.5 s hold, a wide dusk landscape with all five towers side on, each its own shape
(the Old Keep with battlements, the Ivy Tower, the Bell Tower with a belfry and
bell, the Sea Tower as a striped lighthouse with its lamp, the Brickspire's needle).
Climbed towers fly a flag and have lit windows, the next stands lit with its name
(pulsing gently), the rest are dark silhouettes named `?`. A dotted trail draws
itself from the tower just climbed to the next over 2.2 s, a walker at its head;
then space goes on (or after 10 s). Since overnight the camera moves with it: the
map opens close on the climbed tower's top (its new flag) and pulls back over
1.6 s to the whole landscape (the names show once it has settled); going on, it
swoops in on the next tower and fades (1.1 s). A new game opens on the map too:
nothing climbed, the Old Keep lit with the climber waiting at its foot, "THE CLIMB
BEGINS", then "FIRST: THE OLD KEEP". (`--snapshot ... --map T [--leave L]`.)

The Ivy Tower: four floors with vine bricks (green, leafy, a stem along the middle):
every 1.4 s each vine steps one cell along its row, gliding over 0.35 s, and turns
back at a brick, the edge or a ball (alternate rows set off opposite ways). Its
balcony has the Spider (12 hits): it hangs on its thread from the lintel, creeping
to and fro and rising and dropping; it spits web (slower than fire) that gums up
the paddle (40% speed for 2.5 s), and every third hit spins a row of web bricks. On
its roof the Rook (14 hits) sits in a nest of 14 twigs the ball must break through;
it hops about the nest, then crouches (1.1 s: a dotted line and a mark on the ring
where the paddle is), swoops out there and back (1.6 s), and stuns the paddle if it
is still there; every third hit it puts back up to four twigs. The autopilot beats
the Spider and the Rook in about 90 s each (the first tower's in about a minute).
The Spider's balcony has steel in its top corners, not gilt: the autopilot got the
ball stuck between a gilt corner and the end of the arc.

Build step 4 done (2026-10-09): circle mode on the roof and the first tower boss. The
first tower is Ground Floor, The Spire, Chequers, Twin Lanterns, the Gargoyle's
Balcony, and the Storm Cloud's Roof; after the roof the climb starts again (next tower:
step 5). Beating the Gargoyle doesn't turn the tower: the camera rises and tilts to look
straight down (2.6 s); the roof opens from an ellipse into a circle, the tower's side
foreshortens away with its faces' tops following the cylinder's curve, the sky gives way
to the ground far below.

The roof, reworked the same day after David played it ("too hard: fewer bricks, a boss
that moves, the circle needn't be the only play area"): the roof is bigger (radius 33), and on the
roof the camera frames just the roof rather than the 100x56 world, so it fills the
screen's height whatever the terminal's shape (David's cells are taller than 1:2, which
had left it small); the framing eases across during the climb. The paddle's ring (radius 26) sits
inside the parapet with open roof between: a ball that gets past the paddle bounces off
the 16 merlons (65% of the rim) back into play and is lost only through the gaps. On the
roof the paddle is longer (18) and faster (188, the same turning speed round the bigger ring) and the ball slower (40 to 66), since
the paddle may have to go half way round while the ball crosses (a bigger ring alone
wouldn't help: both distances scale together). Twelve bricks in small groups.

The roof boss is the Storm Cloud (since the same day: David found the Weathercock a
hard fight, and it moves to a later tower). It drifts over the middle (12 hits on its
glowing core); the ball always flies straight under it. Its threat is aimed at the
paddle: every 4.5 s (sooner when hurt) it marks the stretch of ring where the paddle
is, pulsing for 1.2 s, then lightning strikes there and stuns the paddle for 1 s if
any of it is still on the mark. Every third hit it rains, and up to three cyan bricks
grow back round it (never over its core or a ball). The autopilot, which never dodges
on purpose, beats it in about a minute without losing a ball.

The Weathercock (16 hits, `--roof weathercock`) roams the middle in a slow figure (faster when hurt, still while
it blows); its arrow turns and bats the ball (can't be hurt), the rooster is the target;
it shudders, then gusts along the arrow, bending the ball's path; every fourth hit it
spins up. Beating it climbs the tower: a 5000 bonus.

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
