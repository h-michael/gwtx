/// Output manager that respects quiet mode.
#[derive(Debug, Clone)]
pub(crate) struct Output {
    quiet: bool,
}

impl Output {
    /// Create a new Output instance.
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    /// Print an info message (suppressed in quiet mode).
    #[allow(dead_code)]
    pub fn info(&self, message: &str) {
        if !self.quiet {
            println!("{message}");
        }
    }

    /// Print a success message (suppressed in quiet mode).
    pub fn success(&self, message: &str) {
        if !self.quiet {
            println!("{message}");
        }
    }

    /// Print a warning message (always shown).
    #[allow(dead_code)]
    pub fn warn(&self, message: &str) {
        eprintln!("[warn] {message}");
    }

    /// Print mkdir operation.
    pub fn mkdir(&self, path: &std::path::Path, description: Option<&str>) {
        if self.quiet {
            return;
        }
        match description {
            Some(desc) => println!("Creating: {} ({desc})", path.display()),
            None => println!("Creating: {}", path.display()),
        }
    }

    /// Print symlink operation.
    pub fn link(
        &self,
        source: &std::path::Path,
        target: &std::path::Path,
        description: Option<&str>,
    ) {
        if self.quiet {
            return;
        }
        let source_str = source.file_name().unwrap_or_default().to_string_lossy();
        let target_str = target.file_name().unwrap_or_default().to_string_lossy();

        if source_str == target_str {
            match description {
                Some(desc) => println!("Linking: {source_str} ({desc})"),
                None => println!("Linking: {source_str}"),
            }
        } else {
            match description {
                Some(desc) => println!("Linking: {source_str} → {target_str} ({desc})"),
                None => println!("Linking: {source_str} → {target_str}"),
            }
        }
    }

    /// Print copy operation.
    pub fn copy(
        &self,
        source: &std::path::Path,
        target: &std::path::Path,
        description: Option<&str>,
    ) {
        if self.quiet {
            return;
        }
        let source_str = source.file_name().unwrap_or_default().to_string_lossy();
        let target_str = target.file_name().unwrap_or_default().to_string_lossy();

        if source_str == target_str {
            match description {
                Some(desc) => println!("Copying: {source_str} ({desc})"),
                None => println!("Copying: {source_str}"),
            }
        } else {
            match description {
                Some(desc) => println!("Copying: {source_str} → {target_str} ({desc})"),
                None => println!("Copying: {source_str} → {target_str}"),
            }
        }
    }

    /// Print skip message.
    pub fn skip(&self, path: &std::path::Path) {
        if !self.quiet {
            println!("Skipped: {} (conflict)", path.display());
        }
    }

    /// Print dry-run message.
    pub fn dry_run(&self, message: &str) {
        if !self.quiet {
            println!("[dry-run] {message}");
        }
    }
}
