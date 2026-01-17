use crate::cli::TrustArgs;
use crate::color::{ColorConfig, ColorScheme};
use crate::{config, error::Error, error::Result, git, prompt, trust};

pub(crate) fn run(args: TrustArgs, color_config: ColorConfig) -> Result<()> {
    if args.check {
        let repo_root = match args.path {
            Some(p) => p.canonicalize()?,
            None => {
                if !git::is_inside_repo() {
                    return Ok(());
                }
                git::repository_root()?
            }
        };

        let config = match config::load(&repo_root)? {
            Some(config) => config,
            None => return Ok(()),
        };

        if !config.hooks.has_hooks() {
            return Ok(());
        }

        let main_worktree_path = git::main_worktree_path_for(&repo_root)?;
        let is_trusted = trust::is_trusted(&main_worktree_path, &config.hooks)?;
        if is_trusted {
            return Ok(());
        }

        return Err(Error::TrustCheckFailed);
    }

    let repo_root = match args.path {
        Some(p) => p.canonicalize()?,
        None => git::repository_root()?,
    };

    let main_worktree_path = git::main_worktree_path_for(&repo_root)?;

    let config = config::load(&repo_root)?.ok_or_else(|| Error::ConfigNotFound {
        path: repo_root.clone(),
    })?;

    if !config.hooks.has_hooks() {
        return Err(Error::NoHooksDefined);
    }

    if args.show {
        let use_color = color_config.is_enabled();

        println!("Hooks in {}:", repo_root.display());
        if !config.hooks.pre_add.is_empty() {
            println!();
            if use_color {
                println!("{}", ColorScheme::hook_type("pre_add:"));
            } else {
                println!("pre_add:");
            }
            for entry in &config.hooks.pre_add {
                println!("  {}", entry.command);
                if let Some(desc) = &entry.description {
                    if use_color {
                        println!(
                            "  {} {}",
                            ColorScheme::hook_arrow("->"),
                            ColorScheme::hook_description(desc)
                        );
                    } else {
                        println!("  -> {}", desc);
                    }
                } else if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::dimmed("no description")
                    );
                } else {
                    println!("  -> no description");
                }
            }
        }
        if !config.hooks.post_add.is_empty() {
            println!();
            if use_color {
                println!("{}", ColorScheme::hook_type("post_add:"));
            } else {
                println!("post_add:");
            }
            for entry in &config.hooks.post_add {
                println!("  {}", entry.command);
                if let Some(desc) = &entry.description {
                    if use_color {
                        println!(
                            "  {} {}",
                            ColorScheme::hook_arrow("->"),
                            ColorScheme::hook_description(desc)
                        );
                    } else {
                        println!("  -> {}", desc);
                    }
                } else if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::dimmed("no description")
                    );
                } else {
                    println!("  -> no description");
                }
            }
        }
        if !config.hooks.pre_remove.is_empty() {
            println!();
            if use_color {
                println!("{}", ColorScheme::hook_type("pre_remove:"));
            } else {
                println!("pre_remove:");
            }
            for entry in &config.hooks.pre_remove {
                println!("  {}", entry.command);
                if let Some(desc) = &entry.description {
                    if use_color {
                        println!(
                            "  {} {}",
                            ColorScheme::hook_arrow("->"),
                            ColorScheme::hook_description(desc)
                        );
                    } else {
                        println!("  -> {}", desc);
                    }
                } else if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::dimmed("no description")
                    );
                } else {
                    println!("  -> no description");
                }
            }
        }
        if !config.hooks.post_remove.is_empty() {
            println!();
            if use_color {
                println!("{}", ColorScheme::hook_type("post_remove:"));
            } else {
                println!("post_remove:");
            }
            for entry in &config.hooks.post_remove {
                println!("  {}", entry.command);
                if let Some(desc) = &entry.description {
                    if use_color {
                        println!(
                            "  {} {}",
                            ColorScheme::hook_arrow("->"),
                            ColorScheme::hook_description(desc)
                        );
                    } else {
                        println!("  -> {}", desc);
                    }
                } else if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::dimmed("no description")
                    );
                } else {
                    println!("  -> no description");
                }
            }
        }

        let is_trusted = trust::is_trusted(&main_worktree_path, &config.hooks)?;
        println!(
            "\nTrust status: {}",
            if is_trusted { "trusted" } else { "not trusted" }
        );
        return Ok(());
    }

    // Display hooks
    let use_color = color_config.is_enabled();

    if use_color {
        println!(
            "{}",
            ColorScheme::warning("WARNING: Review these commands before trusting")
        );
    } else {
        println!("WARNING: Review these commands before trusting");
    }
    println!();
    println!("Repository: {}", repo_root.display());

    if !config.hooks.pre_add.is_empty() {
        if use_color {
            println!("{}", ColorScheme::hook_type("pre_add:"));
        } else {
            println!("pre_add:");
        }
        for entry in &config.hooks.pre_add {
            println!("  {}", entry.command);
            if let Some(desc) = &entry.description {
                if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::hook_description(desc)
                    );
                } else {
                    println!("  -> {}", desc);
                }
            } else if use_color {
                println!(
                    "  {} {}",
                    ColorScheme::hook_arrow("->"),
                    ColorScheme::dimmed("no description")
                );
            } else {
                println!("  -> no description");
            }
        }
        println!();
    }
    if !config.hooks.post_add.is_empty() {
        if use_color {
            println!("{}", ColorScheme::hook_type("post_add:"));
        } else {
            println!("post_add:");
        }
        for entry in &config.hooks.post_add {
            println!("  {}", entry.command);
            if let Some(desc) = &entry.description {
                if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::hook_description(desc)
                    );
                } else {
                    println!("  -> {}", desc);
                }
            } else if use_color {
                println!(
                    "  {} {}",
                    ColorScheme::hook_arrow("->"),
                    ColorScheme::dimmed("no description")
                );
            } else {
                println!("  -> no description");
            }
        }
        println!();
    }
    if !config.hooks.pre_remove.is_empty() {
        if use_color {
            println!("{}", ColorScheme::hook_type("pre_remove:"));
        } else {
            println!("pre_remove:");
        }
        for entry in &config.hooks.pre_remove {
            println!("  {}", entry.command);
            if let Some(desc) = &entry.description {
                if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::hook_description(desc)
                    );
                } else {
                    println!("  -> {}", desc);
                }
            } else if use_color {
                println!(
                    "  {} {}",
                    ColorScheme::hook_arrow("->"),
                    ColorScheme::dimmed("no description")
                );
            } else {
                println!("  -> no description");
            }
        }
        println!();
    }
    if !config.hooks.post_remove.is_empty() {
        if use_color {
            println!("{}", ColorScheme::hook_type("post_remove:"));
        } else {
            println!("post_remove:");
        }
        for entry in &config.hooks.post_remove {
            println!("  {}", entry.command);
            if let Some(desc) = &entry.description {
                if use_color {
                    println!(
                        "  {} {}",
                        ColorScheme::hook_arrow("->"),
                        ColorScheme::hook_description(desc)
                    );
                } else {
                    println!("  -> {}", desc);
                }
            } else if use_color {
                println!(
                    "  {} {}",
                    ColorScheme::hook_arrow("->"),
                    ColorScheme::dimmed("no description")
                );
            } else {
                println!("  -> no description");
            }
        }
        println!();
    }

    // Prompt for confirmation
    if prompt::is_interactive() {
        if prompt::prompt_trust_hooks(&repo_root)? {
            trust::trust(&main_worktree_path, &config.hooks)?;
            println!("\n✓ Hooks trusted for: {}", repo_root.display());
            println!("These hooks will now run automatically on gwtx add/remove commands.");
        } else {
            println!("\nHooks were not trusted.");
            return Err(Error::Aborted);
        }
    } else {
        // Non-interactive: cannot prompt
        eprintln!("\nError: Cannot prompt for confirmation in non-interactive mode.");
        eprintln!("Run this command in an interactive terminal to trust hooks.");
        return Err(Error::NonInteractive);
    }

    Ok(())
}
