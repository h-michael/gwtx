# Changelog

## [0.5.0](https://github.com/h-michael/gwtx/compare/v0.4.0..v0.5.0) - 2026-01-14


- **feat**: Add trusted publishing support for crates.io - ([22fc2d4](https://github.com/h-michael/gwtx/commit/22fc2d439daa70a9e220a9956fa0ea39c8565f39))
- **feat**: [**breaking**] Migrate configuration from TOML to YAML with JSON Schema - ([c2501c5](https://github.com/h-michael/gwtx/commit/c2501c57bde59fe1bb0dffd15ec7ff5abaea5ea0))
- **fix**: Filter out symbolic refs from remote branch list - ([1c6000a](https://github.com/h-michael/gwtx/commit/1c6000aca2277357fe75e7b4c0ff4443be0e9c61))
- **fix**: Reject unknown fields in YAML configuration - ([0f67c64](https://github.com/h-michael/gwtx/commit/0f67c64afa29caf0a8b9ce4e349bf7f4f05deaf2))
- **feat**: [**breaking**] Add branch_template support for interactive branch creation - ([4d0ca0e](https://github.com/h-michael/gwtx/commit/4d0ca0edcddf5e42189e5c00b9be2e5dd6c3f10c))
- **feat**: [**breaking**] Require double braces for template variables - ([977fcf4](https://github.com/h-michael/gwtx/commit/977fcf498e042d5090f20aca5a17d4a937431674))
- **feat**: Add clippy lints configuration - ([d91de6f](https://github.com/h-michael/gwtx/commit/d91de6f235decb04f7ee9279669edc439f498614))
- **fix**: Clear screen immediately when entering interactive mode - ([0bd6cae](https://github.com/h-michael/gwtx/commit/0bd6caebf771bc235cfb2d081daf6d57a162218a))
- **refactor**: Move color options from global to per-command scope - ([5c9814f](https://github.com/h-michael/gwtx/commit/5c9814fb5f7b7ddb7ffa4c615e8245af7e2dcb7f))
## [0.4.0](https://github.com/h-michael/gwtx/compare/v0.3.0..v0.4.0) - 2026-01-13


- **feat**: Add hooks with trust mechanism - ([8f77ecf](https://github.com/h-michael/gwtx/commit/8f77ecfacc8ebf0c12c0ade4d9c603d1874319da))
- **feat**: Add gwtx list command - ([de2f16d](https://github.com/h-michael/gwtx/commit/de2f16d3c6c8ec5f5d05b7859d799640b2f1e31d))
- **feat**: Add hook description field and color output options - ([4a222db](https://github.com/h-michael/gwtx/commit/4a222dbe83eaf30b323e178f1f82677b4839089d))
- **refactor**: Improve code quality and documentation - ([48e55bb](https://github.com/h-michael/gwtx/commit/48e55bbb47ac96c841043b066b630ed30d0d4466))
- **feat**: Integrate skim fuzzy finder for Unix platforms - ([0178cae](https://github.com/h-michael/gwtx/commit/0178cae33d95e8f675a41a64b717d3b9f5552809))
- **fix**: Enable directory symlinks with glob patterns - ([fcab10c](https://github.com/h-michael/gwtx/commit/fcab10cc2abe578fd74d40705d5745061ee7f162))
- **feat**: Add worktree path configuration with variable expansion - ([425c5ca](https://github.com/h-michael/gwtx/commit/425c5caf9074061ef4d3a1644a8b5a2701e47ea4))
- **feat**: Add color output to remove command warnings - ([5eb286b](https://github.com/h-michael/gwtx/commit/5eb286baf2a70a3621b9521f04bdf0180dcccf08))
- **docs**: Organize examples into dedicated directory - ([a57e780](https://github.com/h-michael/gwtx/commit/a57e780e7bd703304a767183f1b29d493f686a96))
## [0.3.0](https://github.com/h-michael/gwtx/compare/v0.2.0..v0.3.0) - 2026-01-11


- **chore**(deps): Add Dependabot configuration - ([672b7a8](https://github.com/h-michael/gwtx/commit/672b7a8e74e8f16c73369e05aebc9fb16c7d09bf))
- **docs**: Add contributing guide - ([af4804e](https://github.com/h-michael/gwtx/commit/af4804efb89fe09118ab2520b1b9a87d6d72ef08))
- **feat**: Add remove command - ([16fcf61](https://github.com/h-michael/gwtx/commit/16fcf610de135fad908301d756a362b0808ffadd))
- **docs**: Add missing git worktree options to README - ([99ba301](https://github.com/h-michael/gwtx/commit/99ba301d03af3bcf9a2697a6cdd610b28e7178e4))
- **chore**: Update git-cliff configuration - ([df8c7fb](https://github.com/h-michael/gwtx/commit/df8c7fb78ae37b39220e02fb78bfb78977f4472d))
- **chore**(ci): Bump actions/checkout from 4.3.1 to 6.0.1 - ([0da41e2](https://github.com/h-michael/gwtx/commit/0da41e2d011c696dcf19e745759ff9dd4fcde064))

## New Contributors

* @dependabot[bot] made their first contribution

## [0.2.0](https://github.com/h-michael/gwtx/compare/v0.1.2..v0.2.0) - 2026-01-11


- **docs**: Add INSTALL.md and flake.nix - ([d61ab5a](https://github.com/h-michael/gwtx/commit/d61ab5a20b974d6984bf173c2fc327b0c5f24a10))
- **docs**: Add badges to README - ([d6059dd](https://github.com/h-michael/gwtx/commit/d6059dd005245725162c4b70f196ca6c03413471))
- **feat**: Add glob pattern support with skip_tracked option - ([bd75501](https://github.com/h-michael/gwtx/commit/bd755011d7209b1d251a51ae5e1e08809a154d7e))
- **docs**: Improve config examples with realistic use cases - ([0a1676e](https://github.com/h-michael/gwtx/commit/0a1676e560d187fee6733396959bef26e91d8537))
- **chore**: Bump version to 0.2.0 - ([3e7e1a4](https://github.com/h-michael/gwtx/commit/3e7e1a477ccfa41a0d3f1c0fb75a88d769782853))
## [0.1.2](https://github.com/h-michael/gwtx/compare/v0.1.1..v0.1.2) - 2026-01-05


- **chore**: Bump version to 0.1.2 - ([1af2b0d](https://github.com/h-michael/gwtx/commit/1af2b0d1c47a7fc984e1005e2b7a1af4f3130f54))
## [0.1.1] - 2026-01-05


- **refactor**: Make config file optional - ([9a51929](https://github.com/h-michael/gwtx/commit/9a5192909fb4ee6fd137e7d77a8e4ddaefba5ecf))
- **fix**: Detect Unix-style absolute paths on Windows - ([0bce1cd](https://github.com/h-michael/gwtx/commit/0bce1cddd7eb81c1cb5d26f78cc6ba097e747c31))
- **fix**: Add write permission for release workflow - ([fd91b72](https://github.com/h-michael/gwtx/commit/fd91b7296f7164b6f7799bd2a4db3a1ef0d82171))
- **chore**: Bump version to 0.1.1 and use draft releases - ([93762c5](https://github.com/h-michael/gwtx/commit/93762c5ae7da689cdec03547f0d69dad9fb444b4))

## New Contributors

* @h-michael made their first contribution in [#5](https://github.com/h-michael/gwtx/pull/5)

<!-- generated by git-cliff -->
