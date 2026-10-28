//! Scene Split: cuts the selected clip (or the one under the playhead)
//! wherever ffmpeg sees the picture change scene. One undo step.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

use serde_json::{Value, json};

/// How different two frames must be to count as a new scene (0 to 1).
const THRESHOLD: f64 = 0.3;
/// Pieces shorter than this (seconds) stay joined to the one before.
const SHORTEST: f64 = 0.5;

fn main() {
    splice::run(|host, command| {
        if command != "split" {
            return Ok(());
        }
        let edit = host.edit()?;
        let i = splice::target_clip(&edit).ok_or("Add a clip to the timeline first.")?;
        let clip = &edit["clips"][i];
        let (media, src_in, src_out) = (clip["media"].as_u64().unwrap_or(0), num(&clip["src_in"]), num(&clip["src_out"]));
        let path = edit["media"][media as usize]["path"].as_str().unwrap_or_default().to_owned();
        host.notify("Looking for scene changes…");
        let args: Vec<String> = [
            "-hide_banner", "-nostdin", "-ss", &format!("{src_in:.6}"), "-t", &format!("{:.6}", src_out - src_in), "-i", &path,
            "-an", "-vf", &format!("select='gt(scene,{THRESHOLD})',showinfo"), "-f", "null", "-",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let report = splice::ffmpeg_report(&host.ffmpeg(), &args)?;
        let cuts: Vec<f64> = scene_times(&report).into_iter().map(|t| src_in + t).collect();
        let pieces = pieces(media, src_in, src_out, &cuts);
        if pieces.len() < 2 {
            host.notify("No scene changes found in that clip.");
            return Ok(());
        }
        let mut clips = splice::plain_clips(&edit);
        let n = pieces.len();
        clips.splice(i..=i, pieces);
        host.set_clips(Value::Array(clips))?;
        host.notify(&format!("Split into {n} scenes. Undo puts it back."));
        Ok(())
    });
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

/// The times (from the start of what was analysed) that showinfo reports.
fn scene_times(report: &str) -> Vec<f64> {
    report.split("pts_time:").skip(1).filter_map(|rest| rest.split_whitespace().next()?.parse().ok()).collect()
}

/// The clip from `src_in` to `src_out`, cut at `cuts`, leaving out pieces
/// shorter than SHORTEST.
fn pieces(media: u64, src_in: f64, src_out: f64, cuts: &[f64]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut at = src_in;
    for &cut in cuts.iter().chain(std::iter::once(&src_out)) {
        if cut - at >= SHORTEST || cut == src_out {
            out.push(json!({ "media": media, "src_in": at, "src_out": cut }));
            at = cut;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_showinfo_and_cuts() {
        let report = "[Parsed_showinfo_1 @ 0x1] n:   0 pts:  50 pts_time:2.000 duration:1\n[Parsed_showinfo_1 @ 0x1] n:   1 pts: 100 pts_time:4 duration:1\n";
        assert_eq!(scene_times(report), [2.0, 4.0]);
        let p = pieces(0, 1.0, 7.0, &[3.0, 3.2, 5.0]);
        let spans: Vec<(f64, f64)> = p.iter().map(|c| (num(&c["src_in"]), num(&c["src_out"]))).collect();
        assert_eq!(spans, [(1.0, 3.0), (3.0, 5.0), (5.0, 7.0)]);
    }
}
