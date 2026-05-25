use crate::knife;
use crate::l10n;
use crate::SubsCmd;
use crate::SwitchCmd;
use anyhow::{bail, Context, Result};
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;
use vpn_subs::crud;

pub fn handle_subs_ext(
    action: Option<SubsCmd>,
    subs_path: &Path,
    config: &AppConfig,
    db: Option<&rusqlite::Connection>,
    tester: &dyn HttpTester,
) -> Result<()> {
    match action.unwrap_or(SubsCmd::List) {
        SubsCmd::List => {
            crud::list_subscriptions(subs_path, config);
            println!("{}", l10n::t("subs.help"));
            Ok(())
        }
        SubsCmd::Add => crud::add_subscription(subs_path),
        SubsCmd::Edit { id } => crud::edit_subscription(subs_path, id),
        SubsCmd::Remove { ids } => crud::remove_subscriptions(subs_path, &ids),
        SubsCmd::Update {
            target,
            info,
            protocol,
            limit,
            keep_raw,
            ..
        } => {
            let t = target.unwrap_or_else(|| "all".to_string());
            crate::update::handle_update(
                &t, &protocol, limit, keep_raw, info, config, subs_path, db, tester,
            )?;
            Ok(())
        }
        SubsCmd::Switch { action } => match action {
            SwitchCmd::On { ids } => crud::switch_subscriptions(subs_path, &ids, true),
            SwitchCmd::Off { ids } => crud::switch_subscriptions(subs_path, &ids, false),
        },
        SubsCmd::Cfscanner { sub_id, args } => {
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
            full_args.extend(args);
            knife::run_knife("cfscanner", &full_args)
        }
    }
}
