use anyhow::Result;
use colored::*;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use rand::Rng;
use rand::seq::SliceRandom;
use std::io::{Write, stdout};
use std::thread;
use std::time::Duration;

const BANNER: &str = r#"
 _   _
( `-._)    s o j i
 `  / \    ----------------------------
(_/ \_/    Bring "Zen" to your machine.
"#;

const CANVAS_WIDTH: u16 = 39;
const CANVAS_HEIGHT: u16 = 10;
const DUST_DENSITY: f32 = 0.2;

const UPDATE_INTERVAL_MS: u64 = 50;
const CLEANING_STEPS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Particle {
    x: u16,
    y: u16,
    char: char,
    is_purifying: bool,
}

#[derive(Debug)]
struct Canvas {
    width: u16,
    height: u16,
    particles: Vec<Particle>,
    total_particles: usize,
}

impl Canvas {
    fn new(width: u16, height: u16, density: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut particles = Vec::new();
        let chars = ['.', '*', '°', '+']; // Characters like dust

        let total_cells = (width * height) as f32;
        let count = (total_cells * density) as usize;

        for _ in 0..count {
            particles.push(Particle {
                x: rng.gen_range(1..width - 1), // Inside the border
                y: rng.gen_range(1..height - 1),
                char: chars[rng.gen_range(0..chars.len())],
                is_purifying: false,
            });
        }

        let total = particles.len();

        // 1. Shuffle to randomize X order (and relative order of same-Y particles)
        particles.shuffle(&mut rng);
        // 2. Sort by Y to ensure strictly Top-to-Bottom processing
        particles.sort_by_key(|p| p.y);

        Canvas {
            width,
            height,
            particles,
            total_particles: total,
        }
    }

    // Clean dust one by one
    fn update(&mut self, progress: f32) {
        // 1. Remove particles that were purifying in the last frame
        self.particles.retain(|p| !p.is_purifying);

        // 2. Calculate how many need to be removed to match progress
        let target_count = (self.total_particles as f32 * (1.0 - progress)) as usize;
        let current_count = self.particles.len();

        if current_count > target_count {
            let num_to_purify = current_count - target_count;

            // Mark the first N particles as purifying
            // Since the vector is unsorted (random order), this picks random particles.
            for i in 0..num_to_purify {
                if let Some(p) = self.particles.get_mut(i) {
                    p.is_purifying = true;
                }
            }
        }
    }

    fn draw(&self) -> Result<()> {
        let mut stdout = stdout();

        // 1. Draw frame (background)
        execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
        for y in 0..self.height {
            execute!(stdout, cursor::MoveToColumn(0))?;

            if y == 0 || y == self.height - 1 {
                // Top and bottom borders
                print!("+{}+", "-".repeat((self.width - 2) as usize));
            } else {
                // Side borders + clear content
                print!("|{}|", " ".repeat((self.width - 2) as usize));
            }

            println!();
        }

        // 2. Draw dust particles
        let mut current_color = Color::Reset; // Sentinel

        for p in &self.particles {
            let color = if p.is_purifying {
                Color::White
            } else {
                Color::DarkYellow
            };

            if color != current_color {
                execute!(stdout, SetForegroundColor(color))?;
                current_color = color;
            }

            execute!(
                stdout,
                cursor::MoveUp(self.height - p.y),
                cursor::MoveToColumn(p.x),
                Print(p.char),
                cursor::MoveDown(self.height - p.y), // Return to bottom
            )?;
        }

        execute!(stdout, ResetColor)?;
        Ok(())
    }
}

struct CursorGuard;

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), cursor::Show);
    }
}

fn main() -> Result<()> {
    let mut stdout = stdout();

    // Clear screen and show banner, hide cursor
    execute!(
        stdout,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide
    )?;

    // Restore cursor on exit
    let _guard = CursorGuard;

    println!("{}\n", BANNER.bright_cyan());

    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT, DUST_DENSITY);

    // Reserve space for the canvas
    for _ in 0..canvas.height {
        println!();
    }

    for i in 0..=CLEANING_STEPS {
        let progress = i as f32 / CLEANING_STEPS as f32;

        // Update logic
        canvas.update(progress);

        // Move cursor up to the start of the canvas to redraw
        execute!(stdout, cursor::MoveUp(canvas.height))?;
        canvas.draw()?;

        // After draw(), we are at the bottom of the canvas.

        // Display status at the bottom
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            SetForegroundColor(Color::Cyan)
        )?;
        print!(" Cleaning... [{:.0}%]  ", progress * 100.0);
        execute!(stdout, ResetColor)?;
        stdout.flush()?;

        thread::sleep(Duration::from_millis(UPDATE_INTERVAL_MS));
    }

    // Final cleanup: Remove the last purifying particles and draw empty canvas
    canvas.update(1.0);
    execute!(stdout, cursor::MoveUp(canvas.height))?;
    canvas.draw()?;

    // Final status update
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        SetForegroundColor(Color::Cyan)
    )?;
    print!(" Cleaning... [100%]");
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    println!(); // Move past the status line

    println!("\nDone. The machine is clean.");
    Ok(())
}
