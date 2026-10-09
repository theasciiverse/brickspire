//! The title: BRICKSPIRE spelled in bricks, over the map a new game opens on.
//!
//! Each letter is a 3 by 5 grid, and each lit cell of it one brick, the letters
//! in turn in the bricks' colours, with a sheen sweeping across now and then. In
//! pixel mode a brick is a bevelled square of half-block pixels, as big as the
//! screen allows; in pipe mode it is a length of pipe two cells wide, `├┤`, on
//! its colour.

use crate::game::brick_rgb;
use crate::game::Kind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

const WORD: &str = "BRICKSPIRE";

/// A letter's rows, `#` a brick.
fn letter(c: char) -> [&'static str; 5] {
    match c {
        'B' => ["##.", "#.#", "##.", "#.#", "##."],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'I' => ["###", ".#.", ".#.", ".#.", "###"],
        'C' => [".##", "#..", "#..", "#..", ".##"],
        'K' => ["#.#", "#.#", "##.", "#.#", "#.#"],
        'S' => [".##", "#..", ".#.", "..#", "##."],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        _ => ["...", "...", "...", "...", "..."],
    }
}

/// Bricks across (letters of 3 with a gap of 1) and down.
const COLS: usize = WORD.len() * 4 - 1;
const ROWS: usize = 5;

/// The brick at logo cell (`x`, `y`): which letter it belongs to, if any.
fn brick(x: usize, y: usize) -> Option<usize> {
    let (k, lx) = (x / 4, x % 4);
    let c = WORD.chars().nth(k)?;
    (lx < 3 && letter(c)[y].as_bytes()[lx] == b'#').then_some(k)
}

/// Each letter's colour: the bricks' colours in turn (no white).
fn colour(k: usize) -> [f32; 3] {
    let [r, g, b] = brick_rgb(Kind::Plain(1 + (k % 7) as u8));
    [r as f32, g as f32, b as f32]
}

fn col(c: [f32; 3], k: f32) -> Color {
    Color::Rgb((c[0] * k).clamp(0.0, 255.0) as u8, (c[1] * k).clamp(0.0, 255.0) as u8, (c[2] * k).clamp(0.0, 255.0) as u8)
}

/// How bright the sheen makes logo column `x` at `time`.
fn sheen(x: f32, time: f32) -> f32 {
    let at = (time * 0.35).fract() * 3.0 * COLS as f32 - COLS as f32;
    if (x - at).abs() < 2.0 { 1.0 + 0.5 * (1.0 - (x - at).abs() / 2.0) } else { 1.0 }
}

/// Draw the logo centred near the top of `area` (`fade` 0 to 1). Returns the row
/// below it.
pub fn draw(area: Rect, buf: &mut Buffer, glyphs: bool, time: f32, fade: f32) -> u16 {
    if glyphs {
        // a brick is two cells by one row
        let (w, h) = (COLS as u16 * 2, ROWS as u16);
        if w > area.width || h + 2 > area.height {
            return area.y;
        }
        let (x0, y0) = (area.x + (area.width - w) / 2, area.y + area.height / 10);
        for y in 0..ROWS {
            for x in 0..COLS {
                let Some(k) = brick(x, y) else { continue };
                let c = colour(k);
                let lit = sheen(x as f32, time) * fade;
                for (i, sym) in ["├", "┤"].iter().enumerate() {
                    let cell = &mut buf[(x0 + x as u16 * 2 + i as u16, y0 + y as u16)];
                    cell.set_symbol(sym).set_fg(col(c, lit * 1.15)).set_bg(col(c, lit * 0.45));
                }
            }
        }
        return y0 + h + 1;
    }
    // pixel mode: as big as fits in 85% of the width and a third of the height
    let s = ((area.width as f32 * 0.85 / COLS as f32).min(area.height as f32 * 2.0 / 3.0 / ROWS as f32).floor() as u16).max(1);
    let (w, h) = (COLS as u16 * s, ROWS as u16 * s);
    if w > area.width || h / 2 + 1 > area.height {
        return area.y;
    }
    let x0 = area.x + (area.width - w) / 2;
    let py0 = (area.height / 10) * 2;
    for py in 0..h {
        for px in 0..w {
            let (x, y) = ((px / s) as usize, (py / s) as usize);
            let Some(k) = brick(x, y) else { continue };
            let (lx, ly) = (px % s, py % s);
            // bevelled: lit top and left, shaded bottom and right
            let bevel = if s < 3 {
                1.0
            } else if lx == 0 || ly == 0 {
                1.35
            } else if lx == s - 1 || ly == s - 1 {
                0.55
            } else {
                1.0
            };
            let c = col(colour(k), bevel * sheen(x as f32, time) * fade);
            let row = py0 + py;
            let cell = &mut buf[(x0 + px, area.y + row / 2)];
            cell.set_symbol("▀");
            if row % 2 == 0 {
                cell.set_fg(c);
            } else {
                cell.set_bg(c);
            }
        }
    }
    area.y + (py0 + h) / 2 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_word_is_spelled_in_bricks() {
        // every letter has bricks, and the gaps between letters are clear
        for k in 0..WORD.len() {
            assert!((0..ROWS).any(|y| (0..3).any(|x| brick(k * 4 + x, y) == Some(k))), "letter {k} empty");
            if k + 1 < WORD.len() {
                assert!((0..ROWS).all(|y| brick(k * 4 + 3, y).is_none()));
            }
        }
        let area = Rect::new(0, 0, 100, 28);
        let mut buf = Buffer::empty(area);
        assert!(draw(area, &mut buf, false, 0.0, 1.0) > 2);
        assert!(draw(area, &mut buf, true, 0.0, 1.0) > 2);
    }
}
