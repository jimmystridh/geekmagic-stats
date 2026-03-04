use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{ensure_pace, ActiveData, UsageWindow};

#[derive(Deserialize)]
struct CcusageOutput {
    blocks: Vec<CcusageBlock>,
}

#[derive(Deserialize)]
struct CcusageBlock {
    #[serde(rename = "isActive")]
    is_active: bool,
    #[serde(rename = "isGap")]
    is_gap: Option<bool>,
    #[serde(rename = "totalTokens")]
    total_tokens: u64,
    #[serde(rename = "actualEndTime")]
    actual_end_time: Option<String>,
    projection: Option<CcusageProjection>,
}

#[derive(Deserialize)]
struct CcusageProjection {
    #[serde(rename = "remainingMinutes")]
    remaining_minutes: f64,
}

pub fn fetch() -> Result<ActiveData> {
    let output = Command::new("ccusage")
        .args(["blocks", "--json", "--offline"])
        .output()
        .context("failed to run ccusage")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: CcusageOutput =
        serde_json::from_str(&stdout).context("failed to parse ccusage output")?;

    // Historical max: highest totalTokens among completed (non-gap) blocks
    let max_past_tokens = parsed
        .blocks
        .iter()
        .filter(|b| !b.is_active && b.is_gap != Some(true) && b.total_tokens > 0)
        .map(|b| b.total_tokens)
        .max()
        .unwrap_or(0);

    // Find active non-gap block
    let active = parsed
        .blocks
        .iter()
        .find(|b| b.is_active && b.is_gap != Some(true))
        .context("no active block found in ccusage output")?;

    // Calibrate against historical max; fallback to active tokens on first session
    let reference = if max_past_tokens > 0 {
        max_past_tokens
    } else {
        active.total_tokens
    };

    let utilization = if reference > 0 {
        (active.total_tokens as f64 / reference as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let resets_in_minutes = active.projection.as_ref().map(|p| p.remaining_minutes);

    let usage_level = if utilization > 90.0 {
        "danger"
    } else if utilization > 70.0 {
        "warn"
    } else {
        "normal"
    }
    .to_string();

    let mut data = ActiveData {
        five_hour: Some(UsageWindow {
            utilization,
            resets_in_minutes,
            usage_level,
            pace: None,
        }),
        seven_day: None,
        updated_at: active.actual_end_time.clone(),
    };

    if let Some(w) = &mut data.five_hour {
        ensure_pace(w, 300.0);
    }

    Ok(data)
}
