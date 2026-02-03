# gwtx (git/jj worktree extra)

[![Crates.io](https://img.shields.io/crates/v/gwtx.svg)](https://crates.io/crates/gwtx)
[![CI](https://github.com/h-michael/gwtx/actions/workflows/ci.yml/badge.svg)](https://github.com/h-michael/gwtx/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/gwtx.svg)](LICENSE-MIT)

CLI tool that enhances git worktree and jj workspace with automated setup and utilities.

**Supported VCS:**
- **Git** - `git worktree` operations
- **jj (Jujutsu)** - `jj workspace` operations (including colocated repositories)

> **Note:** This tool is under active development. Commands and configuration format may change in future versions.

## Problem

Every time you create a git worktree or jj workspace, you end up doing the same manual setup:

- Creating symlinks to config files like `.env.local`
- Copying `.env.example` to `.env`
- Creating cache directories

gwtx reads `.gwtx/config.yaml` from your repository and runs these tasks automatically when creating a worktree or workspace. It automatically detects whether you're in a git repository or jj repository and uses the appropriate commands.

## Installation

```bash
cargo install gwtx
```

See [INSTALL.md](INSTALL.md) for other installation methods (mise, Nix, GitHub Releases).

## Quick Start

1.  **Create `.gwtx/config.yaml`**: In your repository root, create a `.gwtx/config.yaml` file to define your worktree setup.
    ```yaml
    # .gwtx/config.yaml example
    mkdir:
      - path: build
        description: Build output directory
    link:
      - source: .env.local
        description: Local environment variables
    ```
2.  **Add a worktree**: Use `gwtx add` to create a new worktree with the defined setup.
    ```bash
    gwtx add ../my-feature-worktree
    ```
    This will create a new worktree at `../my-feature-worktree` and apply the `mkdir` and `link` operations defined in `.gwtx/config.yaml`.

For comprehensive details on all commands, options, and configuration, please refer to the CLI's built-in help (`gwtx --help` and `gwtx <subcommand> --help`) and `man` pages (`gwtx man`).

## VCS Support

gwtx automatically detects the version control system in use:

| VCS | Detection | Commands Used |
|-----|-----------|---------------|
| **Git** | `.git` directory | `git worktree add/remove/list` |
| **jj (Jujutsu)** | `.jj` directory | `jj workspace add/forget/list` |
| **jj colocated** | Both `.git` and `.jj` | `jj workspace` commands |

All gwtx commands work the same regardless of the underlying VCS. The tool transparently translates operations to the appropriate VCS commands.

**jj-specific notes:**
- jj "workspaces" serve a similar purpose to git "worktrees" (multiple working directories for the same repository)
- The "default" workspace in jj is treated as the main workspace
- Colocated repositories (jj on top of git) are fully supported

## Usage

### Creating new worktrees/workspaces

```bash
# Create a worktree/workspace with setup
gwtx add ../feature-branch

# Create a new branch and worktree (git)
# or new workspace with bookmark (jj)
gwtx add -b new-feature ../new-feature

# Interactive mode - select branch and path
gwtx add --interactive

# Preview without executing
gwtx add --dry-run ../test
```

### Listing worktrees/workspaces

```bash
# List all worktrees/workspaces with detailed information
gwtx list
gwtx ls  # Short alias

# Show header row with column names
gwtx list --header

# List only paths (useful for scripting)
gwtx list --path-only
gwtx ls -p
```

**Status Symbols:**
- `*` = Uncommitted changes (modified, deleted, or untracked files)

**jj-specific columns:**
- Shows workspace name and change ID instead of branch name when applicable
- Displays bookmark name if associated with the workspace

### Removing worktrees/workspaces

```bash
# Remove a worktree/workspace with safety checks
gwtx remove ../feature-branch

# Shorthand alias
gwtx rm ../feature-branch

# Remove current worktree/workspace (the one you're in)
gwtx remove --current

# Interactive mode - select worktrees/workspaces to remove
# Shows [current] marker for the one you're in
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
- Unpushed commits (git) / commits not on remote bookmarks (jj)

Use `--force` to bypass all checks and confirmation prompts.

### Changing to selected worktree

```bash
# Interactively select and cd to a worktree
gwtx cd

# Or get worktree path for scripting (works without shell integration)
cd "$(gwtx path)"
```

**`gwtx cd`**: **Requires shell integration** (`gwtx init` - see [Shell Integration](#shell-integration) section). Displays an interactive fuzzy finder (on Unix) or selection menu (on Windows) and automatically changes to the selected worktree. If shell integration is not enabled, the command will display a helpful error message with setup instructions.

**`gwtx path`**: Prints the selected worktree path to stdout. Works without shell integration. Useful for scripting or as a fallback.

### Configuration commands

```bash
# Show config format help
gwtx config

# Create a new repo config
gwtx config new

# Create a new global config
gwtx config new --global

# Validate configuration
gwtx config validate
```

### Hooks (trust required)

Hooks allow you to run custom commands before/after worktree operations:

```bash
# Review and trust hooks in .gwtx/config.yaml
gwtx trust

# Show hooks without trusting
gwtx trust --show

# Check trust status (exit 0 if trusted, 1 if untrusted)
gwtx trust --check

# Revoke trust
gwtx untrust

# List all trusted repositories
gwtx untrust --list
```

**Security:** For security, hooks require explicit trust via `gwtx trust` before execution. See [Hooks Configuration](#hooks) below.

## Shell Integration

**Optional feature.** Shell integration is not required for basic gwtx functionality (`add`, `remove`, `list`, `path`, etc.). It only adds quality-of-life improvements.

To enable shell completions and automatic hook trust warnings, add the following to your shell configuration:

**Bash** (`~/.bashrc`):
```bash
eval "$(gwtx init bash)"
```

**Zsh** (`~/.zshrc`):
```zsh
eval "$(gwtx init zsh)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
gwtx init fish | source
```

**PowerShell** (profile):
```powershell
Invoke-Expression (& gwtx init powershell | Out-String)
```

**Elvish** (`~/.config/elvish/rc.elv`):
```elvish
eval (gwtx init elvish | slurp)
```

Shell integration provides:
- **Shell completions** for commands and options
- **`gwtx cd` command** to interactively change directory to selected worktree (only available with shell integration)
- **Auto cd after add** - Automatically `cd` to newly created worktree (configurable via `auto_cd.after_add`)
- **Auto cd after remove** - Automatically `cd` when current worktree is removed (configurable via `auto_cd.after_remove`)
- **Automatic trust warnings** when entering directories with untrusted hooks

## Configuration

Create `.gwtx/config.yaml` in your repository root. See [examples/](examples/) for various use cases.

**JSON Schema:** The configuration format is validated against a JSON Schema located at `schema/gwtx.schema.json`. This schema can be used with editors that support YAML schema validation for autocomplete and validation.

**Editor Integration:** To enable schema validation in VS Code or other editors using [yaml-language-server](https://github.com/redhat-developer/yaml-language-server#using-inlined-schema), add this comment at the top of your `.gwtx/config.yaml`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/h-michael/gwtx/main/schema/gwtx.schema.json

on_conflict: backup
```

### Basic configuration

```yaml
on_conflict: backup  # abort, skip, overwrite, backup

mkdir:
  - path: build
    description: Build output directory

link:
  - source: .env.local
    description: Local environment

copy:
  - source: .env.example
    target: .env
    description: Environment template
```

**Operations:**
- `mkdir` - Create directories
- `link` - Create symbolic links
- `copy` - Copy files or directories

**Examples:** [examples/basic.yaml](examples/basic.yaml)

### Worktree path configuration

Configure default worktree path with template variables:

```yaml
worktree:
  path_template: ../worktrees/{branch}
```

**Template variables:**
- `{{branch}}` or `{{ branch }}` - Branch name (e.g., `feature/foo`)
- `{{repository}}` or `{{ repository }}` - Repository name (e.g., `myrepo`)

**Examples:** [examples/worktree-path.yaml](examples/worktree-path.yaml)

### Glob patterns

Use glob patterns in `link` operations to match multiple files:

```yaml
link:
  - source: fixtures/*
    ignore_tracked: true
    description: Link untracked test fixtures
```

**Supported patterns:**
- `*` - matches any characters
- `?` - matches a single character
- `[...]` - matches character ranges
- `**` - matches directories recursively

**Options:**
- `ignore_tracked: true` - Skip git-tracked files (useful for linking only untracked files like local configs or test data)

**Examples:** [examples/glob-patterns.yaml](examples/glob-patterns.yaml)

### Hooks

Execute custom commands before/after worktree operations. **Requires explicit trust via `gwtx trust`.**

```yaml
hooks:
  post_add:
    - command: npm install
      description: Install dependencies
    - command: mise install
      description: Install mise tools
```

**Hook types:**
- `pre_add` - Before worktree creation
- `post_add` - After worktree setup
- `pre_remove` - Before worktree removal
- `post_remove` - After worktree removal

**Template variables:**
- `{{worktree_path}}` - Full path to worktree
- `{{worktree_name}}` - Worktree directory name
- `{{branch}}` - Branch name
- `{{repo_root}}` - Repository root

**Security:**
- Variables are shell-escaped automatically (POSIX sh on Unix, PowerShell on Windows)
- Must trust hooks via `gwtx trust` before execution
- Changes require re-trusting

**Windows shells:**
- Default shell auto-detect order: `pwsh` → `powershell` → `bash` (Git Bash) → `cmd`
- Override with `--hook-shell` or `GWTHOOK_SHELL` (`pwsh`, `powershell`, `bash`, `cmd`, `wsl`)
- `--hook-shell` takes precedence over `GWTHOOK_SHELL`
- `wsl` is only used when explicitly set, because Windows paths may not map to WSL

**Examples:** [examples/hooks-basic.yaml](examples/hooks-basic.yaml), [examples/nodejs-project.yaml](examples/nodejs-project.yaml)

## Features

### Operations

| Operation | Description |
|-----------|-------------|
| `mkdir` | Create directories |
| `link` | Create symbolic links |
| `copy` | Copy files or directories |
| `hooks.*` | Run custom commands (requires trust) |

### Conflict handling

When a target file already exists, gwtx can:

- `abort` - Stop immediately (default in non-interactive mode)
- `skip` - Skip the file and continue
- `overwrite` - Replace the existing file
- `backup` - Rename existing file to `.bak` and proceed

Set globally with `on_conflict`, per-operation, or via `--on-conflict` flag.

### Auto cd (shell integration)

Automatically change directory after worktree operations. **Requires shell integration.**

```yaml
auto_cd:
  after_add: true     # cd to new worktree after creation (default: true)
  after_remove: main  # cd after removing current worktree (default: main)
```

**`after_remove` options:**
- `main` - cd to main worktree
- `select` - Show interactive selection

### Other options

| Option | Description |
|--------|-------------|
| `--interactive`, `-i` | Select branch and path interactively |
| `--dry-run` | Preview actions without executing |
| `--quiet`, `-q` | Suppress output |
| `--no-setup` | Skip setup (run git worktree add only) |
| `--hook-shell <SHELL>` | Windows only: choose hook shell (`pwsh`, `powershell`, `bash`, `cmd`, `wsl`) |

## Command Options

### gwtx add

Passes through git worktree options:

```
gwtx add [OPTIONS] [PATH] [COMMITISH]

gwtx Options:
  -i, --interactive         Interactive mode
      --on-conflict <MODE>  abort, skip, overwrite, backup
      --dry-run             Preview without executing
      --no-setup            Skip .gwtx/config.yaml setup
      --hook-shell <SHELL>  Windows only: hook shell

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

Remove worktrees with safety checks (alias: `gwtx rm`):

```
gwtx remove [OPTIONS] [PATHS]...
gwtx rm [OPTIONS] [PATHS]...

gwtx Options:
  -i, --interactive         Select worktrees interactively
  -c, --current             Remove current worktree
      --dry-run             Preview without executing
      --hook-shell <SHELL>  Windows only: hook shell

git worktree Options:
  -f, --force               Force removal (skip all checks and prompts)

Shared:
  -q, --quiet               Suppress output
```

## Shell Completion

Generate completion scripts for your shell:

```bash
# Bash
gwtx completions bash > ~/.local/share/bash-completion/completions/gwtx

# Zsh (add ~/.zfunc to your fpath)
gwtx completions zsh > ~/.zfunc/_gwtx

# Fish
gwtx completions fish > ~/.config/fish/completions/gwtx.fish

# PowerShell (add to your profile)
gwtx completions powershell >> $PROFILE

# PowerShell (or save to a file and dot-source it)
gwtx completions powershell > _gwtx.ps1
# Then add to $PROFILE: . /path/to/_gwtx.ps1
```

Supported shells: bash, elvish, fish, powershell, zsh

### PowerShell note

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

### Hooks

Hooks are supported on Unix-like systems (Linux, macOS) and Windows.

On Windows, hooks run via an auto-detected shell (`pwsh`, `powershell`, Git Bash,
or `cmd`). Use `GWTHOOK_SHELL` to force a specific shell if needed.

### Windows

On Windows, creating symbolic links requires one of the following:
- **Windows 11**: No special permissions required
- **Windows 10 Creators Update (1703) or later**: Enable [Developer Mode](https://blogs.windows.com/windowsdeveloper/2016/12/02/symlinks-windows-10/) in Settings > Update & Security > For Developers
- **Older Windows versions**: Run as administrator

Without these permissions, `[[link]]` operations will fail with a permission error.

For more information, see:
- [Symbolic Links - Win32 apps](https://learn.microsoft.com/en-us/windows/win32/fileio/symbolic-links)
- [CreateSymbolicLink API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinka)

## License

MIT OR Apache-2.0
