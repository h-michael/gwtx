# Contributing to gwtx

Thank you for considering contributing to gwtx! We welcome all contributions, including:

- Bug reports
- Feature requests
- Documentation improvements
- Code contributions

## Requirements

- Rust 1.85 or higher
- cargo

## Development

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/gwtx.git
   cd gwtx
   ```
3. Build the project:
   ```bash
   cargo build
   ```

### Development Commands

```bash
# Run tests
cargo test

# Check formatting
cargo fmt --all --check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Fail on clippy warnings (CI standard)
cargo clippy -- -D warnings
```

## Commit Messages

We recommend using [Conventional Commits](https://www.conventionalcommits.org/) format for commit messages:

- `feat: Add new feature` - New features
- `fix: Fix bug description` - Bug fixes
- `docs: Update documentation` - Documentation changes
- `perf: Improve performance` - Performance improvements
- `refactor: Refactor code` - Code refactoring
- `chore: Update dependencies` - Chores and maintenance tasks

### CHANGELOG

The CHANGELOG is automatically generated using [git-cliff](https://git-cliff.org/). Only commits matching the above types will appear in the CHANGELOG. The following commits are excluded:

- `chore(release): ...` - Release preparation commits
- `chore.*CHANGELOG` - CHANGELOG update commits

**Note:** Conventional Commits format is recommended but not required. Non-conventional commits simply won't appear in the auto-generated CHANGELOG.

## Pull Requests

1. **Discuss first**: For large changes, please open an issue first to discuss the proposed changes.
2. **Make your changes**: Ensure all tests pass and code is properly formatted.
3. **Submit PR**: Open a pull request with a clear description of your changes.
4. **Merge strategy**: We use regular merge (not squash merge) to preserve commit history.

## License

By contributing to gwtx, you agree that your contributions will be licensed under either:

- MIT License
- Apache License 2.0

at the user's option, matching the project's dual licensing.
