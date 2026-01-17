use anyhow::Result;
use colored::*;
use crossterm::{
    cursor, execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use is_elevated::is_elevated;
use rand::Rng;
use rand::seq::SliceRandom;
use std::io::{Write, stdout}; // Removed stdin, Command
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
            // Since they are sorted Top-to-Bottom (random in row), this cleans row by row.
            for i in 0..num_to_purify {
                if let Some(p) = self.particles.get_mut(i) {
                    p.is_purifying = true;
                }
            }
        }
    }

    fn draw(&self) -> Result<()> {
        let mut stdout = stdout();

        // --- Double Buffering ---
        // Prepare the buffer with empty spaces and default color
        // height x width grid of (char, color)
        let mut buffer: Vec<Vec<(char, Color)>> =
            vec![vec![(' ', Color::Reset); self.width as usize]; self.height as usize];

        // 1. Draw Frame into buffer
        // Note: Coordinates are (y, x) for the buffer
        let border_color = Color::DarkGrey;
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                if y == 0 || y == (self.height - 1) as usize {
                    if x == 0 || x == (self.width - 1) as usize {
                        buffer[y][x] = ('+', border_color);
                    } else {
                        buffer[y][x] = ('-', border_color);
                    }
                } else if x == 0 || x == (self.width - 1) as usize {
                    buffer[y][x] = ('|', border_color);
                }
            }
        }

        // 2. Draw Dust Particles into buffer
        for p in &self.particles {
            let color = if p.is_purifying {
                Color::White
            } else {
                Color::DarkYellow
            };

            // Safety check for bounds
            if (p.y as usize) < self.height as usize && (p.x as usize) < self.width as usize {
                buffer[p.y as usize][p.x as usize] = (p.char, color);
            }
        }

        // 3. Render Buffer to Stdout
        // Move cursor to top-left of the canvas area
        // We know the loop in main moves cursor UP by canvas.height before calling draw.
        // So we start printing from there.

        for row in buffer {
            let mut last_color = Color::Reset; // Track color to minimize escape codes

            // Optimization: Detect color changes or batch strings?
            // Simple approach: Iterate chars.

            // To prevent artifacting from previous line lengths if we were not using fixed width,
            // but here we have fixed width box.

            execute!(stdout, cursor::MoveToColumn(0))?;

            for (ch, color) in row {
                if color != last_color {
                    execute!(stdout, SetForegroundColor(color))?;
                    last_color = color;
                }
                print!("{}", ch);
            }
            println!(); // Next line
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

    // 1. Check for Admin Privileges (Warn only)
    if !is_elevated() {
        println!(
            "{}",
            "Warning: Running without Administrator privileges.".yellow()
        );
        println!(
            "{}",
            "Some temporary files may not be deleted due to permissions.".yellow()
        );
        // Simple pause so user sees message
        thread::sleep(Duration::from_secs(2));
    }

    // Calculate temp dir size and collect files
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = scan_temp_files();
        let _ = tx.send(result);
    });

    let spinner_chars = ['|', '/', '-', '\\'];
    let mut i = 0;

    // Initial print to reserve line
    print!("{} Analyzing...", "⠋".yellow());
    stdout.flush()?;

    let (files, total_size) = loop {
        if let Ok(result) = rx.try_recv() {
            // Clear the spinner line
            print!("\r\x1b[2K");
            stdout.flush()?;
            break result?;
        }

        print!("\r{} Analyzing... {}", "⠋".yellow(), spinner_chars[i % 4]);
        stdout.flush()?;
        i += 1;
        thread::sleep(Duration::from_millis(100));
    };

    println!(
        "Found temporary files: {}",
        format_size(total_size).yellow()
    );

    // Safety check
    if total_size == 0 {
        println!("Nothing to clean.");
        return Ok(());
    }

    let mut canvas = Canvas::new(CANVAS_WIDTH, CANVAS_HEIGHT, DUST_DENSITY);

    // Reserve space for the canvas
    for _ in 0..canvas.height {
        println!();
    }

    let mut cleaned_size = 0;
    let mut processed_size = 0;

    // Iterate over actual files to calculate progress
    for file in files {
        // Try to delete the file, ignore errors (e.g. permission denied)
        if std::fs::remove_file(&file.path).is_ok() {
            cleaned_size += file.size;
        }

        // Always increment processed size so the animation completes
        processed_size += file.size;

        // Simulation delay
        thread::sleep(Duration::from_millis(5));

        // Calculate progress based on real bytes processed (success+failed)
        let progress = processed_size as f32 / total_size as f32;

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
        print!(
            " Cleaning... [{:.0}%] {} / {}  ",
            progress * 100.0,
            format_size(cleaned_size),
            format_size(total_size)
        );
        execute!(stdout, ResetColor)?;
        stdout.flush()?;
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
    print!(" Cleaning... [100%] ({})", format_size(cleaned_size));
    execute!(stdout, ResetColor)?;
    stdout.flush()?;
    println!(); // Move past the status line

    println!("\nDone. The machine is clean.");

    Ok(())
}

struct FileInfo {
    path: std::path::PathBuf,
    size: u64,
}

fn scan_temp_files() -> Result<(Vec<FileInfo>, u64)> {
    let temp_dir = std::env::temp_dir();
    let mut files = Vec::new();
    let mut total_size = 0;

    for entry in walkdir::WalkDir::new(&temp_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                let size = metadata.len();
                total_size += size;
                files.push(FileInfo {
                    path: entry.path().to_path_buf(),
                    size,
                });
            }
        }
    }
    Ok((files, total_size))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
