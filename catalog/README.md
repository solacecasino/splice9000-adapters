# The adapter catalog

Each file here lists one adapter in [CATALOG.md](../CATALOG.md). To add
yours, add a file named after it, such as `scene-split.toml`:

```toml
repo = "you/scene-split"   # a public GitHub repo whose latest release has its zip(s)
category = "Editing"       # optional: Editing, Export, Audio, Look, Tools, ...
```

That's all. Everything else (name, description, author, version,
permissions, what it adds, and the download links) is read from your latest
release, so the catalog stays current every time you release. A check runs
on your pull request to make sure the zips install.

Optional keys:

```toml
adapter = "scene-split"    # the zip name prefix, if the release holds several adapters
tags = ["cuts", "scenes"]  # extra words people might search for
```

Release zips are named `<adapter>-<version>-<platform>.zip`, with platform
one of `macos-arm64`, `macos-x64`, `windows-x64`, `windows-arm64`,
`linux-x64` or `linux-arm64`, or `<adapter>-<version>.zip` for script
adapters and presets that run everywhere. The [template](../template) makes
them for you when you push a version tag.
