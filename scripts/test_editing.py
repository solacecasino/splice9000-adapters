"""Runs the editing adapters (Waveforms, Thumbnails, Shuttle, Crossfades)
the way splice9000 does, on made-up clips, and checks what they send.

    python3 scripts/test_editing.py <folder holding the built programs' folders>

e.g. build/, with build/waveforms/waveforms (.exe on Windows) and so on.
"""
import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ffmpeg = shutil.which("ffmpeg") or sys.exit("needs ffmpeg")
root = Path(sys.argv[1]).resolve()
exe = ".exe" if os.name == "nt" else ""
work = Path(tempfile.mkdtemp())
config = work / "config"
config.mkdir()
env = dict(os.environ, SPLICE9000_CONFIG_DIR=str(config))

# Six seconds: a second of tone, a second of silence, and so on.
clip = work / "beeps.mp4"
subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=320x180:r=25:d=6",
                "-f", "lavfi", "-i", "sine=f=440:d=6", "-filter_complex", "[1]volume='lt(mod(t,2),1)':eval=frame[a]",
                "-map", "0:v", "-map", "[a]", "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p",
                "-c:a", "aac", str(clip)], check=True)
media = {"path": str(clip), "duration": 6.0, "width": 320, "height": 180, "fps": 25.0, "audio": True}


def edit(**over):
    e = {"playhead": 0.0, "rate": 0.0, "selected": None, "duration": 6.0, "media": [media],
         "clips": [{"media": 0, "src_in": 0.0, "src_out": 3.0, "start": 0.0, "fade": 0.0},
                   {"media": 0, "src_in": 3.0, "src_out": 6.0, "start": 3.0, "fade": 0.0}]}
    e.update(over)
    return e


def session(name, commands, state, answers=None, stop=lambda m: False, permissions=("edit.read", "edit.change")):
    """Runs one adapter: starts it, sends `commands`, answers its requests
    from `state` (the edit) and `answers`, and returns what it sent."""
    p = subprocess.Popen([str(root / name / (name + exe))], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, env=env)
    def send(m):
        p.stdin.write(json.dumps(m) + "\n")
        p.stdin.flush()
    send({"method": "initialize", "params": {"protocol": 3, "splice9000": "0.0.7", "ffmpeg": ffmpeg, "permissions": list(permissions)}})
    sent = []
    pending = list(commands)
    def next_command():
        if pending:
            send({"method": "command", "params": {"command": pending.pop(0)}})
    next_command()
    while True:
        line = p.stdout.readline()
        if not line:
            break
        m = json.loads(line)
        sent.append(m)
        method = m.get("method")
        reply = None
        if method == "edit.get":
            reply = state()
        elif method in (answers or {}):
            reply = answers[method](m["params"])
        if "id" in m:
            send({"id": m["id"], "result": reply})
        if method in ("notify", "playback.set") or stop(m):
            if stop(m) or not pending:
                break
            next_command()
    send({"method": "shutdown"})
    p.wait(timeout=20)
    return sent


# Waveforms: 100 peaks a second, loud then quiet, and kept for next time.
for attempt in ("worked out", "from the cache"):
    got = session("waveforms", [], lambda: edit(), stop=lambda m: m.get("method") in ("media.waveform", "log"))
    wave = next(m["params"] for m in got if m.get("method") == "media.waveform")
    peaks = wave["peaks"]
    assert wave["per_second"] == 100 and 590 <= len(peaks) <= 610, (wave["per_second"], len(peaks))
    loud, quiet = peaks[20:80], peaks[120:180]
    assert min(loud) > 60 and max(quiet) < 10, (loud[:5], quiet[:5])
    print(f"Waveforms, {attempt}: {len(peaks)} peaks, tone {min(loud)}-{max(loud)}, silence {max(quiet)}: ok")
assert list((config / "cache" / "waveforms").iterdir())

# Thumbnails: a grid PNG of the promised size.
got = session("thumbnails", [], lambda: edit(), stop=lambda m: m.get("method") in ("media.thumbnails", "log"))
t = next(m["params"] for m in got if m.get("method") == "media.thumbnails")
data = Path(t["image"]).read_bytes()
assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
w, h = struct.unpack(">II", data[16:24])
rows = -(-t["count"] // t["columns"])
assert (t["width"], t["height"], t["count"], t["every"]) == (128, 72, 12, 0.5), t
assert w >= t["columns"] * t["width"] and h >= rows * t["height"], (w, h, t)
print(f"Thumbnails: {t['count']} pictures of {t['width']}x{t['height']} in a {w}x{h} grid: ok")

# Shuttle: L, L, J, J from stopped: 1, 2, then slower, then backward.
rate = [0.0]
def playback(params):
    rate[0] = params["rate"]
    return rate[0]
got = session("shuttle", ["forward", "forward", "back", "back", "back", "stop"], lambda: edit(rate=rate[0]),
              answers={"playback.set": playback})
rates = [m["params"]["rate"] for m in got if m.get("method") == "playback.set"]
assert rates == [1.0, 2.0, 1.0, -1.0, -2.0, 0.0], rates
print(f"Shuttle: L L J J J K plays at {rates}: ok")

# Crossfades: at the cut, then a new length, then every cut, then none.
state = {"e": edit(playhead=2.6)}
def set_clips(params):
    state["e"] = edit(playhead=2.6, clips=[dict(c, start=[0.0, 3.0][i]) for i, c in enumerate(params["clips"])])
    return None
got = session("crossfades", ["add", "length", "add", "none"], lambda: state["e"],
              answers={"edit.set": set_clips, "ui.prompt": lambda p: "1.2"})
fades = [[c.get("fade", 0) for c in m["params"]["clips"]] for m in got if m.get("method") == "edit.set"]
assert fades == [[0.0, 0.5], [0.0, 1.2], [0.0, 0.0]], fades
assert (config / "crossfades.txt").read_text().strip() == "1.2"
notes = [m["params"]["text"] for m in got if m.get("method") == "notify"]
print(f"Crossfades: {fades}, said {notes}: ok")
