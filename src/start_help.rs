use std::fs;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_subs::crud;

pub fn show_start_help(config: &AppConfig, subs_path: &Path) {
    println!("═════════════════ start – запуск прокси ═════════════════");
    println!(" vpn-manager start [menu] это меню");
    println!(" vpn-manager start now последний профиль");
    println!(" vpn-manager start region <REGION> регион (ru, eu, de…)");
    println!(" vpn-manager start sub <ID> подписка по ID");
    println!(" vpn-manager start exec <cmd...> выполнить в неймспейсе");
    println!(" Флаги:");
    println!(" -m, --method <random|fastest> метод выбора");
    println!(" -r, --rotate <S> интервал ротации");
    println!(" все флаги xray-knife proxy также принимаются");
    println!("Регионы:");
    if let Some(config_dir) = dirs::config_dir() {
        let lists_dir = config_dir.join("vpn-manager").join("lists");
        for region in &["ru", "us", "eu", "de", "pl", "fi", "nl", "other"] {
            let file = lists_dir.join(format!("{}.txt", region));
            let count = if file.exists() {
                fs::read_to_string(&file)
                    .map(|s| s.lines().count())
                    .unwrap_or(0)
            } else {
                0
            };
            println!(" {:<6} – {:>6} живых", region, count);
        }
    } else {
        println!("Не удалось получить путь к конфигурации.");
    }
    println!("Подписки:");
    crud::list_subscriptions(subs_path, config);
    println!("════════════════════════════════════════════════════════");
}
