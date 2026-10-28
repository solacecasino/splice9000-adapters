"""Scene Split: cuts the selected clip (or the one under the playhead)
wherever ffmpeg sees the picture change scene. One undo step."""

import re
import subprocess

import splice

#: How different two frames must be to count as a new scene (0 to 1).
THRESHOLD = 0.3
#: Shorter pieces than this (seconds) are left joined to the one before.
SHORTEST = 0.5


def scene_changes(path, start, length):
    """Seconds, from `start`, where a new scene begins."""
    cmd = [splice.ffmpeg(), "-hide_banner", "-nostdin", "-ss", f"{start:.6f}", "-t", f"{length:.6f}", "-i", path,
           "-an", "-vf", f"select='gt(scene,{THRESHOLD})',showinfo", "-f", "null", "-"]
    out = subprocess.run(cmd, capture_output=True, text=True, errors="replace").stderr
    return [float(t) for t in re.findall(r"pts_time:([0-9.]+)", out)]


@splice.command("split")
def split():
    edit = splice.edit()
    clips = edit["clips"]
    if not clips:
        splice.notify("Add a clip to the timeline first.")
        return
    i = edit["selected"]
    if i is None:
        t = edit["playhead"]
        i = next((n for n, c in reversed(list(enumerate(clips))) if c["start"] <= t), 0)
    clip = clips[i]
    path = edit["media"][clip["media"]]["path"]
    splice.notify("Looking for scene changes…")
    cuts = [clip["src_in"] + t for t in scene_changes(path, clip["src_in"], clip["src_out"] - clip["src_in"])]

    pieces, at = [], clip["src_in"]
    for cut in cuts + [clip["src_out"]]:
        if cut - at >= SHORTEST or cut == clip["src_out"]:
            pieces.append({"media": clip["media"], "src_in": at, "src_out": cut})
            at = cut
    if len(pieces) < 2:
        splice.notify("No scene changes found in that clip.")
        return
    keep = [{"media": c["media"], "src_in": c["src_in"], "src_out": c["src_out"]} for c in clips]
    splice.set_clips(keep[:i] + pieces + keep[i + 1:])
    splice.notify(f"Split into {len(pieces)} scenes. Undo puts it back.")


splice.run()
