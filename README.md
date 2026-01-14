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

gwtx reads `.gwtx.yaml` from your repository and runs these tasks automatically when creating a worktree.

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

### Listing Worktrees

```bash
# List all worktrees with detailed information (branch, commit hash, status)
gwtx list
gwtx ls  # Short alias

# Show header row with column names
gwtx list --header

# List only worktree paths (useful for scripting)
gwtx list --path-only
gwtx ls -p
```

**Status Symbols:**
- `*` = Uncommitted changes (modified, deleted, or untracked files)

Note: Use `git status` in the worktree directory for detailed status information.

### Removing Worktrees

```bash
# Remove a worktree with safety checks
gwtx remove ../feature-branch

# Shorthand alias
gwtx rm ../feature-branch

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

### Hooks (Trust Required)

Hooks allow you to run custom commands before/after worktree operations:

```bash
# Review and trust hooks in .gwtx.yaml
gwtx trust

# Show hooks without trusting
gwtx trust --show

# Revoke trust
gwtx untrust

# List all trusted repositories
gwtx untrust --list
```

**Security:** For security, hooks require explicit trust via `gwtx trust` before execution. See [Hooks Configuration](#hooks) below.

## Configuration

Create `.gwtx.yaml` in your repository root. See [examples/](examples/) for various use cases.

**JSON Schema:** The configuration format is validated against a JSON Schema located at `schema/gwtx.schema.json`. This schema can be used with editors that support YAML schema validation for autocomplete and validation.

**Editor Integration:** To enable schema validation in VS Code or other editors using [yaml-language-server](https://github.com/redhat-developer/yaml-language-server#using-inlined-schema), add this comment at the top of your `.gwtx.yaml`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/h-michael/gwtx/main/schema/gwtx.schema.json

options:
  on_conflict: backup
```

### Basic Configuration

```yaml
options:
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

### Worktree Path Configuration

Configure default worktree path with template variables:

```yaml
worktree:
  path: ../worktrees/{branch}
```

**Template variables:**
- `{{branch}}` or `{{ branch }}` - Branch name (e.g., `feature/foo`)
- `{{repository}}` or `{{ repository }}` - Repository name (e.g., `myrepo`)

**Examples:** [examples/worktree-path.yaml](examples/worktree-path.yaml)

### Glob Patterns

Use glob patterns in `link` operations to match multiple files:

```yaml
link:
  - source: fixtures/*
    skip_tracked: true
    description: Link untracked test fixtures
```

**Supported patterns:**
- `*` - matches any characters
- `?` - matches a single character
- `[...]` - matches character ranges
- `**` - matches directories recursively

**Options:**
- `skip_tracked: true` - Skip git-tracked files (useful for linking only untracked files like local configs or test data)

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
- Variables are shell-escaped automatically
- Must trust hooks via `gwtx trust` before execution
- Changes require re-trusting

**Examples:** [examples/hooks-basic.yaml](examples/hooks-basic.yaml), [examples/nodejs-project.yaml](examples/nodejs-project.yaml)

## Features

### Operations

| Operation | Description |
|-----------|-------------|
| `mkdir` | Create directories |
| `link` | Create symbolic links |
| `copy` | Copy files or directories |
| `hooks.*` | Run custom commands (requires trust) |

### Conflict Handling

When a target file already exists, gwtx can:

- `abort` - Stop immediately (default in non-interactive mode)
- `skip` - Skip the file and continue
- `overwrite` - Replace the existing file
- `backup` - Rename existing file to `.bak` and proceed

Set globally in `options`, per-operation, or via `--on-conflict` flag.

### Other Options

| Option | Description |
|--------|-------------|
| `--interactive`, `-i` | Select branch and path interactively |
| `--dry-run` | Preview actions without executing |
| `--quiet`, `-q` | Suppress output |
| `--no-setup` | Skip setup (run git worktree add only) |

## Command Options

### gwtx add

Passes through git worktree options:

```
gwtx add [OPTIONS] [PATH] [COMMITISH]

gwtx Options:
  -i, --interactive         Interactive mode
      --on-conflict <MODE>  abort, skip, overwrite, backup
      --dry-run             Preview without executing
      --no-setup            Skip .gwtx.yaml setup

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

### Hooks

**Hooks are currently supported on Unix-like systems (Linux, macOS) only.**

Windows support is not yet available. For Windows users:
- Use Git Bash or WSL for hooks functionality
- Or use `--no-setup` flag to skip hooks

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
