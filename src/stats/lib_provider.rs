use anyhow::{Context, Result};
use serde::Deserialize;

use super::{ensure_pace, ActiveData};

#[derive(Debug, Deserialize)]
struct StatsPayload {
    #[allow(dead_code)]
    status: String,
    data: Option<ActiveData>,
}

pub fn fetch() -> Result<ActiveData> {
    let payload_json = claude_code_stats::collect_widget_payload_json();
    let payload: StatsPayload =
        serde_json::from_str(&payload_json).context("failed to parse claude-code-stats payload")?;

    let mut data = payload
        .data
        .context("claude-code-stats returned non-active status")?;

    if let Some(w) = &mut data.five_hour {
        ensure_pace(w, 300.0);
    }
    if let Some(w) = &mut data.seven_day {
        ensure_pace(w, 10080.0);
    }

    Ok(data)
}
