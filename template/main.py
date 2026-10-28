"""My Adapter. splice.py (next to this file) handles talking to splice9000."""

import splice


def stats(edit):
    return [
        f"{len(edit['clips'])} clips",
        f"{edit['duration']:.1f} s long",
        f"{len(edit['media'])} media files",
        {"text": "Count again", "command": "count"},
    ]


@splice.command("count")
def count():
    edit = splice.edit()
    splice.panel("stats", stats(edit))
    splice.notify(f"{len(edit['clips'])} clips, {edit['duration']:.1f} seconds")


@splice.on("changed")
def changed(event):
    splice.panel("stats", stats(splice.edit()))


splice.run()
