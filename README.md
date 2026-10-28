# splice9000 adapters

Add-ons for [splice9000](https://github.com/solacecasino/splice9000), the
bare, modular video editor. splice9000 has only what you need; you add the
rest.

## 👉 [Browse the catalog](CATALOG.md)

Every adapter, including other people's, with a download for your computer.
To install one, download its zip and **drop it on the splice9000 window**.
splice9000 shows what the adapter adds and what it may do, and installs it
when you click Install. Remove it any time in the Adapters panel.
splice9000 itself never goes online: this page is where you find things.

## Make one and share it

1. **Start from the [template](template):** a working Python adapter, with
   [`splice.py`](sdk/python/splice.py), a one-file helper library, so a
   command is a few lines. Prefer Rust? [`splice.rs`](sdk/rust/splice.rs) is
   the same in one file; [silence-cut](silence-cut) shows how to use it. Only want new export formats? Then you need no
   code at all: an adapter can be just an `adapter.toml` of presets.
2. **Push a version tag.** The template's workflow publishes your release,
   with a zip for every kind of computer if it's compiled.
3. **[List it](https://github.com/solacecasino/splice9000-adapters/issues/new?template=list-my-adapter.yml):**
   type your repo's name into the form. A check downloads your release and
   opens the listing for you. Later releases show up in the catalog by
   themselves.

Prefer a pull request? Add `catalog/<name>.toml` with `repo = "you/name"`;
see [catalog/README.md](catalog/README.md).

## splice9000's own adapters

| Adapter | Adds | Built with |
|---|---|---|
| [silence-cut](silence-cut) | **Edit → Remove Silences** (⌘⇧J): takes the silent stretches out of a clip, leaving room around words. Jump cuts for talking video, one undo step. | Rust, ffmpeg's silencedetect |
| [loudness-normalize](loudness-normalize) | Export formats leveled to −14 LUFS (YouTube, Spotify), −16 LUFS (podcasts) or −23 LUFS (broadcast). Needs splice9000 0.0.6. | Presets only |
| [scene-split](scene-split) | **Edit → Split at Scene Changes** (⌘⇧K): cuts a clip wherever the picture changes scene. One undo step. | Rust, ffmpeg's scene detection |

The compiled ones come for Mac (Apple Silicon and Intel), Windows (x64 and
Arm) and Linux (x64 and Arm), and need nothing else installed.

Two more come built into splice9000 itself, and can be removed there:
**Export Presets** (YouTube, vertical, square, GIF, ProRes, WebM, sound only)
and **Frame Grab** (save the frame under the playhead as a PNG). Their
sources are in [splice9000/adapters](https://github.com/solacecasino/splice9000/tree/main/adapters).

Python adapters (such as ones made from the template) need Python 3: macOS installs it with its developer tools,
most Linux systems have it, and on Windows get it from
[python.org](https://www.python.org/downloads/).

## The protocol

An adapter is a folder with an `adapter.toml` and, usually, a program in any
language. splice9000 starts the program when it's needed and exchanges one
JSON message per line with it over stdin and stdout. Everything is in
splice9000's [docs/adapters.md](https://github.com/solacecasino/splice9000/blob/main/docs/adapters.md).

## Build

```sh
python3 scripts/build.py --root <folder>                 # builds and zips the adapter(s) in a folder into dist/
python3 scripts/test_scene_split.py <scene-split program> # runs it on a made-up three-scene clip
python3 scripts/test_silence_cut.py <silence-cut program> # runs it on sound, silence, sound
python3 scripts/test_loudness.py                         # measures each loudness format
python3 scripts/catalog.py                               # rebuilds CATALOG.md from catalog/
```

## License

[MIT](LICENSE)
