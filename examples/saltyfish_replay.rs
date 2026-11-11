//! Replays the locally obtained clearance once, without running another challenge.
use std::{fs, path::Path, time::Instant};

use anyhow::{Context, Result, ensure};
use cfsolve::{Url, fingerprint::Transport, http};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let url: Url = "https://portal.saltyfish.io/store/fra-premium".parse()?;
    let output = Path::new("results/first-run");
    let stdout = fs::read_to_string(output.join("stdout.txt"))?;
    let cookie = stdout.trim().strip_prefix("cf_clearance=")
        .context("the first run returned no clearance cookie")?;
    let client = Transport::read(Path::new("profiles/safari-26-ios.json"))?.client(None)?;
    client.add_cookie(&format!("cf_clearance={cookie}; Path=/; Secure"), &url);
    let started = Instant::now();
    let page = http::navigate(&client, &url).await?;
    let elapsed = started.elapsed().as_secs_f64();
    let challenge = page.html.contains("window._cf_chl_opt");
    let product = page.html.contains("Frankfurt VPS - Premium Network")
        && page.html.contains("fra.p1.mini");
    fs::write(output.join("replay.html"), page.html.as_bytes())?;
    let summary = json!({
        "url": page.url.as_str(), "http_status": page.status.as_u16(),
        "elapsed_seconds": elapsed, "page_bytes": page.html.len(),
        "challenge_config_present": challenge, "expected_product_present": product,
        "same_profile": "profiles/safari-26-ios.json", "new_http_client": true,
        "solver_invoked": false, "browser_invoked": false
    });
    let text = serde_json::to_string_pretty(&summary)?;
    fs::write(output.join("replay-summary.json"), &text)?;
    println!("{text}");
    ensure!(page.status.is_success() && !challenge && product,
        "clearance replay did not return the expected product page");
    Ok(())
}
