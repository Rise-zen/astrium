use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,

    /// Legacy: `astrium X.jpg` still works as a shortcut for `astrium apply X.jpg`.
    #[arg(value_name = "IMAGE_PATH", hide = true)]
    image_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Apply a wallpaper one-shot: set it via awww + write all palette files.
    Apply {
        #[arg(value_name = "IMAGE_PATH")]
        image_path: PathBuf,
        /// Only regenerate the palette; don't set the wallpaper via awww.
        /// Use when the caller already displayed the image itself.
        #[arg(long)]
        no_wallpaper: bool,
    },
    /// Poll awww for wallpaper changes and re-theme automatically.
    /// Replaces the standalone bash wallpaper-watch script with a single
    /// native process that doesn't fork awww+sed+grep every tick.
    Watch {
        /// Poll interval in milliseconds.
        #[arg(long, default_value_t = 250)]
        interval: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = astrium::config::Config::load();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let cache_dir = PathBuf::from(&home).join(".cache/astrium");

    // Resolve the legacy "just give me a path" form.
    let cmd = cli.command.unwrap_or_else(|| Cmd::Apply {
        image_path: cli.image_path.unwrap_or_else(|| PathBuf::from(".")),
        no_wallpaper: false,
    });

    match cmd {
        Cmd::Apply { image_path, no_wallpaper } => {
            let path = resolve_path(&image_path, &home);
            match astrium::apply_with(&path, &cfg, &cache_dir, !no_wallpaper) {
                Ok(_) => println!("ok"),
                Err(e) => eprintln!("[astrium] error: {e:?}"),
            }
        }
        Cmd::Watch { interval } => watch(&cfg, &cache_dir, interval)?,
    }

    Ok(())
}

/// If the user passed a bare filename, fall back to `~/Wallpaper/<name>`.
fn resolve_path(p: &PathBuf, home: &str) -> PathBuf {
    if p.exists() {
        p.clone()
    } else {
        PathBuf::from(format!("{home}/Wallpaper/{}", p.to_string_lossy()))
    }
}

/// Tight polling loop in native Rust. The previous bash version forked
/// awww+grep+sed every interval and slept 250ms; we just call awww and parse
/// in-process. Same external behaviour, lower CPU, no $PATH shell parsing.
fn watch(cfg: &astrium::config::Config, cache_dir: &PathBuf, interval_ms: u64) -> Result<()> {
    let mut last: Option<PathBuf> = None;
    let sleep = Duration::from_millis(interval_ms);
    loop {
        if let Some(p) = astrium::current_wallpaper() {
            if last.as_ref() != Some(&p) && p.exists() {
                if let Err(e) = astrium::apply(&p, cfg, cache_dir) {
                    eprintln!("[astrium] watch: {e:?}");
                }
                last = Some(p);
            }
        }
        thread::sleep(sleep);
    }
}
