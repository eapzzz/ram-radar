mod process;
mod ui;

use std::env;
use crate::process::classifier::ProcessClassifier;
use crate::process::scanner::SystemScanner;
use crate::ui::theme::Theme;
use crate::ui::RamRadarApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Bonus CLI mode if run with --cli, --summary, or -c
    if args.iter().any(|a| a == "--cli" || a == "--summary" || a == "-c") {
        run_cli_summary();
        return Ok(());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RamRadar — Linux PSS RAM & Process Tree Inspector")
            .with_inner_size([1180.0, 800.0])
            .with_min_inner_size([880.0, 560.0])
            .with_active(true)
            .with_decorations(true),
        ..Default::default()
    };

    eframe::run_native(
        "RamRadar",
        native_options,
        Box::new(|cc| Ok(Box::new(RamRadarApp::new(cc)))),
    )
}

fn run_cli_summary() {
    println!("\x1b[1;36m===============================================================\x1b[0m");
    println!("\x1b[1;36m🎯 RamRadar — Realistic Linux RAM Consumption (PSS Metrics)\x1b[0m");
    println!("\x1b[1;36m===============================================================\x1b[0m\n");

    let scanner = SystemScanner::new();
    let (sys_mem, procs) = scanner.scan_all_processes();
    let groups = ProcessClassifier::group_processes(procs);

    println!(
        "Total RAM: \x1b[1;37m{}\x1b[0m | System Used: \x1b[1;33m{}\x1b[0m | Sum PSS: \x1b[1;32m{}\x1b[0m ({:.1}%)",
        Theme::format_kb(sys_mem.total_kb),
        Theme::format_kb(sys_mem.used_kb()),
        Theme::format_kb(sys_mem.total_pss_sum_kb),
        sys_mem.pss_percentage()
    );
    println!(
        "Available: \x1b[1;32m{}\x1b[0m | Swap Used:   \x1b[1;35m{}\x1b[0m / {}\n",
        Theme::format_kb(sys_mem.available_kb),
        Theme::format_kb(sys_mem.swap_used_kb),
        Theme::format_kb(sys_mem.swap_total_kb)
    );

    println!("{:<32} {:<10} {:<15} {:<15} {:<8}", "APPLICATION / GROUP", "PROCS", "REAL (PSS)", "TRADITIONAL (RSS)", "% RAM");
    println!("{:-<86}", "");

    for g in groups.iter().take(20) {
        if g.total_pss_kb == 0 {
            continue;
        }
        let pss_pct = (g.total_pss_kb as f32 / sys_mem.total_kb.max(1) as f32) * 100.0;
        println!(
            "{:<32} {:<10} \x1b[1;36m{:<15}\x1b[0m {:<15} \x1b[1;33m{:>5.1}%\x1b[0m",
            format!("{} {}", g.icon, g.display_name),
            format!("{} procs", g.processes.len()),
            Theme::format_kb(g.total_pss_kb),
            Theme::format_kb(g.total_rss_kb),
            pss_pct
        );
    }
    println!("\n\x1b[90mTo launch the interactive GUI HUD: ./target/release/ram-radar\x1b[0m\n");
}
