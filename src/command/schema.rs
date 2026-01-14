use crate::config::RawConfig;
use crate::error::Result;

/// Generate JSON Schema for configuration.
pub(crate) fn run() -> Result<()> {
    let schema = schemars::schema_for!(RawConfig);
    let json = serde_json::to_string_pretty(&schema)
        .map_err(|e| crate::error::Error::Internal(format!("Failed to serialize schema: {}", e)))?;
    println!("{json}");
    Ok(())
}
