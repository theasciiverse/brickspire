//! Brickspire: an Arkanoid-style brick breaker climbing a turning tower.
//!
//! Build-order step 1: flat physics on a fixed field (paddle, ball, bricks, lives).
//! The paddle already runs on a general path, so `--path arc` and `--path circle`
//! preview the mini-boss balcony and the roof ring on the same field.
//!
//! Run: `cargo run --release [-- --path line|arc|circle] [--floor N] [--glyphs]`
//! (`RUST_LOG=debug ... 2>brickspire.log` for a session log). Snapshot without a
//! terminal: `cargo run --release -- --snapshot out.ppm [--size 100x30] [--secs t]
//! [--path P] [--floor N] [--glyphs] [--turn F]` autoplays for `t` seconds and renders one frame as an image.

mod boss;
mod game;
mod glyph;
mod input;
mod levels;
mod map;
mod physics;
mod render;
mod roof;
mod tower;

use game::{Game, Mode, Pilot, State, STEP};
use input::Input;
use levels::RoofBoss;
use log::{debug, info};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Widget};
use render::View;
use std::io;
use std::time::{Duration, Instant};

const FRAME: Duration = Duration::from_millis(16);
const AMBER: Color = Color::Rgb(255, 176, 0);

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// `--path` forces one kind of paddle path on every floor; otherwise each floor has its own.
fn mode_arg() -> Option<Mode> {
    match arg("--path").as_deref() {
        Some("arc") => Some(Mode::Arc),
        Some("circle") => Some(Mode::Circle),
        Some("line") => Some(Mode::Line),
        _ => None,
    }
}

/// A new game on `--floor N` (1-based) if given; `--roof weathercock` puts the
/// Weathercock on the roof instead of the Storm Cloud.
fn new_game() -> Game {
    let mut g = Game::new(mode_arg());
    g.roof_boss = match arg("--roof").as_deref() {
        Some("weathercock") => Some(RoofBoss::Weathercock),
        Some("storm") => Some(RoofBoss::Storm),
        Some("rook") => Some(RoofBoss::Rook),
        Some("bell") => Some(RoofBoss::Bell),
        Some("mason") => Some(RoofBoss::Mason),
        _ => None,
    };
    if let Some(n) = arg("--floor").and_then(|v| v.parse::<usize>().ok()).filter(|&n| n > 1) {
        g.load_floor(n - 1);
    }
    g
}

/// `--glyphs` for character graphics; pixels (half-blocks) otherwise.
fn glyphs_arg() -> bool {
    std::env::args().any(|a| a == "--glyphs")
}

fn make_view(game: &Game, area: Rect, glyphs: bool) -> View {
    if glyphs { glyph::view(area, game.origin, game.frame()) } else { View::new(area, game.frame()) }
}

fn draw_world(game: &Game, view: &View, glyphs: bool, buf: &mut Buffer) {
    if glyphs { glyph::draw(game, view, buf) } else { render::draw(game, view, buf) }
}

fn main() -> io::Result<()> {
    env_logger::init();
    if std::env::args().any(|a| a == "--snapshot") {
        return snapshot(&arg("--snapshot").unwrap_or_else(|| "snapshot.ppm".into()));
    }
    run()
}

/// The world sits between the HUD row and the help row.
fn world_area(size: Rect) -> Rect {
    if size.height > 4 { Rect { y: size.y + 1, height: size.height - 2, ..size } } else { size }
}

fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    execute!(io::stdout(), EnableMouseCapture)?;
    info!("start: kitty keyboard = {kitty}, path {:?}", mode_arg());

    let mut game = new_game();
    let mut input = Input::new(kitty);
    let mut acc = 0.0f32;
    let mut last = Instant::now();
    let mut last_log = Instant::now();
    let mut last_state = game.state;
    let mut paused = false;
    let mut best = 0u32;
    let mut glyphs = glyphs_arg();

    let result = loop {
        let deadline = last + FRAME;
        while event::poll(deadline.saturating_duration_since(Instant::now()))? {
            match event::read()? {
                Event::Key(k) => input.on_key(k, Instant::now()),
                Event::Mouse(m) => input.on_mouse(m),
                _ => {}
            }
        }
        input.poll_pad();
        if input.quit {
            break Ok(());
        }
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32().min(0.1);
        last = now;

        if std::mem::take(&mut input.restart) {
            info!("restart");
            game = new_game();
            paused = false;
        }
        if std::mem::take(&mut input.graphics) {
            glyphs = !glyphs;
            info!("graphics: {}", if glyphs { "glyphs" } else { "pixels" });
        }
        if std::mem::take(&mut input.skip) {
            game.skip_floor();
        }
        if std::mem::take(&mut input.pause) && game.state != State::Over {
            paused = !paused;
            info!("{}", if paused { "paused" } else { "resumed" });
        }
        if game.state == State::Over && std::mem::take(&mut input.fire) {
            info!("new game");
            game = new_game();
        }

        let view = make_view(&game, world_area(terminal.size()?.into()), glyphs);
        let steer = input.steer(now, |c, r| view.cell_to_world(c, r));
        // no time builds up while paused, or play would resume with a burst of steps
        acc = if paused { 0.0 } else { acc + dt };
        while acc >= STEP {
            let fire = std::mem::take(&mut input.fire);
            game.step(steer, fire);
            acc -= STEP;
        }
        if paused {
            input.fire = false;
        }
        if std::mem::discriminant(&game.state) != std::mem::discriminant(&last_state) {
            info!("state {last_state:?} -> {:?}", game.state);
            last_state = game.state;
        }
        best = best.max(game.score);
        if now.duration_since(last_log) >= Duration::from_millis(500) {
            last_log = now;
            debug!("snapshot: {} steer {steer:?} via {:?}", summary(&game), input.last_source);
        }

        terminal.draw(|f| {
            let area = f.area();
            draw_world(&game, &view, glyphs, f.buffer_mut());
            hud(&game, &input, best, paused, area, f.buffer_mut());
        })?;
    };

    let _ = execute!(io::stdout(), DisableMouseCapture);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn summary(game: &Game) -> String {
    let balls: Vec<String> = game.balls.iter().map(|b| format!("({:.1},{:.1} v {:.1},{:.1})", b.p.x, b.p.y, b.v.x, b.v.y)).collect();
    format!(
        "floor {} {:?} state {:?} paddle s {:.1} ({:?}){} balls [{}] speed {:.1} bricks {} lives {} score {}{}",
        game.floor + 1,
        game.floor_name,
        game.state,
        game.paddle.s,
        game.arena.mode,
        if game.stun > 0.0 { " stunned" } else { "" },
        balls.join(" "),
        game.speed,
        game.left(),
        game.lives,
        game.score,
        game.storm.as_ref().map_or(String::new(), |s| format!(
            " storm at {:.1},{:.1} hp {}/{}{}",
            s.centre.x,
            s.centre.y,
            s.hp,
            s.max,
            s.charging.map_or(String::new(), |(a, t)| format!(" marking {:.0} deg ({t:.1} s)", a.to_degrees()))
        )) + &game.vane.as_ref().map_or(String::new(), |w| format!(
            " weathercock at {:.0} deg hp {}/{}{}{}",
            w.angle.to_degrees(),
            w.hp,
            w.max,
            if w.gust > 0.0 { " gusting" } else { "" },
            if w.spinning > 0.0 { " spinning" } else { "" }
        )) + &game.rook.as_ref().map_or(String::new(), |r| format!(
            " rook at {:.1},{:.1} hp {}/{} twigs {} {:?}",
            r.p.x,
            r.p.y,
            r.hp,
            r.max,
            r.twigs.iter().filter(|&&t| t).count(),
            r.flight
        )) + &game.bell.as_ref().map_or(String::new(), |b| format!(
            " bell at {:.1},{:.1} hp {}/{} waves {}{}",
            b.p.x,
            b.p.y,
            b.hp,
            b.max,
            b.waves.len(),
            if b.tolling.is_some() { " tolling" } else { "" }
        )) + &format!(" ghosts {}", game.ghosts.len()) + &game.webbed.gt(&0.0).then_some(" webbed").unwrap_or("") + game.iced.gt(&0.0).then_some(" iced").unwrap_or("") + game.chipped.gt(&0.0).then_some(" chipped").unwrap_or("") + &game.mason.as_ref().map_or(String::new(), |m| format!(" mason at {:.1},{:.1} hp {}/{} lobs {}", m.p.x, m.p.y, m.hp, m.max, m.lobs.len())) + &game.boss.as_ref().map_or(String::new(), |g| format!(
            " {:?} at {:.1},{:.1} hp {}/{} shots {}",
            g.kind,
            g.p.x,
            g.p.y,
            g.hp,
            g.max,
            game.shots.len()
        ))
    )
}

/// One line of text centred over the world, on black.
fn banner(msg: &str, y: u16, area: Rect, buf: &mut Buffer) {
    let w = (msg.chars().count() as u16).min(area.width);
    let r = Rect { x: area.x + (area.width - w) / 2, y: y.min(area.bottom().saturating_sub(1)), width: w, height: 1 };
    Clear.render(r, buf);
    Paragraph::new(msg).style(Style::default().bg(Color::Rgb(0, 0, 0)).fg(AMBER).add_modifier(Modifier::BOLD)).render(r, buf);
}

fn hud(game: &Game, input: &Input, best: u32, paused: bool, area: Rect, buf: &mut Buffer) {
    let bar = Style::default().bg(Color::Rgb(0, 0, 0)).fg(AMBER);
    let grey = bar.fg(Color::Rgb(170, 170, 170));
    let lives: String = "▬ ".repeat(game.lives as usize);
    let (t, first) = levels::tower_of(game.floor);
    let tower = &levels::TOWERS[t];
    let top = Line::from(vec![
        Span::styled(" BRICKSPIRE ", bar.add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} · FLOOR {} {} ", tower.name.to_uppercase(), game.floor + 1 - first, game.floor_name), grey),
        Span::styled(format!(" SCORE {:06} ", game.score), bar.add_modifier(Modifier::BOLD)),
        // (room for a boss's health instead, during a fight)
        Span::styled(if game.boss_left() { String::new() } else { format!(" BEST {best:06} ") }, grey),
        Span::styled(format!(" {lives}"), bar.fg(Color::Rgb(235, 170, 55))),
    ]);
    let mut top = top;
    let boss_bar = match (&game.boss, &game.vane, &game.storm, &game.rook) {
        (Some(g), ..) => Some((
            match g.kind {
                levels::MiniKind::Spider => "SPIDER",
                levels::MiniKind::Jack => "THE JACK",
                levels::MiniKind::Serpent => "SEA SERPENT",
                levels::MiniKind::Apprentice => "APPRENTICE",
                levels::MiniKind::Gargoyle => "GARGOYLE",
            },
            g.hp,
            g.max,
        )),
        (_, Some(w), ..) => Some(("WEATHERCOCK", w.hp, w.max)),
        (_, _, Some(s), _) => Some(("STORM CLOUD", s.hp, s.max)),
        (.., Some(r)) => Some(("ROOK", r.hp, r.max)),
        _ => game.bell.as_ref().map(|b| ("THE GREAT BELL", b.hp, b.max)).or(game.mason.as_ref().map(|m| ("THE MASON", m.hp, m.max))),
    };
    if let Some((name, hp, max)) = boss_bar {
        let red = bar.fg(Color::Rgb(255, 90, 60));
        top.spans.push(Span::styled(format!(" {name} "), red.add_modifier(Modifier::BOLD)));
        top.spans.push(Span::styled(format!("{}{}", "■".repeat(hp as usize), "·".repeat((max - hp) as usize)), red));
    }
    // clear first: a world cell is a `▀` whose top half takes the fg colour, so text
    // styled over it would leave coloured half-blocks wherever there is no glyph
    let top_row = Rect { height: 1, ..area };
    Clear.render(top_row, buf);
    Paragraph::new(top).style(bar).render(top_row, buf);
    if area.height > 4 {
        let pad = if input.pad_name.is_some() { " · pad" } else { "" };
        let help = format!(
            " ←/→ mouse pad · space launch · p pause · r restart · n skip · g graphics · q quit [{}{}]",
            input.keys_note(),
            pad
        );
        let bottom_row = Rect { y: area.bottom() - 1, height: 1, ..area };
        Clear.render(bottom_row, buf);
        Paragraph::new(help).style(bar.fg(Color::Rgb(130, 130, 130))).render(bottom_row, buf);
    }
    let mid = area.y + area.height * 3 / 5;
    let msg = match game.state {
        State::Serve { wait } if wait < 2.5 && game.floor == first => Some(format!(" {} · {} · space to launch ", tower.name.to_uppercase(), game.floor_name)),
        State::Serve { wait } if wait < 2.5 => Some(format!(" FLOOR {}: {} · space to launch ", game.floor + 1 - first, game.floor_name)),
        State::Lost { .. } if game.lives == 0 => Some(" LAST BALL LOST ".to_string()),
        State::Lost { .. } => Some(" BALL LOST ".to_string()),
        State::Clear { .. } if game.arena.mode == Mode::Circle => Some(" TOWER CLIMBED · bonus 5000 ".to_string()),
        State::Clear { .. } => Some(format!(" FLOOR {} CLEAR ", game.floor + 1 - first)),
        State::Over => Some(format!(" GAME OVER · score {} · space to play again ", game.score)),
        _ => None,
    };
    if let Some(p) = game.map() {
        map_labels(&p, area, buf);
    }
    if paused {
        banner(" PAUSED · p to carry on ", mid, area, buf);
    } else if let Some(msg) = msg {
        banner(&msg, mid, area, buf);
    }
}

/// The map's words: what was climbed, each known tower's name under it (`?` for the
/// rest), and once the trail is drawn, where next.
fn map_labels(p: &map::Progress, area: Rect, buf: &mut Buffer) {
    let world = world_area(area);
    let h_px = world.height as f32 * 2.0;
    let aspect = world.width as f32 / h_px;
    let fade = (p.t / 0.6).min(1.0);
    let lit = |c: [u8; 3]| Color::Rgb((c[0] as f32 * fade) as u8, (c[1] as f32 * fade) as u8, (c[2] as f32 * fade) as u8);
    for (k, l) in map::SKYLINE.iter().enumerate() {
        let known = p.climbed(k) || k == p.next;
        let (x, base, ..) = map::place(k, aspect);
        let name = if known { l.name.to_uppercase() } else { "?".to_string() };
        let w = name.chars().count() as u16;
        let col = (x * h_px) as i32 - w as i32 / 2;
        let row = world.y + ((base + 0.07) * h_px / 2.0) as u16;
        if row >= world.bottom() {
            continue;
        }
        let col = col.clamp(0, world.width.saturating_sub(w) as i32) as u16;
        let fg = if k == p.next { lit([255, 176, 0]) } else if known { lit([210, 210, 210]) } else { lit([120, 110, 150]) };
        let r = Rect { x: world.x + col, y: row, width: w.min(world.width), height: 1 };
        Paragraph::new(name).style(Style::default().fg(fg).add_modifier(Modifier::BOLD)).render(r, buf);
    }
    let title = if p.next <= p.from {
        " EVERY TOWER CLIMBED · THE CLIMB BEGINS AGAIN ".to_string()
    } else {
        format!(" {} CLIMBED · TOWER BONUS 5000 ", map::SKYLINE[p.from].name.to_uppercase())
    };
    banner(&title, world.y + world.height / 8, area, buf);
    if p.ready() {
        banner(&format!(" NEXT: {} · space to climb ", map::SKYLINE[p.next].name.to_uppercase()), world.y + world.height / 8 + 2, area, buf);
    }
}

/// Headless: autoplay `--secs` seconds, render one frame, and paint the terminal
/// cells into a PPM (8x16 pixels per cell) exactly as drawn, plus the text in `.txt`.
fn snapshot(out: &str) -> io::Result<()> {
    let (cols, rows) = arg("--size")
        .and_then(|s| s.split_once('x').map(|(a, b)| (a.parse().unwrap_or(100), b.parse().unwrap_or(30))))
        .unwrap_or((100u16, 30u16));
    let secs: f32 = arg("--secs").and_then(|s| s.parse().ok()).unwrap_or(3.0);
    let mut game = new_game();
    let mut pilot = Pilot::new();
    for _ in 0..(secs / STEP) as usize {
        let s = pilot.steer(&game);
        game.step(s, true);
    }
    // --map T: straight to the map after this tower, T seconds in
    if let Some(t) = arg("--map").and_then(|s| s.parse::<f32>().ok()) {
        game.to_map();
        for _ in 0..(t / STEP) as usize {
            game.step(game::Steer::Move(0.0), false);
        }
    }
    // --turn F: clear the floor and stop F of the way through the tower's turn
    // (or tilt, onto the roof); F is how far through its eased movement
    if let Some(f) = arg("--turn").and_then(|s| s.parse::<f32>().ok()) {
        game.skip_floor();
        let floor = game.floor;
        for _ in 0..2000 {
            let done = game.tilt().unwrap_or_else(|| game.camera().fract());
            if done >= f.min(0.999) || game.floor != floor {
                break;
            }
            game.step(game::Steer::Move(0.0), false);
        }
    }
    let area = Rect::new(0, 0, cols, rows);
    let mut buf = Buffer::empty(area);
    let glyphs = glyphs_arg();
    let view = make_view(&game, world_area(area), glyphs);
    let t0 = Instant::now();
    let reps = 20;
    for _ in 0..reps {
        draw_world(&game, &view, glyphs, &mut buf);
    }
    eprintln!("draw: {:.2} ms per frame at {cols}x{rows}", t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    hud(&game, &Input::new(false), game.score, false, area, &mut buf);
    println!("{}", summary(&game));
    let text: String = (0..rows).map(|r| (0..cols).map(|c| buf[(c, r)].symbol().to_string()).collect::<String>() + "\n").collect();
    std::fs::write(format!("{out}.txt"), text)?;
    write_ppm(&buf, out)
}

fn write_ppm(buf: &Buffer, out: &str) -> io::Result<()> {
    let (cw, ch) = (8usize, 16usize);
    let (cols, rows) = (buf.area.width as usize, buf.area.height as usize);
    let (w, h) = (cols * cw, rows * ch);
    let mut img = vec![0u8; w * h * 3];
    let rgb = |c: Color| match c {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => [0, 0, 0],
    };
    for r in 0..rows {
        for c in 0..cols {
            let cell = &buf[(c as u16, r as u16)];
            let (fg, bg) = (rgb(cell.fg), rgb(cell.bg));
            let sym = cell.symbol().chars().next().unwrap_or(' ');
            for y in 0..ch {
                for x in 0..cw {
                    let col = if sym == '▀' {
                        if y < ch / 2 { fg } else { bg }
                    } else if sym != ' ' {
                        if (2..6).contains(&x) && (4..12).contains(&y) { fg } else { bg }
                    } else {
                        bg
                    };
                    let i = ((r * ch + y) * w + c * cw + x) * 3;
                    img[i..i + 3].copy_from_slice(&col);
                }
            }
        }
    }
    let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
    data.extend_from_slice(&img);
    std::fs::write(out, data)
}
