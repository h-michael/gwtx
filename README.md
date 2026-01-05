# gwtx

CLI tool that adds setup tasks to git worktree add.

## Problem

Every time you create a git worktree, you end up doing the same manual setup:

- Creating symlinks to config files like `.env.local`
- Copying `.env.example` to `.env`
- Creating cache directories

gwtx reads `.gwtx.toml` from your repository and runs these tasks automatically when creating a worktree.

## Installation

```
cargo install --path .
```

## Usage

```bash
# Create a worktree with setup
gwtx add ../feature-branch

# Create a new branch and worktree
gwtx add -b new-feature ../new-feature

# Interactive mode - select branch and path
gwtx add --interactive

# Preview without executing
gwtx add --dry-run ../test

# Validate configuration
gwtx validate
```

### Configuration

Create `.gwtx.toml` in your repository root:

```toml
[options]
on_conflict = "backup"  # default conflict handling

[[mkdir]]
path = "tmp/cache"
description = "Cache directory"

[[link]]
source = ".env.local"
description = "Local environment"

[[link]]
source = "config/credentials.json"
target = ".credentials.json"
on_conflict = "skip"

[[copy]]
source = ".env.example"
target = ".env"
description = "Environment file"
```

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

gwtx passes through git worktree options:

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
      --lock                Lock worktree after creation
      --track / --no-track  Branch tracking

Shared:
  -q, --quiet               Suppress output
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

## License

MIT OR Apache-2.0
