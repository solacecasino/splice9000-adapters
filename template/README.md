# My Adapter

A [splice9000](https://github.com/solacecasino/splice9000) adapter. Replace
this with what yours does.

---

## Making your own from this template

1. **Copy this folder** into a new repo of its own (the repo's name becomes
   your zip's name, such as `my-adapter`), including `.github/`.
2. **Edit it.** Change `adapter.toml` (name, description, commands, panels)
   and `main.py`, and put your name in a `LICENSE`. `splice.py` does the
   talking to splice9000, so a command is a few lines:

   ```python
   @splice.command("count")
   def count():
       edit = splice.edit()
       splice.notify(f"{len(edit['clips'])} clips")
   ```

   Everything an adapter can do is in
   [docs/adapters.md](https://github.com/solacecasino/splice9000/blob/main/docs/adapters.md)
   and in the comments in `splice.py`. Only want new export formats? You
   need no program at all: see the `[[exports]]` example in `adapter.toml`.
3. **Try it.** Drop the folder on splice9000's window (or **Adapters →
   Install from a folder instead…**). Drop it again after each change.
   `splice.log(...)` writes to splice9000's Log panel.
4. **Release it.** Set `version` in `adapter.toml`, then:

   ```sh
   git tag v0.1.0 && git push origin v0.1.0
   ```

   GitHub Actions publishes a release with your zip. A Rust adapter (add a
   `Cargo.toml` and set `run = ["./your-crate-name"]`) gets a zip for every
   kind of computer automatically.
5. **Get it listed** in the [catalog](https://github.com/solacecasino/splice9000-adapters/blob/main/CATALOG.md):
   fill in the [List my adapter](https://github.com/solacecasino/splice9000-adapters/issues/new?template=list-my-adapter.yml)
   form with your repo's name. A check downloads your release, and if it
   installs cleanly, opens the listing for you.
