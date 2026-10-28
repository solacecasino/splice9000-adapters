"""Runs Silence Cut the way splice9000 does, on a made-up clip: a second of
sound, a second and a half of silence, another second of sound. It must
cut out the silence and keep the sound, with a little room around it.

    python3 scripts/test_silence_cut.py <path to the silence-cut program>
"""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ffmpeg = shutil.which("ffmpeg") or sys.exit("needs ffmpeg")
program = str(Path(sys.argv[1]).resolve())
clip = Path(tempfile.mkdtemp()) / "talk.mp4"
subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=320x180:r=25:d=3.5",
                "-f", "lavfi", "-i", "sine=f=220:d=1", "-f", "lavfi", "-i", "anullsrc=r=44100:cl=mono:d=1.5",
                "-f", "lavfi", "-i", "sine=f=330:d=1",
                "-filter_complex", "[1][2][3]concat=n=3:v=0:a=1[a]", "-map", "0:v", "-map", "[a]",
                "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-c:a", "aac", str(clip)], check=True)

p = subprocess.Popen([program], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def send(m):
    p.stdin.write(json.dumps(m) + "\n")
    p.stdin.flush()

send({"method": "initialize", "params": {"protocol": 2, "ffmpeg": ffmpeg, "permissions": ["edit.read", "edit.change"]}})
send({"method": "command", "params": {"command": "cut"}})
edit = {"playhead": 0.5, "selected": None, "duration": 3.5,
        "media": [{"path": str(clip), "duration": 3.5, "fps": 25.0, "audio": True}],
        "clips": [{"media": 0, "src_in": 0.0, "src_out": 3.5, "start": 0.0}]}
clips = None
while True:
    m = json.loads(p.stdout.readline())
    if m.get("method") == "edit.get":
        send({"id": m["id"], "result": edit})
    elif m.get("method") == "edit.set":
        clips = m["params"]["clips"]
        send({"id": m["id"], "result": None})
    elif m.get("method") == "notify":
        send({"id": m["id"], "result": None})
        print("notify:", m["params"]["text"])
        if m["params"]["text"].startswith(("Removed", "No silences", "That clip")):
            break
send({"method": "shutdown"})
p.wait(timeout=10)
assert clips, "nothing was cut"
spans = [(round(c["src_in"], 2), round(c["src_out"], 2)) for c in clips]
print(spans)
assert len(spans) == 2, spans
(a0, a1), (b0, b1) = spans
assert a0 == 0.0 and 1.05 <= a1 <= 1.25, spans     # the first second, plus room
assert 2.3 <= b0 <= 2.45 and b1 == 3.5, spans      # room, then the last second
print("Silence Cut removed the silence and kept the sound: ok")
