use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSnapshot {
    pub generated_at: String,
    pub enabled_providers: Vec<String>,
    pub entries: Vec<ProviderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderEntry {
    pub provider: String,
    pub source: Option<String>,
    pub updated_at: String,
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    pub tertiary: Option<RateWindow>,
    pub credits_remaining: Option<f64>,
    pub code_review_remaining_percent: Option<f64>,
    pub identity: Option<IdentityInfo>,
    pub status: Option<StatusInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RateWindow {
    pub used_percent: Option<f64>,
    pub window_minutes: Option<u64>,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    pub account_email: Option<String>,
    pub account_organization: Option<String>,
    pub login_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusInfo {
    pub indicator: Option<String>,
    pub description: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
}

impl WidgetSnapshot {
    pub fn from_codexbar_cli_values(values: &[Value]) -> Self {
        let entries = values
            .iter()
            .filter_map(ProviderEntry::from_codexbar_cli_value)
            .collect::<Vec<_>>();

        let enabled_providers = entries
            .iter()
            .map(|entry| entry.provider.clone())
            .collect::<Vec<_>>();

        Self {
            generated_at: now_iso8601(),
            enabled_providers,
            entries,
        }
    }

    pub fn sample() -> Self {
        Self {
            generated_at: now_iso8601(),
            enabled_providers: vec!["codex".to_string(), "claude".to_string()],
            entries: vec![
                ProviderEntry {
                    provider: "codex".to_string(),
                    source: Some("openai-web".to_string()),
                    updated_at: now_iso8601(),
                    primary: Some(RateWindow {
                        used_percent: Some(28.0),
                        window_minutes: Some(300),
                        resets_at: Some("2026-02-11T20:00:00Z".to_string()),
                    }),
                    secondary: Some(RateWindow {
                        used_percent: Some(61.0),
                        window_minutes: Some(10080),
                        resets_at: Some("2026-02-14T20:00:00Z".to_string()),
                    }),
                    tertiary: None,
                    credits_remaining: Some(92.4),
                    code_review_remaining_percent: Some(100.0),
                    identity: Some(IdentityInfo {
                        account_email: Some("codex@example.com".to_string()),
                        account_organization: None,
                        login_method: Some("plus".to_string()),
                    }),
                    status: Some(StatusInfo {
                        indicator: Some("none".to_string()),
                        description: Some("Operational".to_string()),
                        updated_at: Some(now_iso8601()),
                        url: Some("https://status.openai.com/".to_string()),
                    }),
                },
                ProviderEntry {
                    provider: "claude".to_string(),
                    source: Some("oauth".to_string()),
                    updated_at: now_iso8601(),
                    primary: Some(RateWindow {
                        used_percent: Some(41.0),
                        window_minutes: Some(300),
                        resets_at: Some("2026-02-11T23:30:00Z".to_string()),
                    }),
                    secondary: Some(RateWindow {
                        used_percent: Some(54.0),
                        window_minutes: Some(10080),
                        resets_at: Some("2026-02-16T01:00:00Z".to_string()),
                    }),
                    tertiary: None,
                    credits_remaining: None,
                    code_review_remaining_percent: None,
                    identity: Some(IdentityInfo {
                        account_email: Some("claude@example.com".to_string()),
                        account_organization: None,
                        login_method: Some("oauth".to_string()),
                    }),
                    status: Some(StatusInfo {
                        indicator: Some("none".to_string()),
                        description: Some("Operational".to_string()),
                        updated_at: Some(now_iso8601()),
                        url: Some("https://status.anthropic.com/".to_string()),
                    }),
                },
            ],
        }
    }
}

impl ProviderEntry {
    pub fn from_codexbar_cli_value(value: &Value) -> Option<Self> {
        let provider = value.get("provider")?.as_str()?.to_string();
        let usage = value.get("usage");

        let updated_at = usage
            .and_then(|obj| get_string(obj, "updatedAt"))
            .or_else(|| get_string(value, "updatedAt"))
            .unwrap_or_else(now_iso8601);

        let source = get_string(value, "source");
        let primary = usage
            .and_then(|obj| obj.get("primary"))
            .and_then(RateWindow::from_codexbar_cli_value);
        let secondary = usage
            .and_then(|obj| obj.get("secondary"))
            .and_then(RateWindow::from_codexbar_cli_value);
        let tertiary = usage
            .and_then(|obj| obj.get("tertiary"))
            .and_then(RateWindow::from_codexbar_cli_value);

        let credits_remaining = value
            .get("credits")
            .and_then(|obj| obj.get("remaining"))
            .and_then(to_f64);

        let code_review_remaining_percent = value
            .get("openaiDashboard")
            .and_then(|obj| obj.get("codeReviewRemainingPercent"))
            .and_then(to_f64);

        let identity = usage
            .and_then(|obj| obj.get("identity"))
            .map(|identity_obj| IdentityInfo {
                account_email: get_string(identity_obj, "accountEmail"),
                account_organization: get_string(identity_obj, "accountOrganization"),
                login_method: get_string(identity_obj, "loginMethod"),
            });

        let status = value.get("status").map(|status_obj| StatusInfo {
            indicator: get_string(status_obj, "indicator"),
            description: get_string(status_obj, "description"),
            updated_at: get_string(status_obj, "updatedAt"),
            url: get_string(status_obj, "url"),
        });

        Some(Self {
            provider,
            source,
            updated_at,
            primary,
            secondary,
            tertiary,
            credits_remaining,
            code_review_remaining_percent,
            identity,
            status,
        })
    }
}

impl RateWindow {
    pub fn from_codexbar_cli_value(value: &Value) -> Option<Self> {
        if value.is_null() {
            return None;
        }

        Some(Self {
            used_percent: value.get("usedPercent").and_then(to_f64),
            window_minutes: value.get("windowMinutes").and_then(to_u64),
            resets_at: get_string(value, "resetsAt"),
        })
    }

    pub fn remaining_percent(&self) -> Option<f64> {
        self.used_percent
            .map(|used| (100.0 - used).max(0.0).min(100.0))
    }
}

pub fn now_iso8601() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("unix:{}", duration.as_secs()),
        Err(_) => "unix:0".to_string(),
    }
}

fn get_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(string_value) => string_value.parse::<f64>().ok(),
        _ => None,
    }
}

fn to_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(string_value) => string_value.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_basic_codexbar_payload() {
        let payload = serde_json::json!({
            "provider": "codex",
            "source": "openai-web",
            "usage": {
                "updatedAt": "2026-02-11T10:00:00Z",
                "primary": {
                    "usedPercent": 30,
                    "windowMinutes": 300,
                    "resetsAt": "2026-02-11T12:00:00Z"
                },
                "secondary": null
            },
            "credits": {
                "remaining": 100.5
            },
            "openaiDashboard": {
                "codeReviewRemainingPercent": 88
            }
        });

        let snapshot = WidgetSnapshot::from_codexbar_cli_values(&[payload]);
        assert_eq!(snapshot.entries.len(), 1);

        let entry = &snapshot.entries[0];
        assert_eq!(entry.provider, "codex");
        assert_eq!(entry.source.as_deref(), Some("openai-web"));
        assert_eq!(entry.credits_remaining, Some(100.5));
        assert_eq!(entry.code_review_remaining_percent, Some(88.0));
        assert_eq!(
            entry
                .primary
                .as_ref()
                .and_then(|window| window.remaining_percent()),
            Some(70.0)
        );
    }
}
