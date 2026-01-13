# gwtx Configuration Examples

Example configuration files for various use cases. Browse through these examples to find patterns that fit your workflow.

## Available Examples

### Basic Examples

- **[basic.toml](basic.toml)**
  Demonstrates the three main operation types: `mkdir`, `link`, and `copy`. Shows conflict handling options and descriptions.

- **[worktree-path.toml](worktree-path.toml)**
  Examples of worktree path configuration with template variables (`{branch}`, `{repo_name}`). Useful for customizing worktree location and naming.

- **[glob-patterns.toml](glob-patterns.toml)**
  Shows how to use glob patterns in `link` operations. Includes `skip_tracked` option for linking only untracked files.

- **[hooks-basic.toml](hooks-basic.toml)**
  Basic hook examples with template variables. Shows all hook types (`pre_add`, `post_add`, `pre_remove`, `post_remove`) and their execution order.

### Project-Specific Examples

- **[nodejs-project.toml](nodejs-project.toml)**
  Node.js project setup with dependency installation (npm/pnpm/yarn). Links environment files and creates build directories.

- **[rust-project.toml](rust-project.toml)**
  Rust project setup with shared build cache. Shows how to link `target` directory and run `cargo check`.

- **[mise-direnv.toml](mise-direnv.toml)**
  Integration with mise (formerly rtx) and direnv. Automatically installs tools and trusts direnv configuration.

- **[coding-agent.toml](coding-agent.toml)**
  Coding agent integration (Claude Code, OpenAI Codex CLI, Cursor, Windsurf, Aider). Shares project-specific coding agent configurations across worktrees.

## Configuration File Format

All examples use TOML format. The basic structure is:

```toml
# Global options (optional)
[options]
on_conflict = "backup"  # abort, skip, overwrite, backup

# Worktree path template (optional)
[worktree]
path = "../worktrees/{branch}"

# Operations (all optional)
[[mkdir]]
path = "build"

[[link]]
source = ".env.local"

[[copy]]
source = ".env.example"
target = ".env"

# Hooks (optional, require trust)
[[hooks.post_add]]
command = "npm install"
description = "Install dependencies"
```

## Hook Security

Hooks require explicit trust before execution:

```bash
# Review and trust hooks
gwtx trust

# Revoke trust
gwtx untrust

# List all trusted repositories
gwtx untrust --list
```

Hooks are only supported on Unix-like systems (Linux, macOS). Windows users should use Git Bash/WSL or the `--no-setup` flag.

## See Also

- [Main README](../README.md) - Full documentation
- [Installation Guide](../INSTALL.md) - How to install
- [Root Example](../.gwtx.example.toml) - Quick reference with all options
