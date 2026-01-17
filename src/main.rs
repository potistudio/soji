use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::thread;
use std::time::Duration;

const BANNER: &str = r#"
 _   _
( `-._)    s o j i
 `  / \    ----------------------------
(_/ \_/    Bring "Zen" to your machine.
"#;

fn main() {
    // Print the banner
    println!("{}", BANNER.bright_cyan());

    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.green} {msg}")
            .expect("Failed to set progress bar style"),
    );
    progress_bar.set_message("Analyzing...");
    progress_bar.enable_steady_tick(Duration::from_millis(100));

    // Simulate work
    thread::sleep(Duration::from_secs(3));

    progress_bar.finish_with_message("Done!");
}
