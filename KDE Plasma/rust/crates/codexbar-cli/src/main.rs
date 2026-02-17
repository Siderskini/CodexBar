use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use codexbar_core::{now_iso8601, IdentityInfo, ProviderEntry, RateWindow, StatusInfo};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(name = "codexbar")]
#[command(about = "Rust CodexBar CLI (Linux-first bootstrap)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Usage(UsageArgs),
    Auth(AuthArgs),
}

#[derive(Debug, Parser, Clone)]
struct UsageArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    #[arg(long, default_value = "all")]
    provider: String,

    #[arg(long, default_value = "auto")]
    source: String,

    #[arg(long, default_value_t = false)]
    status: bool,

    #[arg(long, default_value_t = false)]
    pretty: bool,
}

#[derive(Debug, Parser, Clone)]
struct AuthArgs {
    #[arg(long, default_value = "claude")]
    provider: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

impl Default for UsageArgs {
    fn default() -> Self {
        Self {
            format: OutputFormat::Text,
            provider: "all".to_string(),
            source: "auto".to_string(),
            status: false,
            pretty: false,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("codexbar: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::Usage(UsageArgs::default()));

    match command {
        Commands::Usage(args) => run_usage(&args),
        Commands::Auth(args) => run_auth(&args),
    }
}

fn run_auth(args: &AuthArgs) -> Result<()> {
    match args.provider.trim().to_ascii_lowercase().as_str() {
        "claude" => run_claude_auth(),
        other => bail!("unsupported auth provider '{other}'"),
    }
}

fn run_claude_auth() -> Result<()> {
    println!("Starting Claude browser login...");
    let status = Command::new("claude")
        .arg("auth")
        .arg("login")
        .status()
        .context("failed to launch `claude auth login`; ensure Claude CLI is installed")?;

    if !status.success() {
        bail!("`claude auth login` exited with status {status}");
    }

    if let Some(access_token) = load_claude_oauth_access_token_from_credentials_file()
        .or_else(resolve_claude_oauth_access_token)
    {
        if let Err(error) = store_claude_secret(
            "oauth_access_token",
            "CodexBar Claude OAuth Access Token",
            &access_token,
        ) {
            eprintln!("codexbar: warning: unable to cache OAuth token in keyring: {error:#}");
        }
    }

    println!("Claude browser login complete. CodexBar will use OAuth usage data.");
    Ok(())
}

fn run_usage(args: &UsageArgs) -> Result<()> {
    let entries = selected_entries(args)?;

    match args.format {
        OutputFormat::Json => {
            let payload = entries
                .iter()
                .map(|entry| cli_payload(entry, args))
                .collect::<Vec<_>>();

            if args.pretty {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{}", serde_json::to_string(&payload)?);
            }
        }
        OutputFormat::Text => {
            print_text(entries);
        }
    }

    Ok(())
}

fn selected_entries(args: &UsageArgs) -> Result<Vec<ProviderEntry>> {
    let providers = requested_providers(&args.provider)?;
    let mut entries = Vec::with_capacity(providers.len());

    for provider in providers {
        let live = match fetch_live_entry(provider, args) {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("codexbar: provider '{provider}' live fetch failed: {error:#}");
                None
            }
        };

        if let Some(entry) = live {
            entries.push(entry);
        } else {
            eprintln!("codexbar: provider '{provider}' has no live usage data");
        }
    }

    if entries.is_empty() {
        bail!(
            "no live usage data available for provider '{}'; ensure corresponding CLI tools are installed and authenticated",
            args.provider
        );
    }

    Ok(entries)
}

fn cli_payload(entry: &ProviderEntry, args: &UsageArgs) -> Value {
    let resolved_source = if args.source.eq_ignore_ascii_case("auto") {
        entry.source.as_deref().unwrap_or("rust").to_string()
    } else {
        args.source.clone()
    };

    let identity_payload = entry
        .identity
        .as_ref()
        .map(|identity| {
            json!({
                "providerID": entry.provider,
                "accountEmail": identity.account_email,
                "accountOrganization": identity.account_organization,
                "loginMethod": identity.login_method,
            })
        })
        .unwrap_or(Value::Null);

    let usage = json!({
        "primary": rate_window_value(entry.primary.as_ref()),
        "secondary": rate_window_value(entry.secondary.as_ref()),
        "tertiary": rate_window_value(entry.tertiary.as_ref()),
        "updatedAt": entry.updated_at,
        "identity": identity_payload,
        "accountEmail": entry.identity.as_ref().and_then(|identity| identity.account_email.clone()),
        "accountOrganization": entry.identity.as_ref().and_then(|identity| identity.account_organization.clone()),
        "loginMethod": entry.identity.as_ref().and_then(|identity| identity.login_method.clone()),
    });

    let credits = entry
        .credits_remaining
        .map(|remaining| {
            json!({
                "remaining": remaining,
                "updatedAt": entry.updated_at
            })
        })
        .unwrap_or(Value::Null);

    let openai_dashboard = entry
        .code_review_remaining_percent
        .map(|remaining| {
            json!({
                "codeReviewRemainingPercent": remaining,
                "updatedAt": entry.updated_at
            })
        })
        .unwrap_or(Value::Null);

    let status = if args.status {
        entry
            .status
            .as_ref()
            .map(|status| {
                json!({
                    "indicator": status.indicator,
                    "description": status.description,
                    "updatedAt": status.updated_at,
                    "url": status.url
                })
            })
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    json!({
        "provider": entry.provider,
        "version": env!("CARGO_PKG_VERSION"),
        "source": resolved_source,
        "status": status,
        "usage": usage,
        "credits": credits,
        "antigravityPlanInfo": Value::Null,
        "openaiDashboard": openai_dashboard
    })
}

fn rate_window_value(window: Option<&RateWindow>) -> Value {
    match window {
        Some(window) => json!({
            "usedPercent": window.used_percent,
            "windowMinutes": window.window_minutes,
            "resetsAt": window.resets_at,
        }),
        None => Value::Null,
    }
}

fn print_text(entries: Vec<ProviderEntry>) {
    for entry in entries {
        let session_left = remaining_percent(entry.primary.as_ref());
        let weekly_left = remaining_percent(entry.secondary.as_ref());

        println!(
            "== {} ({}) ==",
            entry.provider,
            entry.source.unwrap_or_else(|| "rust".to_string())
        );
        println!("Session: {}", format_percent(session_left));
        println!("Weekly: {}", format_percent(weekly_left));
        if let Some(credits) = entry.credits_remaining {
            println!("Credits: {:.1}", credits);
        }
        println!("Updated: {}", entry.updated_at);
        println!();
    }
}

fn remaining_percent(window: Option<&RateWindow>) -> Option<f64> {
    window.and_then(|window| {
        window
            .used_percent
            .map(|used| (100.0 - used).max(0.0).min(100.0))
    })
}

fn format_percent(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.0}% left"),
        None => "n/a".to_string(),
    }
}

fn requested_providers(raw: &str) -> Result<Vec<&'static str>> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "all" | "both" => Ok(vec!["codex", "claude"]),
        "codex" => Ok(vec!["codex"]),
        "claude" => Ok(vec!["claude"]),
        _ => bail!("unknown provider '{}'", raw),
    }
}

fn fetch_live_entry(provider: &str, args: &UsageArgs) -> Result<Option<ProviderEntry>> {
    match provider {
        "codex" => fetch_codex_entry(args),
        "claude" => fetch_claude_entry(args),
        _ => Ok(None),
    }
}

fn fetch_codex_entry(args: &UsageArgs) -> Result<Option<ProviderEntry>> {
    match fetch_codex_entry_via_rpc(args) {
        Ok(Some(entry)) => return Ok(Some(entry)),
        Ok(None) => {}
        Err(error) => {
            eprintln!("codexbar: codex RPC fetch failed, trying /status fallback: {error:#}");
        }
    }

    fetch_codex_entry_via_status(args)
}

fn fetch_codex_entry_via_rpc(args: &UsageArgs) -> Result<Option<ProviderEntry>> {
    let mut session = match CodexRpcSession::start()? {
        Some(session) => session,
        None => return Ok(None),
    };

    session.initialize()?;
    let account = session.fetch_account().ok();
    let limits = session
        .fetch_rate_limits()
        .context("failed to fetch codex rate limits via app-server")?;

    let primary = rate_window_from_codex(limits.rate_limits.primary);
    let secondary = rate_window_from_codex(limits.rate_limits.secondary);
    if primary.is_none() && secondary.is_none() {
        return Ok(None);
    }

    let identity =
        account
            .and_then(|response| response.account)
            .and_then(|details| match details {
                RpcAccountDetails::ApiKey => None,
                RpcAccountDetails::ChatGPT { email, plan_type } => Some(IdentityInfo {
                    account_email: email,
                    account_organization: None,
                    login_method: plan_type,
                }),
            });

    let credits_remaining = limits
        .rate_limits
        .credits
        .and_then(|credits| credits.balance)
        .and_then(|balance| balance.parse::<f64>().ok());

    Ok(Some(build_codex_entry(
        args,
        primary,
        secondary,
        credits_remaining,
        identity,
        "codex-cli",
    )))
}

fn fetch_codex_entry_via_status(args: &UsageArgs) -> Result<Option<ProviderEntry>> {
    let output = match run_command_with_timeout_and_input(
        "codex",
        &["-s", "read-only", "-a", "untrusted"],
        Some("/status\n"),
        Duration::from_secs(20),
    ) {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) if error.kind() == ErrorKind::TimedOut => return Ok(None),
        Err(error) => return Err(error).context("failed to run codex /status"),
    };

    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let cleaned = strip_ansi_sequences(&text);

    let five_line = first_line_containing_case_insensitive(&cleaned, "5h limit");
    let weekly_line = first_line_containing_case_insensitive(&cleaned, "weekly limit");
    let five_left = five_line
        .as_deref()
        .and_then(extract_percent_left_from_line)
        .map(|left| (100.0 - left).clamp(0.0, 100.0));
    let weekly_left = weekly_line
        .as_deref()
        .and_then(extract_percent_left_from_line)
        .map(|left| (100.0 - left).clamp(0.0, 100.0));
    let credits_remaining = extract_credits_from_status(&cleaned);

    if five_left.is_none() && weekly_left.is_none() && credits_remaining.is_none() {
        return Ok(None);
    }

    let primary = five_left.map(|used| RateWindow {
        used_percent: Some(used),
        window_minutes: Some(300),
        resets_at: None,
    });
    let secondary = weekly_left.map(|used| RateWindow {
        used_percent: Some(used),
        window_minutes: Some(10080),
        resets_at: None,
    });

    Ok(Some(build_codex_entry(
        args,
        primary,
        secondary,
        credits_remaining,
        None,
        "codex-status",
    )))
}

fn build_codex_entry(
    args: &UsageArgs,
    primary: Option<RateWindow>,
    secondary: Option<RateWindow>,
    credits_remaining: Option<f64>,
    identity: Option<IdentityInfo>,
    default_source: &str,
) -> ProviderEntry {
    let source = if args.source.eq_ignore_ascii_case("auto") {
        default_source.to_string()
    } else {
        args.source.clone()
    };
    let status = if args.status {
        Some(StatusInfo {
            indicator: Some("none".to_string()),
            description: Some("Operational".to_string()),
            updated_at: Some(now_iso8601()),
            url: Some("https://status.openai.com/".to_string()),
        })
    } else {
        None
    };

    ProviderEntry {
        provider: "codex".to_string(),
        source: Some(source),
        updated_at: now_iso8601(),
        primary,
        secondary,
        tertiary: None,
        credits_remaining,
        code_review_remaining_percent: None,
        identity,
        status,
    }
}

fn fetch_claude_entry(args: &UsageArgs) -> Result<Option<ProviderEntry>> {
    let access_token = match resolve_claude_oauth_access_token() {
        Some(value) => value,
        None => return Ok(None),
    };

    let output =
        match fetch_json_with_bearer("https://api.anthropic.com/api/oauth/usage", &access_token) {
            Ok(output) => output,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) if error.kind() == ErrorKind::TimedOut => return Ok(None),
            Err(error) => return Err(error).context("failed to query Claude OAuth usage API"),
        };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let (body, status_code) = match split_curl_body_and_status(&stdout) {
        Some(parts) => parts,
        None => return Ok(None),
    };
    if status_code != 200 {
        return Ok(None);
    }

    Ok(claude_entry_from_usage_json(body, args, "claude-oauth-api"))
}

fn first_env_value(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn resolve_claude_oauth_access_token() -> Option<String> {
    first_env_value(&["CODEXBAR_CLAUDE_OAUTH_TOKEN", "CLAUDE_OAUTH_TOKEN"])
        .or_else(|| lookup_claude_secret("oauth_access_token"))
        .or_else(load_claude_oauth_access_token_from_credentials_file)
}

fn load_claude_oauth_access_token_from_credentials_file() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = PathBuf::from(home)
        .join(".claude")
        .join(".credentials.json");
    let raw = fs::read_to_string(path).ok()?;
    let json = serde_json::from_str::<Value>(&raw).ok()?;
    let token = json
        .get("claudeAiOauth")
        .and_then(|value| value.get("accessToken"))
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn fetch_json_with_bearer(endpoint: &str, access_token: &str) -> io::Result<Output> {
    let args_owned = [
        "-sS".to_string(),
        "--location".to_string(),
        "--max-time".to_string(),
        "15".to_string(),
        "-H".to_string(),
        format!("Authorization: Bearer {access_token}"),
        "-H".to_string(),
        "anthropic-beta: oauth-2025-04-20".to_string(),
        "-H".to_string(),
        "Accept: application/json".to_string(),
        "-w".to_string(),
        "\n%{http_code}".to_string(),
        endpoint.to_string(),
    ];
    let args = args_owned.iter().map(String::as_str).collect::<Vec<_>>();
    run_command_with_timeout("curl", &args, Duration::from_secs(20))
}

fn lookup_claude_secret(field: &str) -> Option<String> {
    lookup_claude_secret_via_secret_tool(field).or_else(|| lookup_claude_secret_via_kwallet(field))
}

fn lookup_claude_secret_via_secret_tool(field: &str) -> Option<String> {
    let args = [
        "lookup", "service", "codexbar", "provider", "claude", "field", field,
    ];
    let output = run_command_with_timeout("secret-tool", &args, Duration::from_secs(8)).ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn store_claude_secret(field: &str, label: &str, value: &str) -> Result<()> {
    if store_claude_secret_via_secret_tool(field, label, value).is_ok() {
        return Ok(());
    }

    if store_claude_secret_via_kwallet(field, value).is_ok() {
        return Ok(());
    }

    bail!(
        "failed to store Claude credentials securely; install libsecret-tools (secret-tool) or ensure KDE Wallet is available"
    );
}

fn store_claude_secret_via_secret_tool(field: &str, label: &str, value: &str) -> Result<()> {
    let args = [
        "store", "--label", label, "service", "codexbar", "provider", "claude", "field", field,
    ];

    let mut secret = value.to_string();
    secret.push('\n');
    let output = run_command_with_timeout_and_input(
        "secret-tool",
        &args,
        Some(secret.as_str()),
        Duration::from_secs(12),
    )
    .with_context(|| {
        "failed to invoke secret-tool; install libsecret-tools (secret-tool)".to_string()
    })?;

    if !output.status.success() {
        bail!(
            "failed to store Claude credentials securely: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

fn lookup_claude_secret_via_kwallet(field: &str) -> Option<String> {
    let entry = format!("claude.{field}");
    for wallet in ["kdewallet", "kdewallet5"] {
        let args = ["-f", "CodexBar", "-r", entry.as_str(), wallet];
        let output = match run_command_with_timeout("kwallet-query", &args, Duration::from_secs(8))
        {
            Ok(output) => output,
            Err(_) => continue,
        };
        if !output.status.success() {
            continue;
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }

    None
}

fn store_claude_secret_via_kwallet(field: &str, value: &str) -> Result<()> {
    let entry = format!("claude.{field}");
    let mut last_error = None;

    for wallet in ["kdewallet", "kdewallet5"] {
        let args = ["-f", "CodexBar", "-w", entry.as_str(), wallet];
        let mut secret = value.to_string();
        secret.push('\n');
        let output = match run_command_with_timeout_and_input(
            "kwallet-query",
            &args,
            Some(secret.as_str()),
            Duration::from_secs(12),
        ) {
            Ok(output) => output,
            Err(error) => {
                last_error = Some(error.to_string());
                continue;
            }
        };

        if output.status.success() {
            return Ok(());
        }

        last_error = Some(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    bail!(
        "failed to store Claude credentials with KDE Wallet: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )
}

fn split_curl_body_and_status(output: &str) -> Option<(&str, u16)> {
    let trimmed = output.trim_end_matches(['\r', '\n']);
    let (body, status_line) = trimmed.rsplit_once('\n')?;
    let status_code = status_line.trim().parse::<u16>().ok()?;
    Some((body, status_code))
}

fn claude_entry_from_usage_json(
    raw_json: &str,
    args: &UsageArgs,
    source_label: &str,
) -> Option<ProviderEntry> {
    let value = serde_json::from_str::<Value>(raw_json).ok()?;
    let primary = rate_window_from_claude_json(&value, "five_hour", 300);
    let secondary = rate_window_from_claude_json(&value, "seven_day", 10080);
    let tertiary = rate_window_from_claude_json(&value, "seven_day_sonnet", 10080)
        .or_else(|| rate_window_from_claude_json(&value, "seven_day_opus", 10080));

    if primary.is_none() && secondary.is_none() && tertiary.is_none() {
        return None;
    }

    let source = if args.source.eq_ignore_ascii_case("auto") {
        source_label.to_string()
    } else {
        args.source.clone()
    };

    let status = if args.status {
        Some(StatusInfo {
            indicator: Some("none".to_string()),
            description: Some("Operational".to_string()),
            updated_at: Some(now_iso8601()),
            url: Some("https://status.claude.com/".to_string()),
        })
    } else {
        None
    };

    Some(ProviderEntry {
        provider: "claude".to_string(),
        source: Some(source),
        updated_at: now_iso8601(),
        primary,
        secondary,
        tertiary,
        credits_remaining: None,
        code_review_remaining_percent: None,
        identity: Some(IdentityInfo {
            account_email: None,
            account_organization: None,
            login_method: Some("oauth".to_string()),
        }),
        status,
    })
}

fn rate_window_from_claude_json(
    value: &Value,
    key: &str,
    window_minutes: u64,
) -> Option<RateWindow> {
    let window = value.get(key)?;
    let used_percent = window
        .get("utilization")
        .and_then(json_number_value)
        .map(|value| value.clamp(0.0, 100.0));
    let resets_at = match window.get("resets_at") {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Some(Value::Number(number)) => number
            .as_i64()
            .map(|seconds| format!("unix:{seconds}"))
            .or_else(|| number.as_u64().map(|seconds| format!("unix:{seconds}"))),
        _ => None,
    };

    if used_percent.is_none() && resets_at.is_none() {
        return None;
    }

    Some(RateWindow {
        used_percent,
        window_minutes: Some(window_minutes),
        resets_at,
    })
}

fn json_number_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(string_value) => string_value.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn rate_window_from_codex(window: Option<RpcRateLimitWindow>) -> Option<RateWindow> {
    let window = window?;
    let used_percent = window.used_percent?;

    Some(RateWindow {
        used_percent: Some(used_percent),
        window_minutes: window.window_duration_mins,
        resets_at: window
            .resets_at
            .map(|timestamp| format!("unix:{timestamp}")),
    })
}

fn run_command_with_timeout(program: &str, args: &[&str], timeout: Duration) -> io::Result<Output> {
    run_command_with_timeout_and_input(program, args, None, timeout)
}

fn run_command_with_timeout_and_input(
    program: &str,
    args: &[&str],
    input: Option<&str>,
    timeout: Duration,
) -> io::Result<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn()?;
    if let Some(input_text) = input {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input_text.as_bytes())?;
            stdin.flush()?;
        }
    }

    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                format!("{program} timed out"),
            ));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

fn strip_ansi_sequences(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == 0x1B {
            index += 1;
            if index < bytes.len() && bytes[index] == b'[' {
                index += 1;
                while index < bytes.len() {
                    let byte = bytes[index];
                    index += 1;
                    if (0x40..=0x7E).contains(&byte) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }

        output.push(bytes[index] as char);
        index += 1;
    }

    output
}

fn first_line_containing_case_insensitive(text: &str, needle: &str) -> Option<String> {
    let needle_lower = needle.to_ascii_lowercase();
    text.lines()
        .find(|line| line.to_ascii_lowercase().contains(&needle_lower))
        .map(ToOwned::to_owned)
}

fn extract_percent_left_from_line(line: &str) -> Option<f64> {
    if !line.to_ascii_lowercase().contains("left") {
        return None;
    }
    let percent_index = line.find('%')?;
    let prefix = &line[..percent_index];
    parse_last_number(prefix)
}

fn extract_credits_from_status(text: &str) -> Option<f64> {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("credits") {
            continue;
        }
        if let Some((_, tail)) = line.split_once(':') {
            if let Some(value) = parse_first_number(tail) {
                return Some(value);
            }
        }
        if let Some(value) = parse_first_number(line) {
            return Some(value);
        }
    }
    None
}

fn parse_first_number(input: &str) -> Option<f64> {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut start = 0;
    while start < bytes.len() && !bytes[start].is_ascii_digit() {
        start += 1;
    }
    if start == bytes.len() {
        return None;
    }

    let mut end = start;
    while end < bytes.len()
        && (bytes[end].is_ascii_digit() || bytes[end] == b'.' || bytes[end] == b',')
    {
        end += 1;
    }

    let raw = input[start..end].replace(',', "");
    raw.parse::<f64>().ok()
}

fn parse_last_number(input: &str) -> Option<f64> {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut end = bytes.len();
    while end > 0 && !bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = end;
    while start > 0 {
        let byte = bytes[start - 1];
        if byte.is_ascii_digit() || byte == b'.' {
            start -= 1;
        } else {
            break;
        }
    }

    input[start..end].parse::<f64>().ok()
}

struct CodexRpcSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
}

impl CodexRpcSession {
    fn start() -> Result<Option<Self>> {
        let mut child = match Command::new("codex")
            .args(["-s", "read-only", "-a", "untrusted", "app-server"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("failed to launch codex app-server"),
        };

        let stdin = child
            .stdin
            .take()
            .context("failed to open codex app-server stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("failed to open codex app-server stdout")?;

        Ok(Some(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        }))
    }

    fn initialize(&mut self) -> Result<()> {
        let _ = self.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codexbar-rust",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )?;
        self.notify("initialized", json!({}))?;
        Ok(())
    }

    fn fetch_account(&mut self) -> Result<RpcAccountResponse> {
        let value = self.request("account/read", json!({}))?;
        serde_json::from_value(value).context("failed to decode codex account response")
    }

    fn fetch_rate_limits(&mut self) -> Result<RpcRateLimitsResponse> {
        let value = self.request("account/rateLimits/read", json!({}))?;
        serde_json::from_value(value).context("failed to decode codex rate limits response")
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        self.send_payload(json!({
            "id": id,
            "method": method,
            "params": params
        }))?;

        loop {
            let message = self.read_message()?;
            let message_id = message.get("id").and_then(Value::as_i64);
            if message_id != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error") {
                bail!("codex app-server request '{method}' failed: {error}");
            }

            if let Some(result) = message.get("result") {
                return Ok(result.clone());
            }

            bail!("codex app-server response missing result for method '{method}'");
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        self.send_payload(json!({
            "method": method,
            "params": params
        }))
    }

    fn send_payload(&mut self, payload: Value) -> Result<()> {
        let bytes =
            serde_json::to_vec(&payload).context("failed to serialize codex RPC payload")?;
        self.stdin
            .write_all(&bytes)
            .context("failed to write codex RPC payload")?;
        self.stdin
            .write_all(b"\n")
            .context("failed to terminate codex RPC payload line")?;
        self.stdin
            .flush()
            .context("failed to flush codex RPC payload")?;
        Ok(())
    }

    fn read_message(&mut self) -> Result<Value> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .stdout
                .read_line(&mut line)
                .context("failed reading codex app-server output")?;
            if read == 0 {
                bail!("codex app-server closed stdout");
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                return Ok(value);
            }
        }
    }
}

impl Drop for CodexRpcSession {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcAccountResponse {
    account: Option<RpcAccountDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum RpcAccountDetails {
    #[serde(rename = "apiKey")]
    ApiKey,
    #[serde(rename = "chatgpt")]
    ChatGPT {
        email: Option<String>,
        #[serde(rename = "planType")]
        plan_type: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RpcRateLimitsResponse {
    #[serde(rename = "rateLimits")]
    rate_limits: RpcRateLimitSnapshot,
}

#[derive(Debug, Deserialize)]
struct RpcRateLimitSnapshot {
    primary: Option<RpcRateLimitWindow>,
    secondary: Option<RpcRateLimitWindow>,
    credits: Option<RpcCreditsSnapshot>,
}

#[derive(Debug, Deserialize)]
struct RpcRateLimitWindow {
    #[serde(rename = "usedPercent")]
    used_percent: Option<f64>,
    #[serde(rename = "windowDurationMins")]
    window_duration_mins: Option<u64>,
    #[serde(rename = "resetsAt")]
    resets_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct RpcCreditsSnapshot {
    balance: Option<String>,
}
