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

/// Parse one answer to an optional-number prompt.
///
/// `Ok(None)` is a blank answer, `Ok(Some(v))` a value, `Err(())` unusable
/// input. Split out from the prompt loop so the acceptance rules can be
/// tested without a terminal. Non-finite input is always rejected.
pub fn parse_optional(s: &str, allow_zero: bool, allow_negative: bool) -> Result<Option<f64>, ()> {
    if s.is_empty() {
        return Ok(None);
    }
    match s.parse::<f64>() {
        Ok(v) if !v.is_finite() => Err(()),
        Ok(v) if v > 0.0 => Ok(Some(v)),
        Ok(v) if v == 0.0 && allow_zero => Ok(Some(v)),
        Ok(v) if v < 0.0 && allow_negative => Ok(Some(v)),
        _ => Err(()),
    }
}

/// Fold a blank answer into the value a prompt already held.
///
/// A prompt showing a current value must keep it when the user presses Enter;
/// resetting to `None` would silently discard it.
pub fn resolve_keep(answer: Option<f64>, current: Option<f64>) -> Option<f64> {
    answer.or(current)
}

/// Like `ask_optional`, with a custom hint explaining what a blank answer means.
pub fn ask_optional_hint(prompt: &str, hint: &str, allow_zero: bool) -> Option<f64> {
    ask_optional_full(prompt, hint, allow_zero, false)
}

/// `ask_optional_hint` that also accepts negative values, for signed
/// quantities such as an apparent magnitude.
pub fn ask_optional_signed(prompt: &str, hint: &str) -> Option<f64> {
    ask_optional_full(prompt, hint, true, true)
}

fn ask_optional_full(prompt: &str, hint: &str, allow_zero: bool, allow_negative: bool) -> Option<f64> {
    loop {
        let s = read_line(&format!("{prompt} ({hint}): "));
        match parse_optional(&s, allow_zero, allow_negative) {
            Ok(v) => return v,
            Err(()) => {
                if allow_negative {
                    println!("  Please enter a number, or leave blank.");
                } else {
                    println!("  Please enter a number greater than zero, or leave blank.");
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_optional_reads_a_positive_value() {
        assert_eq!(parse_optional("0.8", false, false), Ok(Some(0.8)));
    }

    #[test]
    fn parse_optional_treats_blank_as_skip() {
        assert_eq!(parse_optional("", false, false), Ok(None));
    }

    #[test]
    fn parse_optional_rejects_zero_unless_allowed() {
        assert_eq!(parse_optional("0", false, false), Err(()));
        assert_eq!(parse_optional("0", true, false), Ok(Some(0.0)));
    }

    #[test]
    fn parse_optional_rejects_negatives_unless_allowed() {
        assert_eq!(parse_optional("-3", false, false), Err(()));
        assert_eq!(parse_optional("-3", false, true), Ok(Some(-3.0)));
    }

    #[test]
    fn parse_optional_accepts_a_negative_magnitude() {
        // Magnitude is a signed scale: the ISS reaches -5.9 and bright low-orbit
        // targets sit near 0 to 2, so the target-magnitude override has to
        // accept negatives or it is unusable for its headline case.
        assert_eq!(parse_optional("-5.9", true, true), Ok(Some(-5.9)));
        assert_eq!(parse_optional("0", true, true), Ok(Some(0.0)));
    }

    #[test]
    fn parse_optional_rejects_nonsense() {
        assert_eq!(parse_optional("abc", false, true), Err(()));
        assert_eq!(parse_optional("nan", false, true), Err(()));
        assert_eq!(parse_optional("inf", false, true), Err(()));
    }

    #[test]
    fn blank_keeps_the_current_value() {
        // Pressing Enter at a prompt that already holds a value must keep it:
        // changing seeing must not silently discard an entered sky brightness.
        assert_eq!(resolve_keep(None, Some(21.9)), Some(21.9));
        assert_eq!(resolve_keep(None, None), None);
    }

    #[test]
    fn an_entered_value_replaces_the_current_one() {
        assert_eq!(resolve_keep(Some(18.5), Some(21.9)), Some(18.5));
    }
}
