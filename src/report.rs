use crate::diff::DiffResult;
use crate::runner::RunResult;

/// Serialize run result to JSON for stdout.
pub fn to_json(result: &RunResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| {
        eprintln!("Error serializing JSON: {}", e);
        String::new()
    })
}

/// Serialize diff result to JSON.
pub fn diff_to_json(result: &DiffResult) -> String {
    serde_json::to_string_pretty(result).unwrap_or_else(|e| {
        eprintln!("Error serializing JSON: {}", e);
        String::new()
    })
}

/// Human-readable diff output.
pub fn diff_to_human(result: &DiffResult) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "\nWattLint Energy Diff\n\
         ────────────────────\n\
         Total:  {:.1} J → {:.1} J  ({}{:.1}%)\n",
        result.baseline_joules,
        result.current_joules,
        if result.delta_pct >= 0.0 { "+" } else { "" },
        result.delta_pct,
    ));

    for (domain, delta) in &result.domains {
        let sign = if delta.delta_pct >= 0.0 { "+" } else { "" };
        out.push_str(&format!(
            "{:<8} {:.1} J → {:.1} J  ({}{:.1}%)\n",
            format!("{}:", domain),
            delta.baseline,
            delta.current,
            sign,
            delta.delta_pct,
        ));
    }

    out.push_str(&format!("\nThreshold: {:.0}%\n", result.threshold_pct));
    out.push_str(&format!(
        "Status:   {}\n",
        if result.regression {
            "REGRESSION DETECTED"
        } else {
            "OK"
        }
    ));

    out
}

/// Markdown diff output for PR comments.
pub fn diff_to_markdown(result: &DiffResult) -> String {
    let mut out = String::from("<!-- wattlint-report -->\n## ⚡ WattLint Energy Report\n\n");
    out.push_str("| Domain | Baseline | This PR | Δ |\n");
    out.push_str("|--------|----------|---------|---|\n");

    for (domain, delta) in &result.domains {
        let emoji = if delta.delta_pct > 5.0 {
            "🔴"
        } else if delta.delta_pct < -2.0 {
            "🟢"
        } else {
            "✅"
        };
        let sign = if delta.delta_pct >= 0.0 { "+" } else { "" };
        out.push_str(&format!(
            "| {} | {:.1} J | {:.1} J | {} {}{:.1}% |\n",
            domain_title(domain),
            delta.baseline,
            delta.current,
            emoji,
            sign,
            delta.delta_pct,
        ));
    }

    let total_emoji = if result.regression { "🔴" } else { "✅" };
    let sign = if result.delta_pct >= 0.0 { "+" } else { "" };
    out.push_str(&format!(
        "| **Total** | **{:.1} J** | **{:.1} J** | {} **{}{:.1}%** |\n",
        result.baseline_joules, result.current_joules, total_emoji, sign, result.delta_pct,
    ));

    out.push_str(&format!(
        "\n**Threshold:** {:.0}% · **Status:** {} {}\n",
        result.threshold_pct,
        total_emoji,
        if result.regression {
            "REGRESSION DETECTED"
        } else {
            "WITHIN THRESHOLD"
        },
    ));

    out
}

fn domain_title(domain: &str) -> &str {
    match domain {
        "pkg" => "CPU Package",
        "pp0" => "CPU Cores",
        "dram" => "DRAM",
        "psys" => "Platform",
        "gpu" => "GPU",
        _ => domain,
    }
}
