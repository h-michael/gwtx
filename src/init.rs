use crate::cli::InitArgs;
use crate::error::{Error, Result};

use clap_complete::Shell;

use std::env;
use std::path::PathBuf;

const KABU_PATH_PLACEHOLDER: &str = "::KABU::";

const BASH_INIT: &str = include_str!("init/kabu.bash");
const ZSH_INIT: &str = include_str!("init/kabu.zsh");
const FISH_INIT: &str = include_str!("init/kabu.fish");
const POWERSHELL_INIT: &str = include_str!("init/kabu.ps1");
const ELVISH_INIT: &str = include_str!("init/kabu.elv");

pub(crate) fn run(args: InitArgs) -> Result<()> {
    if args.print_full_init {
        let script = init_main(args.shell)?;
        print!("{script}");
    } else {
        let script = init_stub(args.shell)?;
        print!("{script}");
    }
    Ok(())
}

fn init_stub(shell: Shell) -> Result<String> {
    let kabu = KabuPath::init()?;
    let stub = match shell {
        Shell::Bash => format!(
            r#"eval -- "$({} init bash --print-full-init)""#,
            kabu.sprint_posix()?
        ),
        Shell::Zsh => format!(
            r#"source <({} init zsh --print-full-init)"#,
            kabu.sprint_posix()?
        ),
        Shell::Fish => format!(
            r#"source ({} init fish --print-full-init | psub)"#,
            kabu.sprint_posix()?
        ),
        Shell::PowerShell => format!(
            r#"Invoke-Expression (& {} init powershell --print-full-init | Out-String)"#,
            kabu.sprint_pwsh()?
        ),
        Shell::Elvish => format!(
            r#"eval ({} init elvish --print-full-init | slurp)"#,
            kabu.sprint_posix()?
        ),
        _ => {
            return Err(Error::Internal(format!(
                "Unsupported shell for init: {shell:?}"
            )));
        }
    };
    Ok(stub)
}

fn init_main(shell: Shell) -> Result<String> {
    let kabu = KabuPath::init()?;
    let script = match shell {
        Shell::Bash => print_script(BASH_INIT, &kabu.sprint_posix()?),
        Shell::Zsh => print_script(ZSH_INIT, &kabu.sprint_posix()?),
        Shell::Fish => print_script(FISH_INIT, &kabu.sprint_posix()?),
        Shell::PowerShell => print_script(POWERSHELL_INIT, &kabu.sprint_pwsh()?),
        Shell::Elvish => print_script(ELVISH_INIT, &kabu.sprint_posix()?),
        _ => {
            return Err(Error::Internal(format!(
                "Unsupported shell for init: {shell:?}"
            )));
        }
    };
    Ok(script)
}

fn print_script(script: &str, path: &str) -> String {
    script.replace(KABU_PATH_PLACEHOLDER, path)
}

struct KabuPath {
    native_path: PathBuf,
}

impl KabuPath {
    fn init() -> Result<Self> {
        let exe = env::current_exe().map_err(|e| {
            Error::Internal(format!("Failed to determine kabu executable path: {}", e))
        })?;
        Ok(Self { native_path: exe })
    }

    fn str_path(&self) -> Result<&str> {
        self.native_path
            .to_str()
            .ok_or_else(|| Error::Internal("Failed to convert kabu path to string".to_string()))
    }

    fn sprint_posix(&self) -> Result<String> {
        let path = self.str_path()?;
        Ok(posix_quote(path))
    }

    fn sprint_pwsh(&self) -> Result<String> {
        let path = self.str_path()?;
        Ok(format!("'{}'", path.replace('\'', "''")))
    }
}

fn posix_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    if !input.contains('\'') {
        return format!("'{input}'");
    }
    let mut out = String::with_capacity(input.len() + 2);
    out.push('\'');
    for ch in input.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posix_quote_empty_string() {
        assert_eq!(posix_quote(""), "''");
    }

    #[test]
    fn test_posix_quote_no_special_chars() {
        assert_eq!(posix_quote("/usr/bin/kabu"), "'/usr/bin/kabu'");
    }

    #[test]
    fn test_posix_quote_single_quote() {
        assert_eq!(posix_quote("it's"), "'it'\"'\"'s'");
    }

    #[test]
    fn test_posix_quote_multiple_single_quotes() {
        assert_eq!(posix_quote("'a'b'"), "''\"'\"'a'\"'\"'b'\"'\"''");
    }

    #[test]
    fn test_posix_quote_mixed_special_chars() {
        assert_eq!(
            posix_quote("/path/to/'file'"),
            "'/path/to/'\"'\"'file'\"'\"''"
        );
    }
}
