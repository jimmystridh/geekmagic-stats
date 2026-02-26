mod render;
mod stats;

use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::Parser;
use geekmagic_common::config;
use geekmagic_common::disk_render;

#[derive(Parser)]
#[command(about = "Render Claude Code usage stats to a GeekMagic display")]
struct Args {
    /// GeekMagic device IP address
    #[arg(long)]
    host: Option<String>,

    /// Path to config file
    #[arg(long)]
    config: Option<String>,

    /// Save rendered image to this path instead of uploading
    #[arg(short, long)]
    output: Option<String>,

    /// Run as daemon, pushing every N seconds
    #[arg(short, long)]
    daemon: Option<u64>,

    /// Also render and upload disk usage screen
    #[arg(long)]
    with_disk: bool,
}

#[derive(Clone)]
struct RuntimeArgs {
    host: String,
    output: Option<String>,
    daemon: Option<u64>,
    with_disk: bool,
}

fn resolve_args(args: Args) -> Result<RuntimeArgs> {
    let cfg = config::load(args.config.as_deref())?;
    let host = args
        .host
        .or(cfg.host)
        .ok_or_else(|| anyhow!("missing host; pass --host or set host in config"))?;

    Ok(RuntimeArgs {
        host,
        output: args.output,
        daemon: args.daemon.or(cfg.daemon),
        with_disk: if args.with_disk {
            true
        } else {
            cfg.with_disk.unwrap_or(false)
        },
    })
}

fn run_once(args: &RuntimeArgs) -> Result<()> {
    let payload = stats::fetch_stats()?;
    let stats_img = render::render_bars(&payload)?;

    if let Some(path) = &args.output {
        stats_img.save(path)?;
        println!("Saved to {path}");
        return Ok(());
    }

    if args.with_disk {
        let disk_info = disk_render::get_disk_info()?;
        let disk_img = disk_render::render_disk(&disk_info)?;

        geekmagic_common::upload::upload_album(
            &args.host,
            &[("stats.jpg", &stats_img), ("disk.jpg", &disk_img)],
        )?;
        let now = chrono::Local::now().format("%H:%M:%S");
        println!("[{now}] Pushed stats + disk to {}", args.host);
    } else {
        geekmagic_common::upload::upload_and_display(&args.host, &stats_img)?;
        let now = chrono::Local::now().format("%H:%M:%S");
        println!("[{now}] Pushed to {}", args.host);
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = resolve_args(Args::parse())?;

    if let Some(interval) = args.daemon {
        let interval = interval.max(10);
        println!("Daemon mode: pushing every {interval}s to {}", args.host);
        loop {
            if let Err(e) = run_once(&args) {
                let now = chrono::Local::now().format("%H:%M:%S");
                eprintln!("[{now}] Error: {e}");
            }
            thread::sleep(Duration::from_secs(interval));
        }
    } else {
        run_once(&args)
    }
}
