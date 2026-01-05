mod cli;
mod command;
mod config;
mod error;
mod git;
mod operation;
mod output;
mod prompt;

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

// Flag to indicate if a termination signal was received
static SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

fn setup_signal_handlers() {
    // ctrlc handles SIGINT (Ctrl+C) on all platforms
    // With `termination` feature, also handles SIGTERM and SIGHUP on Unix
    let _ = ctrlc::set_handler(|| {
        SIGNAL_RECEIVED.store(true, Ordering::SeqCst);
    });
}

fn main() -> ExitCode {
    setup_signal_handlers();

    let args = cli::parse();

    let result = match args.command {
        cli::Command::Add(add_args) => command::add(add_args),
        cli::Command::Validate => command::validate(),
    };

    // Check if a signal was received
    if SIGNAL_RECEIVED.load(Ordering::SeqCst) {
        // Exit with 130 (128 + SIGINT) - standard for Ctrl+C termination
        // This is the most common case across platforms
        return ExitCode::from(130);
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error::Error::Aborted) => {
            // User-initiated cancellation (e.g., Escape key) - not an error
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
