use crate::color;
use crate::color::ColorScheme;
use crate::config::{self, Config};
use crate::error::{Error, Result};
use crate::hook;
use crate::trust;

use std::path::Path;

pub(crate) enum TrustHint {
    None,
    SkipHooks { command: &'static str },
}

pub(crate) fn load_config_with_trust_check(
    repo_root: &Path,
    main_worktree_path: &Path,
    enforce_hooks: bool,
    hint: TrustHint,
) -> Result<Config> {
    let initial_config = config::load(repo_root)?.unwrap_or_default();
    color::set_cli_theme(&initial_config.ui.colors);

    if enforce_hooks
        && initial_config.hooks.has_hooks()
        && !trust::is_trusted(main_worktree_path, &initial_config)?
    {
        hook::display_hooks_for_review(&initial_config.hooks);

        eprintln!();
        eprintln!("{}", ColorScheme::error("Configuration is not trusted."));
        eprintln!("The .gwtx.yaml file contains hooks that can execute arbitrary commands.");
        eprintln!("For security, you must explicitly review and trust the configuration.");
        eprintln!();
        eprintln!("To trust this configuration, run:");
        eprintln!("  gwtx trust");

        if let TrustHint::SkipHooks { command } = hint {
            eprintln!();
            eprintln!("Or skip hooks:");
            eprintln!("  {command}");
        }

        return Err(Error::HooksNotTrusted);
    }

    // TOCTOU protection: reload config immediately before use
    let config = config::load(repo_root)?.unwrap_or_default();
    color::set_cli_theme(&config.ui.colors);
    if enforce_hooks && config.hooks.has_hooks() && !trust::is_trusted(main_worktree_path, &config)?
    {
        eprintln!(
            "{}",
            ColorScheme::error(".gwtx.yaml was modified after trust check.")
        );
        eprintln!("For security, configuration must be re-trusted after any changes.");
        eprintln!("Run: gwtx trust");
        return Err(Error::HooksNotTrusted);
    }

    Ok(config)
}
