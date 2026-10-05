//! The command-line front end: prompts, the interactive menu and `--demo`.
//! Everything here reads the terminal or prints to it; nothing here
//! evaluates or formats a report.

mod demo;
mod input;
mod interactive;
mod prompts;

pub use demo::run_demo;
pub use interactive::run_interactive;
