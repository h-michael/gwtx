# gwtx (git worktree extra)

[![Crates.io](https://img.shields.io/crates/v/gwtx.svg)](https://crates.io/crates/gwtx)
[![CI](https://github.com/h-michael/gwtx/actions/workflows/ci.yml/badge.svg)](https://github.com/h-michael/gwtx/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/gwtx.svg)](LICENSE-MIT)

CLI tool that enhances git worktree with automated setup and utilities.

> **Note:** This tool is under active development. Commands and configuration format may change in future versions.

## Problem

Every time you create a git worktree, you end up doing the same manual setup:

- Creating symlinks to config files like `.env.local`
- Copying `.env.example` to `.env`
- Creating cache directories

gwtx reads `.gwtx.toml` from your repository and runs these tasks automatically when creating a worktree.

## Installation

```bash
cargo install gwtx
```

See [INSTALL.md](INSTALL.md) for other installation methods (mise, Nix, GitHub Releases).

## Usage

### Creating Worktrees

```bash
# Create a worktree with setup
gwtx add ../feature-branch

# Create a new branch and worktree
gwtx add -b new-feature ../new-feature

# Interactive mode - select branch and path
gwtx add --interactive

# Preview without executing
gwtx add --dry-run ../test
```

### Removing Worktrees

```bash
# Remove a worktree with safety checks
gwtx remove ../feature-branch

# Interactive mode - select worktrees to remove
gwtx remove --interactive

# Preview what would be removed
gwtx remove --dry-run ../feature-branch

# Force removal (skip safety checks and confirmation)
gwtx remove --force ../feature-branch
```

**Safety Checks:**

By default, `gwtx remove` warns about:
- Modified files
- Deleted files
- Untracked files
- Unpushed commits

Use `--force` to bypass all checks and confirmation prompts.

### Configuration Commands

```bash
# Show config format help
gwtx config

# Validate configuration
gwtx config validate
```

## Configuration

Create `.gwtx.toml` in your repository root (see [`.gwtx.example.toml`](.gwtx.example.toml)):

```toml
[options]
on_conflict = "backup"  # default conflict handling

[[mkdir]]
path = "build"
description = "Build output directory"

[[link]]
source = ".env.local"
description = "Local environment"

[[link]]
source = ".envrc"
description = "direnv configuration"

[[copy]]
source = ".env.example"
target = ".env"
description = "Environment file"
```

For all available options, see [`.gwtx.example.toml`](.gwtx.example.toml) or run `gwtx config`.

#### Glob Patterns

You can use glob patterns in `[[link]]` source to match multiple files:

```toml
[[link]]
source = "fixtures/*"
skip_tracked = true
description = "Link untracked test fixtures"
```

This is particularly useful when you want to:
- Link only git-ignored files (e.g., local configs, credentials)
- Skip git-tracked files (e.g., `.gitkeep`) that would conflict with directory symlinks

**Supported patterns:**
- `*` - matches any characters (e.g., `secrets/*` matches all files in `secrets/`)
- `?` - matches a single character (e.g., `file?.txt`)
- `[...]` - matches character ranges (e.g., `file[0-9].txt`)

**Options:**
- `skip_tracked = true` - Skip git-tracked files when creating symlinks (useful for linking only untracked files)

**Example use case:**

When you have a directory like:
```
fixtures/
├── .gitkeep        (tracked by git)
├── test-data.json  (ignored by git)
└── test-image.png  (ignored by git)
```

Using `source = "fixtures/*"` with `skip_tracked = true` will:
- Create symlinks for `test-data.json` and `test-image.png`
- Skip `.gitkeep` (leaving the git-tracked file intact)
- Keep `git status` clean in the worktree

## Features

### Operations

| Operation | Description |
|-----------|-------------|
| `[[mkdir]]` | Create directories |
| `[[link]]` | Create symbolic links |
| `[[copy]]` | Copy files or directories |

### Conflict Handling

When a target file already exists, gwtx can:

- `abort` - Stop immediately (default in non-interactive mode)
- `skip` - Skip the file and continue
- `overwrite` - Replace the existing file
- `backup` - Rename existing file to `.bak` and proceed

Set globally in `[options]`, per-operation, or via `--on-conflict` flag.

### Other Options

| Option | Description |
|--------|-------------|
| `--interactive`, `-i` | Select branch and path interactively |
| `--dry-run` | Preview actions without executing |
| `--quiet`, `-q` | Suppress output |
| `--no-setup` | Skip setup, run git worktree add only |

## Command Options

### gwtx add

Passes through git worktree options:

```
gwtx add [OPTIONS] [PATH] [COMMITISH]

gwtx Options:
  -i, --interactive         Interactive mode
      --on-conflict <MODE>  abort, skip, overwrite, backup
      --dry-run             Preview without executing
      --no-setup            Skip .gwtx.toml setup

git worktree Options:
  -b <name>                 Create new branch
  -B <name>                 Create or reset branch
  -f, --force               Force creation
  -d, --detach              Detach HEAD
      --no-checkout         Do not checkout after creation
      --lock                Lock worktree after creation
      --track / --no-track  Branch tracking
      --guess-remote        Guess remote for tracking
      --no-guess-remote     Do not guess remote

Shared:
  -q, --quiet               Suppress output
```

### gwtx remove

Remove worktrees with safety checks:

```
gwtx remove [OPTIONS] [PATHS]...

gwtx Options:
  -i, --interactive         Select worktrees interactively
      --dry-run             Preview without executing

git worktree Options:
  -f, --force               Force removal (skip all checks and prompts)

Shared:
  -q, --quiet               Suppress output
```

**Interactive Mode Keybindings:**
- `↑/↓` or `Ctrl+n/p` - Navigate
- `Space` - Toggle selection
- `→` (Right arrow) - Select all
- `←` (Left arrow) - Clear all
- `Enter` - Confirm
- `Esc` or `Ctrl+c` - Cancel

## Shell Completion

Generate completion scripts for your shell:

```bash
# Fish
gwtx completions fish > ~/.config/fish/completions/gwtx.fish

# Bash
gwtx completions bash > ~/.local/share/bash-completion/completions/gwtx

# Zsh (add ~/.zfunc to your fpath)
gwtx completions zsh > ~/.zfunc/_gwtx

# PowerShell (add to your profile)
gwtx completions powershell >> $PROFILE

# PowerShell (or save to a file and dot-source it)
gwtx completions powershell > _gwtx.ps1
# Then add to $PROFILE: . /path/to/_gwtx.ps1
```

Supported shells: bash, elvish, fish, powershell, zsh

### PowerShell Note

To find your PowerShell profile path, run `echo $PROFILE`. If the profile file doesn't exist, create it with `New-Item -Path $PROFILE -ItemType File -Force`.

## Man Page

Generate and install the man page:

```bash
# Install to system
sudo gwtx man > /usr/local/share/man/man1/gwtx.1

# View without installing
gwtx man | man -l -
```

## Platform Support

### Windows
On Windows, creating symbolic links requires one of the following:
- **Windows 11**: No special permissions required
- **Windows 10 Creators Update (1703) or later**: Enable [Developer Mode](https://blogs.windows.com/windowsdeveloper/2016/12/02/symlinks-windows-10/) in Settings > Update & Security > For Developers
- **Older Windows versions**: Run as administrator

Without these permissions, `[[link]]` operations will fail with a permission error.

For more information, see:
- [Symbolic Links - Win32 apps](https://learn.microsoft.com/en-us/windows/win32/fileio/symbolic-links)
- [CreateSymbolicLink API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinka)

## Roadmap

Planned features for future releases:

- `gwtx config init` - Generate `.gwtx.toml` template
- Hooks - Run custom scripts (pre-add, post-add)

## License

MIT OR Apache-2.0
