mod ccusage;
#[cfg(feature = "claude-code-stats-provider")]
mod lib_provider;

use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ActiveData {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UsageWindow {
    pub utilization: f64,
    pub resets_in_minutes: Option<f64>,
    pub usage_level: String,
    pub pace: Option<PaceInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PaceInfo {
    pub delta_percent: f64,
    pub expected_percent: f64,
    pub will_last_to_reset: bool,
    pub eta_minutes: Option<f64>,
}

pub fn fetch(provider: &str) -> Result<ActiveData> {
    match provider {
        "ccusage" => ccusage::fetch(),
        #[cfg(feature = "claude-code-stats-provider")]
        "lib" => lib_provider::fetch(),
        other => Err(anyhow!("unknown provider: {other}")),
    }
}

/// Compute pace locally when the API doesn't provide it.
fn compute_pace(utilization: f64, resets_in_minutes: f64, window_minutes: f64) -> Option<PaceInfo> {
    if window_minutes <= 0.0 || resets_in_minutes <= 0.0 || resets_in_minutes > window_minutes {
        return None;
    }

    let elapsed = (window_minutes - resets_in_minutes) * 60.0;
    let duration = window_minutes * 60.0;
    let time_left = resets_in_minutes * 60.0;

    let actual = utilization.clamp(0.0, 100.0);
    let expected = ((elapsed / duration) * 100.0).clamp(0.0, 100.0);

    if (elapsed == 0.0 && actual > 0.0) || expected < 3.0 {
        return None;
    }

    let delta = actual - expected;

    let (will_last_to_reset, eta_minutes) = if elapsed > 0.0 && actual > 0.0 {
        let rate = actual / elapsed;
        if rate > 0.0 {
            let remaining = (100.0 - actual).max(0.0);
            let candidate = remaining / rate;
            if candidate >= time_left {
                (true, None)
            } else {
                (false, Some(candidate / 60.0))
            }
        } else {
            (true, None)
        }
    } else if elapsed > 0.0 {
        (true, None)
    } else {
        return None;
    };

    Some(PaceInfo {
        delta_percent: delta,
        expected_percent: expected,
        will_last_to_reset,
        eta_minutes,
    })
}

/// Fill in pace data for windows that don't have it.
fn ensure_pace(window: &mut UsageWindow, window_minutes: f64) {
    if window.pace.is_some() {
        return;
    }
    if let Some(resets_in) = window.resets_in_minutes {
        window.pace = compute_pace(window.utilization, resets_in, window_minutes);
    }
}
