"""splice.py: write a splice9000 adapter in a few lines of Python.

Copy this one file next to your main.py. No installing, no dependencies.

    import splice

    @splice.command("count")          # the `id` of a [[commands]] entry in adapter.toml
    def count():
        edit = splice.edit()
        splice.notify(f"{len(edit['clips'])} clips, {edit['duration']:.1f} s")

    @splice.on("changed")             # an event listed in adapter.toml's `events`
    def changed(event):
        splice.panel("stats", [f"{event['clips']} clips"])

    splice.run()

The protocol underneath is documented at
https://github.com/solacecasino/splice9000/blob/main/docs/adapters.md
"""

import json
import os
import sys

__all__ = [
    "SpliceError", "command", "on", "run", "request", "send", "info", "ffmpeg", "ffprobe",
    "edit", "set_clips", "import_media", "seek", "notify", "panel", "log",
    "confirm", "prompt", "save_dialog", "open_dialog",
]

#: What splice9000 said when it started this adapter: version, protocol,
#: permissions, and the paths of its ffmpeg and ffprobe.
info = {}

_commands = {}
_events = {}
_next_id = 0
_backlog = []


class SpliceError(Exception):
    """splice9000 refused a request, e.g. for a missing permission."""


# ---- Registering handlers ---------------------------------------------------


def command(command_id):
    """Runs the decorated function when that command is chosen."""
    def register(fn):
        _commands[command_id] = fn
        return fn
    return register


def on(kind):
    """Runs the decorated function with the event's fields as a dict, for
    "changed", "exported" or "saved"."""
    def register(fn):
        _events[kind] = fn
        return fn
    return register


# ---- Talking to splice9000 --------------------------------------------------


def send(method, **params):
    """Sends a notification (no reply)."""
    _write({"method": method, "params": params})


def request(method, **params):
    """Sends a request and waits for splice9000's answer."""
    global _next_id
    _next_id += 1
    my_id = _next_id
    _write({"id": my_id, "method": method, "params": params})
    for line in sys.stdin:
        message = json.loads(line)
        if message.get("id") == my_id and "method" not in message:
            if "error" in message:
                raise SpliceError(message["error"])
            return message.get("result")
        _backlog.append(message)  # handled once this request is answered
    sys.exit(0)  # splice9000 closed the connection


def ffmpeg():
    """The ffmpeg splice9000 uses, so your adapter needs no copy of its own."""
    return info.get("ffmpeg") or os.environ.get("SPLICE9000_FFMPEG") or "ffmpeg"


def ffprobe():
    return info.get("ffprobe") or os.environ.get("SPLICE9000_FFPROBE") or "ffprobe"


def edit():
    """The edit: "format", "duration", "playhead", "selected", "file",
    "media" (each with path, duration, width, height, fps, audio) and
    "clips" (each with media, src_in, src_out and its start on the timeline).
    Needs the "edit.read" permission."""
    return request("edit.get")


def set_clips(clips):
    """Replaces the timeline with these clips ({"media", "src_in", "src_out"},
    and "fade", seconds of crossfade from the clip before, from 0.0.7),
    as one step people can undo. Needs "edit.change"."""
    request("edit.set", clips=clips)


def import_media(paths, append=False):
    """Adds media files; with append, also to the end of the timeline.
    Needs "edit.change"."""
    request("edit.import", paths=[str(p) for p in paths], append=append)


def seek(seconds):
    """Moves the playhead. Needs "edit.change"."""
    return request("playhead.set", time=seconds)


def playback(rate):
    """Plays at `rate`: 1 real time, 2 twice as fast, 0 stopped, below 0
    backward (splice9000 0.0.7). Returns the rate it plays at."""
    return request("playback.set", rate=rate)


def notify(text):
    """Shows a short message for a few seconds."""
    request("notify", text=text)


def panel(panel_id, lines):
    """Fills one of your [[panels]]. A line is a string, or
    {"text": ..., "command": ...} for a button that runs one of your commands."""
    request("panel.set", id=panel_id, lines=lines)


def log(message):
    """Writes to splice9000's Log panel."""
    send("log", message=str(message))


def confirm(title, message):
    """Asks a yes/no question. True for yes."""
    return bool(request("ui.confirm", title=title, message=message))


def prompt(title, message, default=""):
    """Asks for a line of text. None if cancelled."""
    return request("ui.prompt", title=title, message=message, default=default)


def save_dialog(title, file_name, extension):
    """The system's Save dialog. The chosen path, or None if cancelled."""
    return request("ui.save_dialog", title=title, file_name=file_name, extension=extension)


def open_dialog(title, extensions=()):
    """The system's Open dialog. The chosen path, or None if cancelled."""
    return request("ui.open_dialog", title=title, extensions=list(extensions))


# ---- The loop ---------------------------------------------------------------


def run():
    """Handles messages from splice9000 until it says goodbye."""
    while True:
        message = _backlog.pop(0) if _backlog else _read()
        if message is None:
            return
        method = message.get("method")
        params = message.get("params") or {}
        try:
            if method == "initialize":
                info.update(params)
            elif method == "command":
                fn = _commands.get(params.get("command"))
                if fn:
                    fn()
            elif method == "event":
                fn = _events.get(params.get("kind"))
                if fn:
                    fn(params)
            elif method == "shutdown":
                return
        except SpliceError as e:
            notify(str(e))
        except Exception as e:  # keep going: one bad command shouldn't stop the adapter
            log(f"{type(e).__name__}: {e}")
            notify(f"Something went wrong: {e}")


def _read():
    line = sys.stdin.readline()
    if not line:
        return None
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return {}


def _write(message):
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()
