mod actions;
mod desktop;
mod model;
mod process;
mod telemetry;
mod ui;
use std::env;
fn main() -> eframe::Result<()> {
    let args: Vec<_> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Still — native Linux system monitor\n\nUsage: still [--summary | --json | --version]\n\n  --summary    Print a live resource summary\n  --json       Print a machine-readable snapshot\n  --version    Show version\n\nThe default opens the desktop app. No root required.");
        return Ok(());
    }
    if args.iter().any(|a| a == "--version") {
        println!("Still {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--summary" | "--cli" | "-c" | "--json"))
    {
        summary(args.iter().any(|a| a == "--json"));
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Still — System Monitor")
            .with_app_id("still")
            .with_inner_size([1280.0, 880.0])
            .with_min_inner_size([850.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "still",
        options,
        Box::new(|cc| Ok(Box::new(ui::StillApp::new(cc)))),
    )
}
fn summary(json: bool) {
    let mut collector = telemetry::Collector::new();
    let scanner = process::scanner::SystemScanner::new();
    collector.sample();
    scanner.scan_all_processes();
    std::thread::sleep(std::time::Duration::from_millis(250));
    let h = collector.sample();
    let (m, p) = scanner.scan_all_processes();
    let registry = desktop::DesktopRegistry::load();
    let groups = process::classifier::ProcessClassifier::group_processes(p, &registry);
    if json {
        let value = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"cpu_percent":h.cpu,"memory_total_kib":m.total_kb,"memory_used_kib":m.used_kb(),"memory_available_kib":m.available_kb,"swap_used_kib":m.swap_used_kb,"download_bytes_per_second":h.down(),"upload_bytes_per_second":h.up(),"gpu":h.gpus.iter().map(|g|serde_json::json!({"name":g.name,"driver":g.driver,"busy_percent":g.busy,"vram_used_bytes":g.used,"vram_total_bytes":g.total})).collect::<Vec<_>>(),"applications":groups.iter().map(|g|serde_json::json!({"name":g.display_name,"executable":g.key,"pss_kib":g.total_pss_kb,"cpu_percent":g.total_cpu,"processes":g.processes.len(),"estimated_processes":g.estimated_procs})).collect::<Vec<_>>()});
        println!("{}", serde_json::to_string_pretty(&value).unwrap());
    } else {
        println!(
            "Still {}\nCPU {:.1}%  |  RAM {:.2} / {:.2} GiB  |  {} logical cores\n",
            env!("CARGO_PKG_VERSION"),
            h.cpu,
            m.used_kb() as f64 / 1048576.0,
            m.total_kb as f64 / 1048576.0,
            h.cores.len()
        );
        println!(
            "{:<32} {:>12} {:>9} {:>7}",
            "APPLICATION", "PSS MiB", "CPU %", "PIDS"
        );
        for g in groups.iter().take(20) {
            println!(
                "{:<32} {}{:>10.1} {:>9.1} {:>7}",
                g.display_name,
                if g.estimated_procs > 0 { "~" } else { " " },
                g.total_pss_kb as f64 / 1024.0,
                g.total_cpu,
                g.processes.len()
            );
        }
        println!("\n~ RSS estimate where PSS is inaccessible. CPU: 100% = one logical core.");
    }
}
