use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use codexbar_core::{now_iso8601, IdentityInfo, ProviderEntry, RateWindow, StatusInfo};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
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
    }
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
    let output = match run_command_with_timeout("claude", &["/usage"], Duration::from_secs(12)) {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) if error.kind() == ErrorKind::TimedOut => return Ok(None),
        Err(error) => return Err(error).context("failed to run claude /usage"),
    };

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let cleaned = strip_ansi_sequences(&combined);
    if cleaned.trim().is_empty() {
        return Ok(None);
    }

    let session_used = extract_labeled_used_percent(&cleaned, &["current session", "session"]);
    let weekly_used = extract_labeled_used_percent(&cleaned, &["current week", "weekly"]);
    if session_used.is_none() && weekly_used.is_none() {
        return Ok(None);
    }

    let source = if args.source.eq_ignore_ascii_case("auto") {
        "claude-cli".to_string()
    } else {
        args.source.clone()
    };

    let status = if args.status {
        Some(StatusInfo {
            indicator: Some("none".to_string()),
            description: Some("Operational".to_string()),
            updated_at: Some(now_iso8601()),
            url: Some("https://status.anthropic.com/".to_string()),
        })
    } else {
        None
    };

    Ok(Some(ProviderEntry {
        provider: "claude".to_string(),
        source: Some(source),
        updated_at: now_iso8601(),
        primary: session_used.map(|used| RateWindow {
            used_percent: Some(used),
            window_minutes: Some(300),
            resets_at: None,
        }),
        secondary: weekly_used.map(|used| RateWindow {
            used_percent: Some(used),
            window_minutes: Some(10080),
            resets_at: None,
        }),
        tertiary: None,
        credits_remaining: None,
        code_review_remaining_percent: None,
        identity: None,
        status,
    }))
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

fn extract_labeled_used_percent(text: &str, labels: &[&str]) -> Option<f64> {
    let lines = text.lines().collect::<Vec<_>>();
    let normalized_labels = labels
        .iter()
        .map(|label| label.to_ascii_lowercase())
        .collect::<Vec<_>>();

    for (index, line) in lines.iter().enumerate() {
        let lower_line = line.to_ascii_lowercase();
        let has_label = normalized_labels
            .iter()
            .any(|label| lower_line.contains(label));
        if !has_label {
            continue;
        }

        for candidate in lines.iter().skip(index).take(8) {
            if let Some((value, interpretation)) = percent_from_line(candidate) {
                let clamped = value.clamp(0.0, 100.0);
                return Some(match interpretation {
                    PercentInterpretation::Used => clamped,
                    PercentInterpretation::Left => (100.0 - clamped).clamp(0.0, 100.0),
                });
            }
        }
    }

    None
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

#[derive(Debug, Copy, Clone)]
enum PercentInterpretation {
    Used,
    Left,
}

fn percent_from_line(line: &str) -> Option<(f64, PercentInterpretation)> {
    let lower_line = line.to_ascii_lowercase();
    let interpretation = if lower_line.contains("used")
        || lower_line.contains("spent")
        || lower_line.contains("consumed")
    {
        PercentInterpretation::Used
    } else {
        PercentInterpretation::Left
    };

    let percent_index = line.find('%')?;
    let prefix = &line[..percent_index];
    let value = parse_last_number(prefix)?;
    Some((value, interpretation))
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
