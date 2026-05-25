mod cfscanner;
mod handlers;

use anyhow::Result;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;

use crate::SubsCmd;

pub fn handle_subs_ext(
    action: Option<SubsCmd>,
    subs_path: &Path,
    config: &AppConfig,
    db: Option<&rusqlite::Connection>,
    tester: &dyn HttpTester,
) -> Result<()> {
    match action.unwrap_or(SubsCmd::List) {
        SubsCmd::List => handlers::handle_list(subs_path, config),
        SubsCmd::Add => handlers::handle_add(subs_path),
        SubsCmd::Edit { id } => handlers::handle_edit(subs_path, id),
        SubsCmd::Remove { ids } => handlers::handle_remove(subs_path, &ids),
        SubsCmd::Update {
            target,
            info,
            protocol,
            limit,
            keep_raw,
            ..
        } => {
            let t = target.unwrap_or_else(|| "all".to_string());
            handlers::handle_update(
                &t, &protocol, limit, keep_raw, info, config, subs_path, db, tester,
            )
        }
        SubsCmd::Switch { action } => handlers::handle_switch(subs_path, action),
        SubsCmd::Cfscanner { sub_id, args } => cfscanner::handle_cfscanner(sub_id, &args),
    }
}
