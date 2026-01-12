mod cli;
mod color;
mod command;
mod config;
mod error;
mod git;
mod hook;
mod operation;
mod output;
mod prompt;
mod trust;

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
    let color_choice = if args.no_color {
        clap::ColorChoice::Never
    } else {
        args.color
    };
    let color_config = color::ColorConfig::new(color_choice);

    let result = match args.command {
        cli::Command::Add(add_args) => command::add(add_args, color_config),
        cli::Command::Remove(remove_args) => command::remove(remove_args, color_config),
        cli::Command::List(list_args) => command::list(list_args, color_config),
        cli::Command::Config(config_args) => command::config(config_args.command),
        cli::Command::Trust(trust_args) => command::trust(trust_args),
        cli::Command::Untrust(untrust_args) => command::untrust(untrust_args),
        cli::Command::Completions { shell } => command::completions(shell),
        cli::Command::Man => command::man(),
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
        Err(error::Error::HooksNotTrusted) => {
            // Detailed message already displayed by command
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
