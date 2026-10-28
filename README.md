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
   command is a few lines. Only want new export formats? Then you need no
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
| [scene-split](scene-split) | **Edit → Split at Scene Changes** (⌘⇧K): cuts a clip wherever the picture changes scene. One undo step. | Python, ffmpeg's scene detection |

Two more come built into splice9000 itself, and can be removed there:
**Export Presets** (YouTube, vertical, square, GIF, ProRes, WebM, sound only)
and **Frame Grab** (save the frame under the playhead as a PNG). Their
sources are in [splice9000/adapters](https://github.com/solacecasino/splice9000/tree/main/adapters).

Python adapters need Python 3: macOS installs it with its developer tools,
most Linux systems have it, and on Windows get it from
[python.org](https://www.python.org/downloads/).

## The protocol

An adapter is a folder with an `adapter.toml` and, usually, a program in any
language. splice9000 starts the program when it's needed and exchanges one
JSON message per line with it over stdin and stdout. Everything is in
splice9000's [docs/adapters.md](https://github.com/solacecasino/splice9000/blob/main/docs/adapters.md).

## Build

```sh
python3 scripts/build.py --root <folder>    # zips the adapter(s) in a folder into dist/
python3 scripts/test_scene_split.py         # runs Scene Split on a made-up clip
python3 scripts/catalog.py                  # rebuilds CATALOG.md from catalog/
```

## License

[MIT](LICENSE)
