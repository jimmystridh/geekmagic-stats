use anyhow::{anyhow, Result};
use clap::Parser;
use geekmagic_common::config;
use geekmagic_common::disk_render;

#[derive(Parser)]
#[command(about = "Render disk usage pie chart to a GeekMagic display")]
struct Args {
    #[arg(long)]
    host: Option<String>,

    /// Path to config file
    #[arg(long)]
    config: Option<String>,

    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let cfg = config::load(args.config.as_deref())?;
    let host = args
        .host
        .or(cfg.host)
        .ok_or_else(|| anyhow!("missing host; pass --host or set host in config"))?;
    let info = disk_render::get_disk_info()?;

    println!(
        "Disk: {} total, {} used, {} free ({:.1}%)",
        disk_render::format_size(info.total_bytes),
        disk_render::format_size(info.used_bytes),
        disk_render::format_size(info.free_bytes),
        info.free_bytes as f64 / info.total_bytes as f64 * 100.0,
    );

    let img = disk_render::render_disk(&info)?;

    if let Some(path) = &args.output {
        img.save(path)?;
        println!("Saved to {path}");
    } else {
        geekmagic_common::upload::upload_and_display(&host, &img)?;
        println!("Pushed to {host}");
    }

    Ok(())
}
