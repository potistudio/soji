use colored::*;

const BANNER: &str = r#"
 _   _
( `-._)    s o j i
 `  / \    ----------------------------
(_/ \_/    Bring "Zen" to your machine.
"#;

fn main() {
    // Print the banner
    println!("{}", BANNER.bright_cyan());
}
