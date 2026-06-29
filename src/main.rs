use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
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
    Watch {
        /// Poll interval in milliseconds.
        #[arg(long, default_value_t = 250)]
        interval: u64,
    },
    /// Extract a palette and write the artifacts into a directory with no side
    /// effects (no wallpaper, no live reloads). Sandbox-safe; used to bake a
    /// palette at Nix build time.
    Generate {
        #[arg(value_name = "IMAGE_PATH")]
        image_path: PathBuf,
        /// Directory to write the palette files into (created if missing).
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = astrium::config::Config::load();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let cache_dir = PathBuf::from(&home).join(".cache/astrium");

    let cmd = cli.command.unwrap_or_else(|| Cmd::Apply {
        image_path: cli.image_path.unwrap_or_else(|| PathBuf::from(".")),
        no_wallpaper: false,
    });

    match cmd {
        Cmd::Apply {
            image_path,
            no_wallpaper,
        } => {
            let path = resolve_path(&image_path, &home);
            astrium::apply_with(&path, &cfg, &cache_dir, !no_wallpaper)?;
            println!("ok");
        }
        Cmd::Watch { interval } => watch(&cfg, &cache_dir, interval)?,
        Cmd::Generate { image_path, out } => {
            let path = resolve_path(&image_path, &home);
            astrium::generate(&path, &out, &cfg)?;
            println!("ok");
        }
    }

    Ok(())
}

/// If the user passed a bare filename, fall back to `~/Wallpaper/<name>`.
fn resolve_path(p: &Path, home: &str) -> PathBuf {
    if p.exists() {
        p.to_path_buf()
    } else {
        PathBuf::from(format!("{home}/Wallpaper/{}", p.to_string_lossy()))
    }
}

/// Poll awww in-process and re-theme when the displayed wallpaper changes.
fn watch(cfg: &astrium::config::Config, cache_dir: &Path, interval_ms: u64) -> Result<()> {
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
