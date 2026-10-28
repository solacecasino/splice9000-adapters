//! Silence Cut: removes the silent stretches from the selected clip (or
//! the one under the playhead), leaving a little room around the sound so
//! words aren't clipped. One undo step.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

use serde_json::{Value, json};

/// Quieter than this counts as silence.
const NOISE_DB: i32 = -35;
/// Silences shorter than this (seconds) are pauses, and stay.
const MIN_SILENCE: f64 = 0.6;
/// Kept on each side of the sound, so words don't start or end abruptly.
const PAD: f64 = 0.12;
/// Bits of sound shorter than this are noise, and go.
const SHORTEST: f64 = 0.2;

fn main() {
    splice::run(|host, command| {
        if command != "cut" {
            return Ok(());
        }
        let edit = host.edit()?;
        let i = splice::target_clip(&edit).ok_or("Add a clip to the timeline first.")?;
        let clip = &edit["clips"][i];
        let (media, src_in, src_out) = (clip["media"].as_u64().unwrap_or(0), num(&clip["src_in"]), num(&clip["src_out"]));
        let file = &edit["media"][media as usize];
        if file["audio"] == json!(false) {
            return Err("That clip has no sound to find silences in.".into());
        }
        let path = file["path"].as_str().unwrap_or_default().to_owned();
        host.notify("Listening for silences…");
        let len = src_out - src_in;
        let args: Vec<String> = [
            "-hide_banner", "-nostdin", "-ss", &format!("{src_in:.6}"), "-t", &format!("{len:.6}"), "-i", &path,
            "-vn", "-af", &format!("silencedetect=noise={NOISE_DB}dB:d={MIN_SILENCE}"), "-f", "null", "-",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let report = splice::ffmpeg_report(&host.ffmpeg(), &args)?;
        let keep = sounds(&silences(&report, len), len);
        if keep.len() == 1 && keep[0].0 <= 1e-6 && keep[0].1 >= len - 1e-6 {
            host.notify("No silences found in that clip.");
            return Ok(());
        }
        if keep.is_empty() {
            return Err("That clip is all silence; nothing was cut.".into());
        }
        let removed: f64 = len - keep.iter().map(|(a, b)| b - a).sum::<f64>();
        let pieces: Vec<Value> = keep.iter().map(|(a, b)| json!({ "media": media, "src_in": src_in + a, "src_out": src_in + b })).collect();
        let mut clips = splice::plain_clips(&edit);
        clips.splice(i..=i, pieces);
        host.set_clips(Value::Array(clips))?;
        let cuts = keep.len().saturating_sub(1).max(1);
        host.notify(&format!("Removed {removed:.1} s of silence ({cuts} cut{}). Undo puts it back.", if cuts == 1 { "" } else { "s" }));
        Ok(())
    });
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

/// Silent spans (start, end) from silencedetect's report, within 0..len. A
/// silence still going at the end has no silence_end line.
fn silences(report: &str, len: f64) -> Vec<(f64, f64)> {
    let value = |line: &str, key: &str| line.split(key).nth(1)?.split_whitespace().next()?.trim_end_matches('|').parse::<f64>().ok();
    let mut out = Vec::new();
    let mut start = None;
    for line in report.lines() {
        if let Some(s) = value(line, "silence_start:") {
            start = Some(s.max(0.0));
        } else if let Some(e) = value(line, "silence_end:") {
            out.push((start.take().unwrap_or(0.0), e.min(len)));
        }
    }
    if let Some(s) = start {
        out.push((s, len));
    }
    out
}

/// What's left once the silences (each shrunk by PAD on both sides, so the
/// sound keeps some room) are taken out of 0..len.
fn sounds(silences: &[(f64, f64)], len: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut at = 0.0;
    for &(s, e) in silences {
        let (cut_from, cut_to) = (if s <= 0.0 { 0.0 } else { s + PAD }, if e >= len { len } else { e - PAD });
        if cut_to <= cut_from {
            continue;
        }
        if cut_from - at >= SHORTEST {
            out.push((at, cut_from));
        }
        at = cut_to;
    }
    if len - at >= SHORTEST {
        out.push((at, len));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_silencedetect() {
        let report = "[silencedetect @ 0x1] silence_start: 1.0\n[silencedetect @ 0x1] silence_end: 2.5 | silence_duration: 1.5\n[silencedetect @ 0x1] silence_start: 3.2\n";
        assert_eq!(silences(report, 4.0), [(1.0, 2.5), (3.2, 4.0)]);
    }

    #[test]
    fn keeps_the_sound_with_room_around_it() {
        let keep = sounds(&[(1.0, 2.5), (3.2, 4.0)], 4.0);
        let r = |v: f64| (v * 100.0).round() / 100.0;
        let keep: Vec<(f64, f64)> = keep.iter().map(|&(a, b)| (r(a), r(b))).collect();
        assert_eq!(keep, [(0.0, 1.12), (2.38, 3.32)]);
        // Silence from the start: nothing kept before it.
        assert_eq!(sounds(&[(0.0, 1.0)], 3.0)[0].0, 1.0 - PAD);
        // No silences: everything.
        assert_eq!(sounds(&[], 3.0), [(0.0, 3.0)]);
        // All silence: nothing.
        assert!(sounds(&[(0.0, 3.0)], 3.0).is_empty());
    }
}
