"""Runs Scene Split the way splice9000 does, on a made-up clip with three
scenes, and checks it cuts at the scene changes.

    python3 scripts/test_scene_split.py
"""
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

ffmpeg = shutil.which("ffmpeg") or exit("needs ffmpeg")
clip = Path(tempfile.mkdtemp()) / "three-scenes.mp4"
subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=s=640x360:r=25:d=2",
                "-f", "lavfi", "-i", "mandelbrot=s=640x360:r=25", "-f", "lavfi", "-i", "smptebars=s=640x360:r=25:d=2",
                "-filter_complex", "[1]trim=duration=2,setpts=PTS-STARTPTS[m];[0][m][2]concat=n=3:v=1:a=0",
                "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", str(clip)], check=True)

folder = Path(__file__).resolve().parent.parent / "scene-split"
p = subprocess.Popen(["python3", "main.py"], cwd=folder, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def send(m):
    p.stdin.write(json.dumps(m) + "\n")
    p.stdin.flush()

send({"method": "initialize", "params": {"protocol": 1, "ffmpeg": ffmpeg, "permissions": ["edit.read", "edit.change"]}})
send({"method": "command", "params": {"command": "split"}})
edit = {"playhead": 1.0, "selected": None, "duration": 6.0, "media": [{"path": str(clip), "duration": 6.0, "fps": 25.0}],
        "clips": [{"media": 0, "src_in": 0.0, "src_out": 6.0, "start": 0.0}]}
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
        if "Split into" in m["params"]["text"] or "No scene" in m["params"]["text"]:
            break
send({"method": "shutdown"})
p.wait(timeout=10)
cuts = [(round(c["src_in"], 1), round(c["src_out"], 1)) for c in clips]
assert cuts == [(0.0, 2.0), (2.0, 4.0), (4.0, 6.0)], cuts
print("Scene Split cut at 2.0 and 4.0 s: ok")
