use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use codexbar_core::WidgetSnapshot;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Parser)]
#[command(name = "codexbar-service")]
#[command(about = "Rust service layer for KDE Plasma widget consumption")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Snapshot(SnapshotArgs),
}

#[derive(Debug, Parser, Clone)]
struct SnapshotArgs {
    #[arg(long, default_value_t = false)]
    pretty: bool,

    #[arg(long, default_value_t = false)]
    from_codexbar_cli: bool,

    #[arg(long, default_value = "all")]
    provider: String,

    #[arg(long, default_value_t = false)]
    status: bool,

    #[arg(long)]
    input: Option<PathBuf>,

    #[arg(long)]
    write_cache: Option<PathBuf>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("codexbar-service: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::Snapshot(SnapshotArgs {
        pretty: false,
        from_codexbar_cli: true,
        provider: "all".to_string(),
        status: true,
        input: None,
        write_cache: None,
    }));

    match command {
        Commands::Snapshot(args) => render_snapshot(&args),
    }
}

fn render_snapshot(args: &SnapshotArgs) -> Result<()> {
    let snapshot = build_snapshot(args)?;
    let json = if args.pretty {
        serde_json::to_string_pretty(&snapshot)?
    } else {
        serde_json::to_string(&snapshot)?
    };

    if let Some(cache_path) = args.write_cache.as_ref() {
        write_cache_file(cache_path, &json)?;
    }

    println!("{json}");
    Ok(())
}

fn build_snapshot(args: &SnapshotArgs) -> Result<WidgetSnapshot> {
    if let Some(path) = args.input.as_ref() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read JSON input from {}", path.display()))?;
        let values = parse_json_values(&raw)?;
        return Ok(WidgetSnapshot::from_codexbar_cli_values(&values));
    }

    if args.from_codexbar_cli {
        return fetch_from_codexbar_cli(&args.provider, args.status);
    }

    bail!("no live data source selected; pass --from-codexbar-cli or --input <path>")
}

fn fetch_from_codexbar_cli(provider: &str, status: bool) -> Result<WidgetSnapshot> {
    let output = if let Some(sibling) = sibling_codexbar_path() {
        run_codexbar_command(&sibling, provider, status)
            .with_context(|| format!("failed to spawn codexbar CLI at {}", sibling.display()))?
    } else {
        run_codexbar_command(Path::new("codexbar"), provider, status)
            .with_context(|| "failed to spawn codexbar CLI".to_string())?
    };

    if !output.status.success() {
        bail!(
            "codexbar CLI exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8(output.stdout).context("codexbar stdout was not valid UTF-8")?;
    let values = parse_json_values(&stdout).context("failed to decode codexbar JSON payload")?;
    Ok(WidgetSnapshot::from_codexbar_cli_values(&values))
}

fn run_codexbar_command(program: &Path, provider: &str, status: bool) -> std::io::Result<Output> {
    let mut command = Command::new(program);
    command
        .arg("usage")
        .arg("--format")
        .arg("json")
        .arg("--provider")
        .arg(provider)
        .arg("--source")
        .arg("auto");

    if status {
        command.arg("--status");
    }

    command.output()
}

fn sibling_codexbar_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("codexbar");

    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn parse_json_values(raw: &str) -> Result<Vec<Value>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("empty JSON payload");
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(match value {
            Value::Array(items) => items,
            Value::Object(_) => vec![value],
            _ => {
                bail!("JSON payload must be an object or an array");
            }
        });
    }

    let line_values = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();

    if line_values.is_empty() {
        bail!("unable to parse payload as JSON");
    }

    Ok(line_values)
}

fn write_cache_file(path: &PathBuf, payload: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(path, payload).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}
