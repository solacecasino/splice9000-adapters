//! Shuttle: J/K/L playback. L plays forward and each further press doubles
//! the speed up to 8x; J does the same backward; K stops. Pressing the
//! other way first slows down, then turns around, as in most editors.

#[path = "../../sdk/rust/splice.rs"]
mod splice;

/// The fastest it goes, either way.
const FASTEST: f64 = 8.0;

fn main() {
    splice::run(|host, command| {
        let way = match command {
            "forward" => 1.0,
            "back" => -1.0,
            "stop" => 0.0,
            _ => return Ok(()),
        };
        let now = host.edit()?["rate"].as_f64().unwrap_or(0.0);
        host.playback(next(now, way))?;
        Ok(())
    });
}

/// The rate after pressing `way` (1 L, -1 J, 0 K) while playing at `now`.
fn next(now: f64, way: f64) -> f64 {
    if way == 0.0 {
        0.0
    } else if now * way > 0.0 {
        (now * 2.0).clamp(-FASTEST, FASTEST)
    } else if now.abs() > 1.0 {
        // Going fast the other way: slow down first.
        now / 2.0
    } else {
        way
    }
}

#[cfg(test)]
mod tests {
    use super::next;

    #[test]
    fn speeds_up_slows_down_and_turns() {
        assert_eq!([next(0.0, 1.0), next(1.0, 1.0), next(4.0, 1.0), next(8.0, 1.0)], [1.0, 2.0, 8.0, 8.0]);
        assert_eq!([next(0.0, -1.0), next(-2.0, -1.0), next(-8.0, -1.0)], [-1.0, -4.0, -8.0]);
        assert_eq!([next(4.0, -1.0), next(1.0, -1.0), next(-1.0, 1.0)], [2.0, -1.0, 1.0]);
        assert_eq!(next(-4.0, 0.0), 0.0);
    }
}
