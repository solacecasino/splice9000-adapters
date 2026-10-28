//! Waveforms: draws each media file's sound on its clips in splice9000's
//! timeline. The sound is read once, with the same ffmpeg splice9000 uses,
//! into 100 peaks a second, and kept in the config folder's cache, so a
//! file is only ever read once.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::json;

/// Peaks per second of sound.
const PER_SECOND: usize = 100;
/// The sound is read at this rate: plenty for drawing, little to decode.
const SAMPLE_RATE: usize = 8000;

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
            if !m["audio"].as_bool().unwrap_or(false) || !done.insert(path.clone()) {
                continue;
            }
            // Each file in the background, so a long one holds nothing up.
            let ffmpeg = ffmpeg.clone();
            std::thread::spawn(move || match peaks(&ffmpeg, &path) {
                Ok(peaks) => splice::post("media.waveform", json!({ "path": path, "per_second": PER_SECOND, "peaks": peaks })),
                Err(e) => splice::post("log", json!({ "message": format!("{}: {e}", path.display()) })),
            });
        }
        Ok(())
    });
}

/// The file's peaks, from the cache or worked out (and cached).
fn peaks(ffmpeg: &str, path: &Path) -> Result<Vec<u8>, String> {
    let cache = cache_file(path);
    if let Some(c) = &cache
        && let Ok(bytes) = std::fs::read(c)
    {
        return Ok(bytes);
    }
    let mut child = splice::command(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"])
        .arg(path)
        .args(["-vn", "-sn", "-dn", "-ac", "1", "-ar", &SAMPLE_RATE.to_string(), "-f", "s16le", "-"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("couldn't run ffmpeg: {e}"))?;
    let mut out = child.stdout.take().ok_or("no output")?;
    let mut summer = Peaks::default();
    let mut buf = vec![0u8; 64 * 1024];
    let mut have = 0;
    loop {
        let n = out.read(&mut buf[have..]).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        have += n;
        let whole = have / 2 * 2;
        for b in buf[..whole].as_chunks::<2>().0 {
            summer.add(i16::from_le_bytes([b[0], b[1]]));
        }
        buf.copy_within(whole..have, 0);
        have -= whole;
    }
    let _ = child.wait();
    let peaks = summer.finish();
    if let Some(c) = cache {
        let _ = std::fs::create_dir_all(c.parent().unwrap_or(Path::new(".")));
        let _ = std::fs::write(c, &peaks);
    }
    Ok(peaks)
}

/// The loudest sample in each 1/100 s, 0 to 255. The square root lifts
/// quiet sound into view, as ears hear it, without flattening loud sound.
#[derive(Default)]
struct Peaks {
    out: Vec<u8>,
    loudest: u16,
    n: usize,
}

impl Peaks {
    fn add(&mut self, s: i16) {
        self.loudest = self.loudest.max(s.unsigned_abs());
        self.n += 1;
        if self.n == SAMPLE_RATE / PER_SECOND {
            self.push();
        }
    }

    fn push(&mut self) {
        let v = (self.loudest as f64 / 32768.0).sqrt() * 255.0;
        self.out.push(v.round().min(255.0) as u8);
        self.loudest = 0;
        self.n = 0;
    }

    fn finish(mut self) -> Vec<u8> {
        if self.n > 0 {
            self.push();
        }
        self.out
    }
}

/// Where a file's peaks are kept: named for its path, size and time, so an
/// edited file is read again.
fn cache_file(path: &Path) -> Option<PathBuf> {
    use std::hash::{Hash, Hasher};
    let meta = std::fs::metadata(path).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (path, meta.len(), meta.modified().ok()).hash(&mut h);
    let dir = PathBuf::from(std::env::var_os("SPLICE9000_CONFIG_DIR")?);
    Some(dir.join("cache").join("waveforms").join(format!("{:016x}.peaks", h.finish())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_per_hundredth() {
        let mut p = Peaks::default();
        for i in 0..(SAMPLE_RATE / PER_SECOND * 2 + 3) {
            p.add(if i == 5 { -32768 } else if i < 80 { 0 } else { 8192 });
        }
        assert_eq!(p.finish(), [255, 128, 128]);
    }
}
