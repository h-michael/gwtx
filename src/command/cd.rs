//! Change directory command implementation.
//!
//! This command requires shell integration (`gwtx init`). Without shell integration,
//! it returns an error with instructions. The actual directory change is handled
//! by the shell wrapper function that calls `gwtx path` and `cd`.

use crate::error::{Error, Result};

pub(crate) fn run() -> Result<()> {
    Err(Error::CdRequiresShellIntegration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_returns_shell_integration_required() {
        let result = run();
        assert!(matches!(
            result.unwrap_err(),
            Error::CdRequiresShellIntegration
        ));
    }
}
