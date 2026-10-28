//! splice.rs: write a splice9000 adapter in Rust. One file, no crate to
//! depend on besides serde_json. Use it from your main.rs:
//!
//! ```ignore
//! #[path = "splice.rs"]
//! mod splice;
//!
//! fn main() {
//!     splice::run(|host, command| {
//!         if command == "count" {
//!             let edit = host.edit()?;
//!             host.notify(&format!("{} clips", edit["clips"].as_array().map_or(0, Vec::len)));
//!         }
//!         Ok(())
//!     });
//! }
//! ```
//!
//! The protocol underneath is documented at
//! https://github.com/solacecasino/splice9000/blob/main/docs/adapters.md

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::Write;

use serde_json::{Value, json};

/// The connection to splice9000.
pub struct Host {
    lines: std::io::Lines<std::io::StdinLock<'static>>,
    next_id: u64,
    backlog: VecDeque<Value>,
    /// What splice9000 said when it started this adapter: version, protocol,
    /// permissions, and the paths of its ffmpeg and ffprobe.
    pub info: Value,
}

/// Sends a notification (no reply). Safe from any thread, so work done in
/// the background can report straight to splice9000.
pub fn post(method: &str, params: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", json!({ "method": method, "params": params }));
    let _ = out.flush();
}

impl Host {
    /// Sends a notification (no reply).
    pub fn send(&self, method: &str, params: Value) {
        post(method, params);
    }

    /// Sends a request and waits for splice9000's answer. Messages that
    /// arrive meanwhile are handled afterwards.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", json!({ "id": id, "method": method, "params": params }));
            let _ = out.flush();
        }
        loop {
            let line = self.lines.next().ok_or("splice9000 went away")?.map_err(|e| e.to_string())?;
            let Ok(m) = serde_json::from_str::<Value>(&line) else { continue };
            if m.get("method").is_none() && m["id"] == json!(id) {
                return match m.get("error") {
                    Some(e) => Err(e.as_str().unwrap_or("error").to_owned()),
                    None => Ok(m["result"].clone()),
                };
            }
            self.backlog.push_back(m);
        }
    }

    /// The ffmpeg splice9000 uses, so the adapter needs no copy of its own.
    pub fn ffmpeg(&self) -> String {
        self.info["ffmpeg"].as_str().map(String::from).or_else(|| std::env::var("SPLICE9000_FFMPEG").ok()).unwrap_or_else(|| "ffmpeg".into())
    }

    /// The edit: format, duration, playhead, selected, file, media, clips.
    /// Needs the "edit.read" permission.
    pub fn edit(&mut self) -> Result<Value, String> {
        self.request("edit.get", json!({}))
    }

    /// Replaces the timeline as one undo step. Needs "edit.change".
    pub fn set_clips(&mut self, clips: Value) -> Result<(), String> {
        self.request("edit.set", json!({ "clips": clips })).map(|_| ())
    }

    /// Plays at `rate`: 1 real time, 2 twice as fast, 0 stopped, below 0
    /// backward (protocol 3). Returns the rate it plays at.
    pub fn playback(&mut self, rate: f64) -> Result<f64, String> {
        self.request("playback.set", json!({ "rate": rate })).map(|r| r.as_f64().unwrap_or(0.0))
    }

    /// A short message at the bottom of the window.
    pub fn notify(&mut self, text: &str) {
        let _ = self.request("notify", json!({ "text": text }));
    }

    /// Fills one of your panels: strings, or {"text", "command"} buttons.
    pub fn panel(&mut self, id: &str, lines: Value) {
        let _ = self.request("panel.set", json!({ "id": id, "lines": lines }));
    }

    /// Writes to splice9000's Log panel.
    pub fn log(&self, message: &str) {
        self.send("log", json!({ "message": message }));
    }
}

/// What an adapter is told, for [`serve`].
pub enum Event<'a> {
    /// splice9000 started the adapter (after `initialize`).
    Start,
    /// One of its commands was chosen.
    Command(&'a str),
    /// Something happened that it listed in `events`: `kind` and the rest.
    Happened(&'a Value),
}

/// Handles messages until splice9000 says goodbye, calling `on_command` for
/// each command chosen. An error it returns is shown to the user.
pub fn run(mut on_command: impl FnMut(&mut Host, &str) -> Result<(), String>) {
    serve(|host, event| match event {
        Event::Command(c) => on_command(host, c),
        _ => Ok(()),
    });
}

/// Like [`run`], for adapters that also start working when started or
/// listen to events (protocol 3 splice9000s send `media` when files are
/// added).
pub fn serve(mut on: impl FnMut(&mut Host, Event) -> Result<(), String>) {
    let mut host = Host { lines: std::io::stdin().lines(), next_id: 0, backlog: VecDeque::new(), info: Value::Null };
    loop {
        let message = match host.backlog.pop_front() {
            Some(m) => m,
            None => match host.lines.next() {
                Some(Ok(line)) => serde_json::from_str(&line).unwrap_or(Value::Null),
                _ => return,
            },
        };
        match message["method"].as_str() {
            Some("initialize") => {
                host.info = message["params"].clone();
                if let Err(e) = on(&mut host, Event::Start) {
                    host.notify(&e);
                }
            }
            Some("command") => {
                let command = message["params"]["command"].as_str().unwrap_or_default().to_owned();
                if let Err(e) = on(&mut host, Event::Command(&command)) {
                    host.notify(&e);
                }
            }
            Some("event") => {
                if let Err(e) = on(&mut host, Event::Happened(&message["params"])) {
                    host.notify(&e);
                }
            }
            Some("shutdown") => return,
            _ => {}
        }
    }
}

/// The clip to work on: the selected one, or the one under the playhead.
pub fn target_clip(edit: &Value) -> Option<usize> {
    let clips = edit["clips"].as_array()?;
    if clips.is_empty() {
        return None;
    }
    if let Some(i) = edit["selected"].as_u64() {
        return Some(i as usize);
    }
    let t = edit["playhead"].as_f64()?;
    Some(clips.iter().rposition(|c| c["start"].as_f64().is_some_and(|s| s <= t + 1e-9)).unwrap_or(0))
}

/// The timeline's clips, as `edit.set` takes them (with their crossfades,
/// from protocol 3).
pub fn plain_clips(edit: &Value) -> Vec<Value> {
    edit["clips"]
        .as_array()
        .map(|clips| {
            clips.iter().map(|c| json!({ "media": c["media"], "src_in": c["src_in"], "src_out": c["src_out"], "fade": c["fade"].as_f64().unwrap_or(0.0) })).collect()
        })
        .unwrap_or_default()
}

/// A command that runs without a window popping up on Windows.
pub fn command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.stdin(std::process::Stdio::null());
    cmd
}

/// Runs ffmpeg and returns what it printed to stderr (where its analysis
/// filters report).
pub fn ffmpeg_report(ffmpeg: &str, args: &[String]) -> Result<String, String> {
    let out = command(ffmpeg).args(args).output().map_err(|e| format!("Couldn't run ffmpeg: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stderr).into_owned())
}
