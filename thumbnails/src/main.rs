//! Thumbnails: pictures from each media file along its clips in
//! splice9000's timeline. ffmpeg makes them all in one run, as one grid
//! picture kept in the config folder's cache, so a file is only ever read
//! once. Long files are read a keyframe at a time, which is many times
//! faster than decoding every frame.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Each picture's height in pixels: sharp on a 2x screen's timeline.
const HEIGHT: u32 = 72;
/// At most this many pictures a file.
const MOST: f64 = 120.0;
const COLUMNS: u32 = 12;
/// Files longer than this (seconds) are read a keyframe at a time.
const LONG: f64 = 300.0;

fn main() {
    let mut done = HashSet::new();
    splice::serve(|host, event| {
        let media = match event {
            splice::Event::Start | splice::Event::Happened(_) => host.edit()?["media"].clone(),
            splice::Event::Command(_) => return Ok(()),
        };
        let ffmpeg = host.ffmpeg();
        for m in media.as_array().into_iter().flatten() {
            let Some(path) = m["path"].as_str().map(PathBuf::from) else { continue };
            if !done.insert(path.clone()) {
                continue;
            }
            let (ffmpeg, m) = (ffmpeg.clone(), m.clone());
            std::thread::spawn(move || match Grid::of(&m).and_then(|g| g.make(&ffmpeg, &path).map(|image| (g, image))) {
                Ok((g, image)) => splice::post(
                    "media.thumbnails",
                    json!({ "path": path, "image": image, "every": g.every, "width": g.w, "height": HEIGHT, "columns": COLUMNS, "count": g.count }),
                ),
                Err(e) => splice::post("log", json!({ "message": format!("{}: {e}", path.display()) })),
            });
        }
        Ok(())
    });
}

/// How a file's pictures are laid out.
#[derive(Debug, PartialEq)]
struct Grid {
    /// Seconds between pictures.
    every: f64,
    /// Each picture's width (even, for ffmpeg).
    w: u32,
    count: u32,
    duration: f64,
}

impl Grid {
    fn of(m: &Value) -> Result<Self, String> {
        let duration = m["duration"].as_f64().filter(|d| *d > 0.0).ok_or("no length")?;
        let (w, h) = (m["width"].as_f64().unwrap_or(16.0), m["height"].as_f64().unwrap_or(9.0).max(1.0));
        let every = (duration / MOST).max(0.5);
        Ok(Self { every, w: ((HEIGHT as f64 * w / h / 2.0).round() as u32 * 2).clamp(2, 4 * HEIGHT), count: (duration / every).ceil().max(1.0) as u32, duration })
    }

    /// The grid picture's path, from the cache or made (and cached).
    fn make(&self, ffmpeg: &str, path: &Path) -> Result<PathBuf, String> {
        let out = cache_file(path).ok_or("no config folder to keep pictures in")?;
        if out.is_file() {
            return Ok(out);
        }
        std::fs::create_dir_all(out.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
        let part = out.with_extension("part.png");
        let rows = self.count.div_ceil(COLUMNS);
        let mut cmd = splice::command(ffmpeg);
        cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
        if self.duration > LONG {
            cmd.args(["-skip_frame", "nokey"]);
        }
        cmd.arg("-i").arg(path).args(["-an", "-sn", "-dn", "-vf"]);
        // Picture k is the frame at k * every: what the timeline shows from
        // there to the next.
        cmd.arg(format!("fps=1/{e:.6},scale={w}:{HEIGHT}:flags=bilinear,setsar=1,tile={COLUMNS}x{rows}", e = self.every, w = self.w));
        cmd.args(["-frames:v", "1", "-update", "1"]).arg(&part);
        let status = cmd.stderr(std::process::Stdio::null()).status().map_err(|e| format!("couldn't run ffmpeg: {e}"))?;
        if !status.success() || !part.is_file() {
            return Err(format!("ffmpeg couldn't make pictures ({status})"));
        }
        std::fs::rename(&part, &out).map_err(|e| e.to_string())?;
        Ok(out)
    }
}

/// Named for the file's path, size and time, so an edited file is read
/// again.
fn cache_file(path: &Path) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let meta = std::fs::metadata(path).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (path, meta.len(), meta.modified().ok(), HEIGHT, COLUMNS).hash(&mut h);
    let dir = PathBuf::from(std::env::var_os("SPLICE9000_CONFIG_DIR")?);
    Some(dir.join("cache").join("thumbnails").join(format!("{:016x}.png", h.finish())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_the_grid() {
        let g = Grid::of(&json!({ "duration": 10.0, "width": 1920, "height": 1080 })).unwrap();
        assert_eq!(g, Grid { every: 0.5, w: 128, count: 20, duration: 10.0 });
        let long = Grid::of(&json!({ "duration": 3600.0, "width": 1080, "height": 1920 })).unwrap();
        assert_eq!((long.every, long.w, long.count), (30.0, 40, 120));
        assert!(Grid::of(&json!({ "duration": 0 })).is_err());
    }
}
