use anyhow::{bail, Result};
use vpn_l10n as l10n;

pub fn validate_method(method: &str) -> Result<()> {
    if method != "random" && method != "fastest" {
        bail!(l10n::t("settings.invalid_method"));
    }
    Ok(())
}
