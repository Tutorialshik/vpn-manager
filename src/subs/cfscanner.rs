use anyhow::{bail, Context, Result};
use vpn_l10n as l10n;

pub fn handle_cfscanner(sub_id: Option<usize>, args: &[String]) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context(l10n::t("subs.config_dir_missing"))?
        .join("vpn-manager");
    let mut full_args = vec![];
    if let Some(sid) = sub_id {
        let proxy_config = config_dir.join(format!("sub_{}_live.txt", sid));
        if proxy_config.exists() {
            full_args.push("-C".to_string());
            full_args.push(proxy_config.to_string_lossy().into());
        } else {
            bail!(l10n::t_fmt("subs.subs_not_found", &[&sid.to_string()]));
        }
    }
    full_args.extend(args.to_vec());
    vpn_knife::run_knife("cfscanner", &full_args)
}
