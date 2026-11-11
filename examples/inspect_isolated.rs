use std::{env, fs, path::PathBuf};
use anyhow::{Context, Result};
use cfsolve::{Url, challenge::{self, program::PreparedSession}, script::Source};
fn inspect() -> Result<()> {
    let directory = PathBuf::from(env::args().nth(1).context("capture directory required")?);
    let html = fs::read_to_string(directory.join("widget-1.html"))?;
    let javascript = challenge::inline_javascript(&html)?;
    let deobfuscated = cfsolve::deobfuscate(javascript)?;
    fs::write(directory.join("widget.deobfuscated.js"), &deobfuscated.text)?;
    let source = Source::try_new(&deobfuscated.text)?;
    let url: Url = fs::read_to_string(directory.join("iframe-url.txt"))?.trim().parse()?;
    let prepared = PreparedSession::try_new(&source, &url)?;
    fs::write(directory.join("encoder.json"), serde_json::to_vec_pretty(&prepared.program().encoder_json())?)?;
    if directory.join("bridge-1-2-body.bin").exists() {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let body = fs::read(directory.join("bridge-1-2-body.bin"))?;
        let decoded = challenge::decode_response(&body, prepared.program().request.ray()?)?;
        let program = STANDARD.decode(decoded)?;
        let analysis = cfsolve::stage2::analysis::analyze(&source, &program)?;
        fs::write(directory.join("stage2.json"), serde_json::to_vec_pretty(&analysis.artifact())?)?;
    }
    println!("encoder={}", prepared.program().encoder);
    let encoder = &prepared.program().encoder;
    let pattern = regex::Regex::new(&format!(r"\b{}\s*=\s*function\s*\((\w+)\)\s*\{{", regex::escape(encoder)))?;
    let patched = pattern.replace(&html, |captures: &regex::Captures<'_>| format!("{}window.__PAYLOADS=window.__PAYLOADS||[];window.__PAYLOADS.push(JSON.parse(JSON.stringify({})));", &captures[0], &captures[1]));
    anyhow::ensure!(patched != html, "encoder declaration not located in original source");
    fs::write(directory.join("widget.payload-capture.html"), patched.as_bytes())?;
    Ok(())
}
fn main() -> Result<()> {
    std::thread::Builder::new().stack_size(64*1024*1024).spawn(inspect)?.join().expect("analysis panicked")
}
