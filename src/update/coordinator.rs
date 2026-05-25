use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::Connection;
use std::fs;
// use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;
use vpn_core::types::Subscription;
use vpn_core::utils;
use vpn_l10n as l10n;
use vpn_subs::crud;
use vpn_subs::update as subs_update;

use super::classifier::classify_configs;
use super::stats::record_stats;
use super::UpdateParams;

#[allow(clippy::too_many_arguments)]
pub fn handle_update(params: UpdateParams) -> Result<()> {
    let UpdateParams {
        target,
        protocol,
        limit,
        keep_raw,
        show_info,
        config,
        subs_path,
        db,
        tester,
    } = params;

    let subs = crud::load_subs(subs_path)?;
    let ids = if target == "all" {
        subs.iter().map(|s| s.id).collect()
    } else {
        utils::expand_ids(target)?
    };
    for id in ids {
        if let Some(sub) = subs.iter().find(|s| s.id == id) {
            update_single(
                sub, protocol, limit, keep_raw, show_info, config, db, tester,
            )?;
        } else {
            eprintln!("{}", l10n::t_fmt("subs.subs_not_found", &[&id.to_string()]));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_single(
    sub: &Subscription,
    proto: &str,
    limit: usize,
    keep_raw: bool,
    show_info: bool,
    config: &AppConfig,
    db: Option<&Connection>,
    tester: &dyn HttpTester,
) -> Result<()> {
    println!(
        "{}",
        l10n::t_fmt("subs.update_started", &[&sub.id.to_string(), &sub.name])
    );

    let config_dir = dirs::config_dir()
        .context(l10n::t("proxy.config_dir_missing"))?
        .join("vpn-manager");

    let live_files =
        subs_update::update_single_sub(sub, proto, limit, keep_raw, show_info, config, tester)?;

    let merged = format!("/tmp/vpn-sub-{}-live-merged.txt", sub.id);
    utils::merge_files(&live_files, &merged)?;
    let dest = config_dir.join(format!("sub_{}_live.txt", sub.id));
    fs::copy(&merged, &dest)?;
    let ts = Local::now().format("%Y-%m-%d %H:%M").to_string();
    fs::write(config_dir.join(format!("sub_{}_timestamp.txt", sub.id)), ts)?;

    if let Some(conn) = db {
        let active_urls = utils::get_active_urls(config);
        record_stats(sub.id, &active_urls, config, conn)?;
    }

    let mut all_live_content = String::new();
    for entry in fs::read_dir(&config_dir)? {
        let entry = entry?;
        let fname = entry.file_name();
        if let Some(name) = fname.to_str() {
            if name.starts_with("sub_") && name.ends_with("_live.txt") {
                if let Ok(data) = fs::read_to_string(entry.path()) {
                    all_live_content.push_str(&data);
                }
            }
        }
    }
    let all_live = config_dir.join("all_live_merged.txt");
    fs::write(&all_live, utils::unique_lines(&all_live_content))?;
    classify_configs(&all_live, config)?;

    if !keep_raw {
        let _ = fs::remove_file(format!("/tmp/vpn-sub-{}-raw.txt", sub.id));
        let _ = fs::remove_file(format!("/tmp/vpn-sub-{}-filtered.txt", sub.id));
        let _ = fs::remove_file(&merged);
        for f in &live_files {
            let _ = fs::remove_file(f);
        }
    }

    println!(
        "{}",
        l10n::t_fmt("subs.update_finished", &[&sub.id.to_string()])
    );
    Ok(())
}
