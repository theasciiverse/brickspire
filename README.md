# Brickspire

**THE STARS ARE GOING OUT.**

Every night the Lamplighter lights the stars, but the Mason has walled the
Lamplighter up at the top of his tower, and one by one the stars are going
out. Smash your way up to free them, floor by floor: clear one and the tower
turns to bring the next round to face you. Five towers, the Old Keep, the Ivy
Tower, the Bell Tower, the Sea Tower and the Brickspire itself, each with its
own trick brick, a monster on its balcony and a boss on its roof. And as you
climb, your paddle's path changes: straight on the floors, an arc on the
balconies, a full circle round the roofs. Hurry, before the last star fades!

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install brickspire
av play brickspire
```

To build it yourself: `cargo run --release`.

## Keys

| Key | |
|---|---|
| Left / Right, A / D, H / L, mouse | move the paddle |
| Space, Enter, Up | launch, fire Chisels, let a caught ball go |
| P / R / Q, Esc | pause / restart / quit |
| G | pixel art or pipe graphics |
| M | sound on or off |

Gamepad: stick or d-pad moves (the stick aims round the roof), bottom or
right button or right trigger fires, Start pauses.

## Options

`--floor N` starts on floor N (1 to 50), `--glyphs` in pipe graphics,
`--mute` without sound.

Session log for bug reports: `RUST_LOG=debug brickspire 2>brickspire.log`

## Credits

Sound is synthesised in the game.

## Licence

MIT; see [LICENSE](LICENSE).
