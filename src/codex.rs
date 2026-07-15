use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use chrono::{Datelike, Local, TimeZone, Timelike};
use serde_json::{Value, json};

use crate::{
    model::{LimitWindow, UsageSnapshot},
    native,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_INTERVAL: Duration = Duration::from_secs(8);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(12);

pub fn start_worker() {
    thread::Builder::new()
        .name("codex-usage-reader".to_string())
        .spawn(worker_loop)
        .expect("start Codex usage reader");
}

fn worker_loop() {
    loop {
        while !native::codex_is_active() {
            thread::sleep(Duration::from_millis(500));
        }

        native::update_snapshot(UsageSnapshot::connecting());
        let result = discover_desktop_cli().and_then(|executable| run_session(&executable));
        if let Err(error) = result {
            eprintln!("Codex usage reader: {error:#}");
            if native::codex_is_active() {
                native::update_snapshot(UsageSnapshot::error("暂时无法读取用量，正在重试…"));
            }
        }

        let deadline = Instant::now() + RETRY_INTERVAL;
        while native::codex_is_active() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(250));
        }
    }
}

fn run_session(executable: &Path) -> Result<()> {
    let mut child = spawn_app_server(executable)?;
    let result = communicate(&mut child);
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn spawn_app_server(executable: &Path) -> Result<Child> {
    Command::new(executable)
        .args(["-s", "read-only", "-a", "untrusted", "app-server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .with_context(|| format!("launch {} app-server", executable.display()))
}

fn communicate(child: &mut Child) -> Result<()> {
    let mut stdin = child.stdin.take().context("app-server stdin unavailable")?;
    let stdout = child
        .stdout
        .take()
        .context("app-server stdout unavailable")?;
    let (sender, receiver) = mpsc::channel::<String>();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    send_request(
        &mut stdin,
        1,
        "initialize",
        json!({
            "clientInfo": {
                "name": "codex-titlebar-meter",
                "title": "Codex Titlebar Meter",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )?;
    wait_for_response(&receiver, 1, RESPONSE_TIMEOUT)?;

    let mut request_id = 2_u64;
    send_request(
        &mut stdin,
        request_id,
        "account/rateLimits/read",
        Value::Null,
    )?;
    let response = wait_for_response(&receiver, request_id, RESPONSE_TIMEOUT)?;
    native::update_snapshot(parse_snapshot(&response)?);
    let mut last_refresh = Instant::now();

    loop {
        if !native::codex_is_active() {
            return Ok(());
        }

        let should_refresh = match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(line) => {
                let value: Value =
                    serde_json::from_str(&line).context("invalid app-server JSON")?;
                value.get("method").and_then(Value::as_str) == Some("account/rateLimits/updated")
            }
            Err(mpsc::RecvTimeoutError::Timeout) => false,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("app-server output closed");
            }
        };

        if should_refresh || last_refresh.elapsed() >= REFRESH_INTERVAL {
            request_id += 1;
            send_request(
                &mut stdin,
                request_id,
                "account/rateLimits/read",
                Value::Null,
            )?;
            let response = wait_for_response(&receiver, request_id, RESPONSE_TIMEOUT)?;
            native::update_snapshot(parse_snapshot(&response)?);
            last_refresh = Instant::now();
        }
    }
}

fn send_request(stdin: &mut impl Write, id: u64, method: &str, params: Value) -> Result<()> {
    serde_json::to_writer(
        &mut *stdin,
        &json!({"id": id, "method": method, "params": params}),
    )?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

fn wait_for_response(
    receiver: &mpsc::Receiver<String>,
    id: u64,
    timeout: Duration,
) -> Result<Value> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let line = receiver
            .recv_timeout(remaining)
            .context("app-server response timed out")?;
        let value: Value = serde_json::from_str(&line).context("invalid app-server JSON")?;
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if let Some(error) = value.get("error") {
            bail!("app-server error: {error}");
        }
        return Ok(value);
    }
}

fn parse_snapshot(response: &Value) -> Result<UsageSnapshot> {
    let limits = response
        .pointer("/result/rateLimits")
        .context("missing result.rateLimits")?;
    let primary = parse_window(limits.get("primary"));
    let weekly = parse_window(limits.get("secondary"));
    if primary.is_none() && weekly.is_none() {
        bail!("Codex returned no active quota windows");
    }
    Ok(UsageSnapshot {
        primary,
        weekly,
        status: None,
        sampled_at: Some(Local::now()),
    })
}

fn parse_window(value: Option<&Value>) -> Option<LimitWindow> {
    let value = value?;
    if !value.is_object() {
        return None;
    }
    let used = value
        .get("usedPercent")
        .and_then(number_as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    let duration = value
        .get("windowDurationMins")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reset = value
        .get("resetsAt")
        .and_then(Value::as_i64)
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single());

    let label = if duration == 10_080 {
        "1周".to_string()
    } else if duration >= 1_440 && duration % 1_440 == 0 {
        format!("{}天", duration / 1_440)
    } else if duration >= 60 && duration % 60 == 0 {
        format!("{}小时", duration / 60)
    } else {
        "当前".to_string()
    };
    let reset_label = match reset {
        Some(reset) if duration >= 1_440 => format!("{}月{}日", reset.month(), reset.day()),
        Some(reset) => format!("{:02}:{:02}", reset.hour(), reset.minute()),
        None => "--".to_string(),
    };

    Some(LimitWindow {
        label,
        remaining_percent: (100.0 - used).round().clamp(0.0, 100.0) as u8,
        reset_label,
    })
}

fn number_as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_u64().map(|value| value as f64))
}

fn discover_desktop_cli() -> Result<PathBuf> {
    if let Some(source) = native::codex_desktop_cli_source() {
        let package_name = source
            .ancestors()
            .nth(3)
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("OpenAI.Codex");
        if let Some(cached) = cache_desktop_cli(&source, package_name)? {
            return Ok(cached);
        }
    }

    let program_files = env::var_os("ProgramFiles").context("ProgramFiles is unavailable")?;
    let windows_apps = PathBuf::from(program_files).join("WindowsApps");
    let mut packages = fs::read_dir(&windows_apps)
        .with_context(|| format!("read {}", windows_apps.display()))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("OpenAI.Codex_")
        })
        .collect::<Vec<_>>();
    packages.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));

    for package in packages {
        let source = package.path().join("app/resources/codex.exe");
        if !source.is_file() {
            continue;
        }
        if let Some(cached) = cache_desktop_cli(&source, &package.file_name().to_string_lossy())? {
            return Ok(cached);
        }
    }
    bail!("OpenAI Codex Desktop CLI was not found")
}

fn cache_desktop_cli(source: &Path, package_name: &str) -> Result<Option<PathBuf>> {
    let local_app_data = env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is unavailable")?;
    let directory = PathBuf::from(local_app_data)
        .join("ConfigCrate")
        .join("CodexTitlebarMeter")
        .join("desktop-cli")
        .join(package_name);
    let destination = directory.join("codex.exe");
    let source_len = fs::metadata(source)?.len();
    if fs::metadata(&destination).is_ok_and(|metadata| metadata.len() == source_len) {
        return Ok(Some(destination));
    }

    fs::create_dir_all(&directory)?;
    let temporary = directory.join("codex.exe.tmp");
    let _ = fs::remove_file(&temporary);
    fs::copy(source, &temporary).with_context(|| format!("cache {}", source.display()))?;
    if fs::rename(&temporary, &destination).is_err() {
        let _ = fs::remove_file(&destination);
        fs::rename(&temporary, &destination)?;
    }
    Ok(Some(destination))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remaining_percent_and_labels() {
        let response = json!({
            "result": {
                "rateLimits": {
                    "primary": {
                        "usedPercent": 22,
                        "windowDurationMins": 300,
                        "resetsAt": 1_800_000_000
                    },
                    "secondary": {
                        "usedPercent": 6,
                        "windowDurationMins": 10080,
                        "resetsAt": 1_800_500_000
                    }
                }
            }
        });
        let parsed = parse_snapshot(&response).expect("snapshot");
        assert_eq!(parsed.primary.as_ref().unwrap().remaining_percent, 78);
        assert_eq!(parsed.primary.as_ref().unwrap().label, "5小时");
        assert_eq!(parsed.weekly.as_ref().unwrap().remaining_percent, 94);
        assert_eq!(parsed.weekly.as_ref().unwrap().label, "1周");
    }
}
