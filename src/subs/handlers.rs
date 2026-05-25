use anyhow::Result;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;
use vpn_l10n as l10n;
use vpn_subs::crud;

use crate::update::{handle_update as core_handle_update, UpdateParams};
use crate::SwitchCmd;

pub fn handle_list(subs_path: &Path, config: &AppConfig) -> Result<()> {
    crud::list_subscriptions(subs_path, config);
    println!("{}", l10n::t("subs.help"));
    Ok(())
}

pub fn handle_add(subs_path: &Path) -> Result<()> {
    crud::add_subscription(subs_path)
}

pub fn handle_edit(subs_path: &Path, id: usize) -> Result<()> {
    crud::edit_subscription(subs_path, id)
}

pub fn handle_remove(subs_path: &Path, ids: &str) -> Result<()> {
    crud::remove_subscriptions(subs_path, ids)
}

pub fn handle_switch(subs_path: &Path, action: SwitchCmd) -> Result<()> {
    match action {
        SwitchCmd::On { ids } => crud::switch_subscriptions(subs_path, &ids, true),
        SwitchCmd::Off { ids } => crud::switch_subscriptions(subs_path, &ids, false),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_update(
    target: &str,
    protocol: &str,
    limit: usize,
    keep_raw: bool,
    info: bool,
    config: &AppConfig,
    subs_path: &Path,
    db: Option<&rusqlite::Connection>,
    tester: &dyn HttpTester,
) -> Result<()> {
    let params = UpdateParams {
        target,
        protocol,
        limit,
        keep_raw,
        show_info: info,
        config,
        subs_path,
        db,
        tester,
    };
    core_handle_update(params)
}
