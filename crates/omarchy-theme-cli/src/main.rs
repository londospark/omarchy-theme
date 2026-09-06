//! omarchy-theme — export the live (or named) Omarchy theme for any toolkit.
//!
//! Formats: json (canonical manifest), css, toml, c-header, imgui.hpp.
//! Also `watch -- <cmd>`: run a command on every theme change (the hook-free
//! alternative), and `current`/`list` for scripts.

use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use clap::{Parser, Subcommand, ValueEnum};
use omarchy_theme::{export, Theme};

#[derive(Parser)]
#[command(name = "omarchy-theme", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the current theme slug and mode
    Current,
    /// List installed themes (stock + user)
    List,
    /// Export the theme in a given format
    Export {
        #[arg(short, long, value_enum, default_value = "json")]
        format: Format,
        /// Export a named theme instead of the live one (no font/background)
        #[arg(short, long)]
        theme: Option<String>,
        /// Output file (default: stdout)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Run a command once now and again on every theme change (foreground)
    Watch {
        #[arg(last = true)]
        cmd: Vec<String>,
    },
}

#[derive(Clone, ValueEnum)]
enum Format {
    Json,
    Css,
    Toml,
    #[value(name = "c-header")]
    CHeader,
    #[value(name = "imgui.hpp")]
    ImguiHpp,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Current => {
            let theme = Theme::current()?;
            let name = theme.name.clone().unwrap_or_else(|| "unknown".into());
            println!("{name}\t{}", theme.palette.mode());
        }
        Command::List => {
            // user themes shadow stock; later dirs override earlier ones' slugs
            let mut seen: std::collections::BTreeMap<String, ()> = Default::default();
            let dirs = omarchy_theme::theme_search_dirs();
            for dir in dirs.iter().rev() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for e in entries.flatten() {
                        if e.path().join("colors.toml").is_file() {
                            seen.insert(e.file_name().to_string_lossy().into_owned(), ());
                        }
                    }
                }
            }
            for slug in seen.keys() {
                println!("{slug}");
            }
        }
        Command::Export { format, theme: Some(slug), out } => {
            let source = omarchy_theme::find_theme(&slug).ok_or("theme not found")?;
            let manifest = export::Manifest::from_theme(&source.load()?);
            write(out, render(format, &manifest)?)?;
        }
        Command::Export { format, theme: None, out } => {
            let theme = Theme::current()?.with_font();
            let manifest = export::Manifest::from_theme(&theme);
            write(out, render(format, &manifest)?)?;
        }
        Command::Watch { cmd } => {
            if cmd.is_empty() {
                return Err("usage: omarchy-theme watch -- <cmd> [args...]".into());
            }
            run(&cmd)?;
            let mut theme = Theme::current()?;
            let mut pending = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if theme.changed() {
                    // debounce the swap burst; changed() already ignores
                    // the half-state, this covers rapid repeat switches
                    if pending.elapsed().as_millis() < 500 {
                        continue;
                    }
                    pending = std::time::Instant::now();
                    if theme.reload()? {
                        run(&cmd)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn render(format: Format, m: &export::Manifest) -> Result<String, Box<dyn std::error::Error>> {
    Ok(match format {
        Format::Json => export::to_json(m),
        Format::Css => export::to_css(m),
        Format::Toml => export::to_toml(m),
        Format::CHeader => export::to_c_header(m),
        Format::ImguiHpp => export::to_imgui_hpp(m),
    })
}

fn write(out: Option<PathBuf>, content: String) -> Result<(), Box<dyn std::error::Error>> {
    match out {
        Some(path) => {
            // write atomically (tmp + rename) so readers never see half files
            let tmp = path.with_extension("tmp");
            std::fs::write(&tmp, content)?;
            std::fs::rename(tmp, path)?;
        }
        None => {
            std::io::stdout().write_all(content.as_bytes())?;
        }
    }
    Ok(())
}

fn run(cmd: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let status = std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::null())
        .status()?;
    if !status.success() {
        eprintln!("omarchy-theme watch: {} exited {}", cmd[0], status);
    }
    Ok(())
}
