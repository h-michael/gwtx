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

    /// Print a success message (suppressed in quiet mode).
    pub fn success(&self, message: &str) {
        if !self.quiet {
            println!("{message}");
        }
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

    /// Print file operation (link or copy).
    fn print_file_op(
        &self,
        op: &str,
        source: &std::path::Path,
        target: &std::path::Path,
        description: Option<&str>,
    ) {
        if self.quiet {
            return;
        }
        let source_str = source.file_name().unwrap_or_default().to_string_lossy();
        let target_str = target.file_name().unwrap_or_default().to_string_lossy();

        let base = if source_str == target_str {
            format!("{op}: {source_str}")
        } else {
            format!("{op}: {source_str} → {target_str}")
        };

        match description {
            Some(desc) => println!("{base} ({desc})"),
            None => println!("{base}"),
        }
    }

    /// Print symlink operation.
    pub fn link(
        &self,
        source: &std::path::Path,
        target: &std::path::Path,
        description: Option<&str>,
    ) {
        self.print_file_op("Linking", source, target, description);
    }

    /// Print copy operation.
    pub fn copy(
        &self,
        source: &std::path::Path,
        target: &std::path::Path,
        description: Option<&str>,
    ) {
        self.print_file_op("Copying", source, target, description);
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

    /// Print worktree removal message.
    pub fn remove(&self, path: &std::path::Path) {
        if !self.quiet {
            println!("Removed: {}", path.display());
        }
    }

    /// Print safety warning.
    pub fn safety_warning(&self, path: &std::path::Path, message: &str) {
        if !self.quiet {
            println!("Warning: {} - {}", path.display(), message);
        }
    }
}
