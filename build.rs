use std::env;
use std::process::Command;

fn main() {
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let git_hash = git_output(&["rev-parse", "HEAD"]);
    let short_hash = git_hash
        .as_deref()
        .map(shorten_hash)
        .unwrap_or("unknown".to_string());

    println!("cargo:rustc-env=GWTX_GIT_HASH={short_hash}");

    let exact_tag = git_output(&["describe", "--tags", "--exact-match", "HEAD"])
        .filter(|tag| tag == &format!("v{version}"));

    let version_label = match (&exact_tag, short_hash.as_str()) {
        (Some(_), _) => format!("v{version}"),
        (None, "unknown") => format!("v{version}"),
        (None, _) => format!("v{version} nightly {short_hash}"),
    };

    println!("cargo:rustc-env=GWTX_VERSION_LABEL={version_label}");

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn shorten_hash(value: &str) -> String {
    value.chars().take(7).collect()
}
