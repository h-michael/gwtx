use crate::config::RawConfig;
use crate::error::Result;

/// Generate JSON Schema for configuration.
pub(crate) fn run() -> Result<()> {
    let schema = schemars::schema_for!(RawConfig);
    let json = serde_json::to_string_pretty(&schema)
        .expect("Failed to serialize schema (this should never happen)");
    println!("{json}");
    Ok(())
}
