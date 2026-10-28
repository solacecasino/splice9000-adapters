"""Checks each Loudness Normalize format really reaches its target: a quiet
recording is run through the format's filter, then measured with ffmpeg's
EBU R128 meter, which must read within 1 LU of the target.

    python3 scripts/test_loudness.py
"""
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ffmpeg = shutil.which("ffmpeg") or sys.exit("needs ffmpeg")
root = Path(__file__).resolve().parent.parent
manifest = tomllib.loads((root / "loudness-normalize/adapter.toml").read_text())
work = Path(tempfile.mkdtemp())
quiet = work / "quiet.wav"
# Eight seconds of a quiet, speech-like tone that rises and falls.
subprocess.run([ffmpeg, "-v", "error", "-y", "-f", "lavfi", "-i",
                "aevalsrc=0.03*sin(2*PI*220*t)*(0.6+0.4*sin(2*PI*0.5*t)):s=48000:d=8", str(quiet)], check=True)

def loudness(path):
    out = subprocess.run([ffmpeg, "-hide_banner", "-nostdin", "-i", str(path), "-af", "ebur128", "-f", "null", "-"],
                         capture_output=True, text=True).stderr
    return float(re.findall(r"I:\s+(-?[\d.]+) LUFS", out)[-1])

before = loudness(quiet)
print(f"before: {before:.1f} LUFS")
for e in manifest["exports"]:
    target = float(re.search(r"I=(-?[\d.]+)", e["audio_filter"]).group(1))
    out = work / f"{e['id']}.wav"
    subprocess.run([ffmpeg, "-v", "error", "-y", "-i", str(quiet), "-af", e["audio_filter"], "-ar", "48000", str(out)], check=True)
    got = loudness(out)
    print(f"{e['label']}: {got:.1f} LUFS (target {target})")
    assert abs(got - target) <= 1.0, (e["label"], got, target)
print("Every Loudness Normalize format reaches its target: ok")
