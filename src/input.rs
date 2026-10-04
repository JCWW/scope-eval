//! Small helpers for reading validated values from the terminal.

use std::io::{self, Write};

/// Print a prompt and read one trimmed line. Exits cleanly if input is closed.
pub fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
        Ok(0) | Err(_) => {
            println!("\nInput closed. Exiting.");
            std::process::exit(0);
        }
        Ok(_) => buf.trim().to_string(),
    }
}

fn with_default(prompt: &str, default: Option<String>) -> String {
    match default {
        Some(d) => format!("{prompt} [{d}]: "),
        None => format!("{prompt}: "),
    }
}

/// A required number greater than zero.
pub fn ask_positive(prompt: &str, default: Option<f64>) -> f64 {
    loop {
        let s = read_line(&with_default(prompt, default.map(|d| d.to_string())));
        if s.is_empty() {
            if let Some(d) = default {
                return d;
            }
            println!("  A value is required.");
            continue;
        }
        match s.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 => return v,
            _ => println!("  Please enter a number greater than zero."),
        }
    }
}

/// A number that may be zero.
pub fn ask_nonnegative(prompt: &str, default: f64) -> f64 {
    loop {
        let s = read_line(&with_default(prompt, Some(default.to_string())));
        if s.is_empty() {
            return default;
        }
        match s.parse::<f64>() {
            Ok(v) if v.is_finite() && v >= 0.0 => return v,
            _ => println!("  Please enter zero or a positive number."),
        }
    }
}

/// An optional number. Blank means "unknown / skip". Zero allowed only if `allow_zero`.
pub fn ask_optional(prompt: &str, allow_zero: bool) -> Option<f64> {
    ask_optional_hint(prompt, "blank to skip", allow_zero)
}

/// Like `ask_optional`, with a custom hint explaining what a blank answer means.
pub fn ask_optional_hint(prompt: &str, hint: &str, allow_zero: bool) -> Option<f64> {
    loop {
        let s = read_line(&format!("{prompt} ({hint}): "));
        if s.is_empty() {
            return None;
        }
        match s.parse::<f64>() {
            Ok(v) if v.is_finite() && (v > 0.0 || (allow_zero && v == 0.0)) => return Some(v),
            _ => println!("  Please enter a valid number, or leave blank."),
        }
    }
}

/// A required positive whole number.
pub fn ask_count(prompt: &str) -> u32 {
    loop {
        let s = read_line(&format!("{prompt}: "));
        match s.parse::<u32>() {
            Ok(v) if v > 0 => return v,
            _ => println!("  Please enter a whole number greater than zero."),
        }
    }
}

/// A fraction entered either as a percent (56) or a decimal (0.56).
pub fn ask_fraction(prompt: &str) -> f64 {
    loop {
        let v = ask_positive(prompt, None);
        let f = if v > 1.0 { v / 100.0 } else { v };
        if f < 1.0 {
            return f;
        }
        println!("  The obstruction must be less than 100%.");
    }
}

pub fn ask_text(prompt: &str, default: &str) -> String {
    let s = read_line(&with_default(prompt, Some(default.to_string())));
    if s.is_empty() {
        default.to_string()
    } else {
        s
    }
}

pub fn ask_yes_no(prompt: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    loop {
        let s = read_line(&format!("{prompt} [{hint}]: ")).to_lowercase();
        match s.as_str() {
            "" => return default,
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("  Please answer y or n."),
        }
    }
}

/// Numbered menu. Returns the zero-based index of the choice.
pub fn ask_menu(title: &str, options: &[String]) -> usize {
    println!("\n{title}");
    for (i, o) in options.iter().enumerate() {
        println!("  {}) {o}", i + 1);
    }
    loop {
        let s = read_line("Choice: ");
        match s.parse::<usize>() {
            Ok(n) if (1..=options.len()).contains(&n) => return n - 1,
            _ => println!("  Enter a number from 1 to {}.", options.len()),
        }
    }
}
