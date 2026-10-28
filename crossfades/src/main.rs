//! Crossfades: blends one clip into the next across a cut. splice9000
//! plays and exports the fade (protocol 3's `fade` on a clip); this
//! adapter adds and removes them. The length is kept in the config folder.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

use serde_json::{Value, json};

const DEFAULT_LENGTH: f64 = 0.5;

fn main() {
    splice::run(|host, command| {
        if command == "length" {
            let now = length();
            let answer = host.request(
                "ui.prompt",
                json!({ "title": "Crossfade Length", "message": "Seconds each new crossfade lasts:", "default": format!("{now}") }),
            )?;
            let Some(text) = answer.as_str() else { return Ok(()) };
            let secs: f64 = text.trim().trim_end_matches('s').trim().parse().map_err(|_| format!("\"{text}\" isn't a number of seconds"))?;
            if !(0.04..=10.0).contains(&secs) {
                return Err("A crossfade can be 0.04 to 10 seconds.".into());
            }
            save_length(secs)?;
            host.notify(&format!("New crossfades last {secs} s"));
            return Ok(());
        }
        let edit = host.edit()?;
        let mut clips = splice::plain_clips(&edit);
        if clips.len() < 2 {
            return Err("There's no cut to crossfade: split a clip or add another.".into());
        }
        let len = length();
        let message = match command {
            "add" | "remove" => {
                let i = nearest_cut(&edit).ok_or("There's no cut to crossfade.")?;
                let fade = if command == "add" { len } else { 0.0 };
                clips[i]["fade"] = json!(fade);
                if command == "add" { describe(&edit, i, len) } else { "Crossfade removed".into() }
            }
            "all" | "none" => {
                let fade = if command == "all" { len } else { 0.0 };
                for c in clips.iter_mut().skip(1) {
                    c["fade"] = json!(fade);
                }
                if command == "all" { format!("Crossfaded {} cuts", clips.len() - 1) } else { "Crossfades removed".into() }
            }
            _ => return Ok(()),
        };
        host.set_clips(Value::Array(clips))?;
        host.notify(&message);
        Ok(())
    });
}

/// The clip starting at the cut nearest the playhead (never the first).
fn nearest_cut(edit: &Value) -> Option<usize> {
    let t = edit["playhead"].as_f64()?;
    let clips = edit["clips"].as_array()?;
    (1..clips.len()).min_by(|&a, &b| {
        let d = |i: usize| (clips[i]["start"].as_f64().unwrap_or(0.0) - t).abs();
        d(a).total_cmp(&d(b))
    })
}

/// What the fade into clip `i` will be: splice9000 keeps a fade within
/// half of either clip, and holds a picture where a file has no more.
fn describe(edit: &Value, i: usize, asked: f64) -> String {
    let clips = &edit["clips"];
    let (a, b) = (&clips[i - 1], &clips[i]);
    let len = |c: &Value| c["src_out"].as_f64().unwrap_or(0.0) - c["src_in"].as_f64().unwrap_or(0.0);
    let d = asked.min(len(a) / 2.0).min(len(b) / 2.0);
    let media_len = |c: &Value| edit["media"][c["media"].as_u64().unwrap_or(0) as usize]["duration"].as_f64().unwrap_or(0.0);
    let after_a = media_len(a) - a["src_out"].as_f64().unwrap_or(0.0);
    let before_b = b["src_in"].as_f64().unwrap_or(0.0);
    let mut text = format!("Crossfade, {d:.2} s");
    if d < asked - 1e-6 {
        text.push_str(" (half the shorter clip)");
    }
    if after_a < d / 2.0 - 1e-6 || before_b < d / 2.0 - 1e-6 {
        text.push_str(": the picture holds where a clip has no more footage past the cut");
    }
    text
}

fn length_file() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(std::env::var_os("SPLICE9000_CONFIG_DIR")?).join("crossfades.txt"))
}

fn length() -> f64 {
    length_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| t.trim().parse().ok())
        .filter(|s: &f64| (0.04..=10.0).contains(s))
        .unwrap_or(DEFAULT_LENGTH)
}

fn save_length(secs: f64) -> Result<(), String> {
    let f = length_file().ok_or("no config folder to keep the length in")?;
    std::fs::write(f, format!("{secs}\n")).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(playhead: f64) -> Value {
        json!({
            "playhead": playhead,
            "media": [{ "duration": 10.0 }, { "duration": 4.0 }],
            "clips": [
                { "media": 0, "src_in": 0.0, "src_out": 10.0, "start": 0.0 },
                { "media": 1, "src_in": 1.0, "src_out": 3.5, "start": 10.0 },
                { "media": 0, "src_in": 2.0, "src_out": 2.6, "start": 12.5 },
            ],
        })
    }

    #[test]
    fn picks_the_nearest_cut() {
        assert_eq!(nearest_cut(&edit(0.0)), Some(1));
        assert_eq!(nearest_cut(&edit(11.0)), Some(1));
        assert_eq!(nearest_cut(&edit(11.5)), Some(2));
        assert_eq!(nearest_cut(&edit(99.0)), Some(2));
    }

    #[test]
    fn says_what_the_fade_will_be() {
        assert_eq!(describe(&edit(0.0), 1, 0.5), "Crossfade, 0.50 s: the picture holds where a clip has no more footage past the cut");
        assert_eq!(describe(&edit(0.0), 2, 1.0), "Crossfade, 0.30 s (half the shorter clip)");
    }
}
