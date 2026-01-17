use crate::error::{Error, Result};

pub(crate) fn run() -> Result<()> {
    Err(Error::SwitchRequiresShellIntegration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_returns_shell_integration_required() {
        let result = run();
        assert!(matches!(
            result.unwrap_err(),
            Error::SwitchRequiresShellIntegration
        ));
    }
}
