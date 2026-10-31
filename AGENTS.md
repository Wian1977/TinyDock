# AGENTS.md — TinyDock

Concise, machine-oriented orientation for automated agents crawling this repository.

## What this is
A lightweight bottom **dock** for Linux (GTK3 desktops: GNOME, Cinnamon, etc.),
written in **Rust + GTK3**, plus a bundled desktop analog-clock widget **tclock**
(Rust + eframe). Both autostart on login. The signature interaction is an elastic
hover zoom on dock icons and minimal window-state dots.

## Repository layout
- `src/` — TinyDock dock source (Rust/GTK3). Entry point: `src/main.rs`.
- `tclock/` — tclock clock widget (Rust/eframe) as a Cargo subproject.
- `build-deb.sh` — builds release binaries and packages a `.deb`.
- `Makefile` — `make deb`, `make install`, `make uninstall`.
- `install.sh` — installs to `/usr/local` (alternative to the `.deb`).
- `tinydock.desktop.in`, `tclock.desktop.in` — desktop-entry templates (`@BINDIR@` is substituted).
- `logo.svg` / `logo.png` — project logo: split face, left half blue / right half white; the vertical seam between them is the “nose”.
- `.github/workflows/build.yml` — GitHub Actions: builds `.deb` on `ubuntu-24.04` and `ubuntu-22.04`, publishes to the `v1.0` release on `workflow_dispatch` or tag push.
- `docs/PROMO.md` — copy for announcing the project.
- `LICENSE` — MIT.

## Build
```bash
sudo apt install -y cargo rustc libgtk-3-dev libwnck-3-dev pkg-config
cargo build --release                       # tinydock -> target/release/tinydock
(cd tclock && cargo build --release)        # tclock  -> tclock/target/release/tclock
```

## Package
```bash
make deb                              # amd64 .deb for the current distro
make deb DEB_SUFFIX=_ubuntu22         # Ubuntu 22.04 / Mint 21 (build on a 22.04 host)
# or:
./build-deb.sh 1.0
```
The two variants differ only in runtime library names (24.04 uses the `t64`
renamed `libgtk-3-0t64`; 22.04 uses `libgtk-3-0`). Dependencies are derived at
package time via `dpkg-shlibdeps`, so the produced `.deb` always matches the host.

## Install
```bash
sudo dpkg -i tinydock_1.0_amd64.deb     # or tinydock_1.0_amd64_ubuntu22.deb
sudo apt -f install                     # resolve missing deps if any
```
or `make install` (installs to `/usr/local`).

## Run / configure
- Run `tinydock` (also appears in the applications menu).
- Config lives in `~/.config/tinydock/`: `launchers.txt` (pinned desktop IDs,
one per line), `hover_scale` (1/2/3), `bar_transparent` (1/0), `dot_shape` (0 dot / 1 line).
- Right-click the empty dock area for the menu (hover scale, indicator shape, transparency, add app).

## Tests
No automated test suite. Verification is manual (build + run).

## Releasing
`.deb` artifacts are produced by CI and attached to the `v1.0` GitHub Release.
`build.yml` uses `softprops/action-gh-release@v2` with `overwrite: true`, so
re-running the workflow replaces existing assets.

## Constraints / known limits
- GTK3 + libwnck-3 only; window state is polled every 500ms via libwnck.
- Only `amd64` is build-verified. Other arches need a native rebuild (source unchanged).
- CI produces only `.deb`. RPM users use `install.sh` or build a spec manually.

## License
MIT (see `LICENSE`).
