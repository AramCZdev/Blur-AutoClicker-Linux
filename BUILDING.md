# Building From Source

This fork targets **Linux desktops using X11 or XWayland**. Native Wayland support is not
implemented. The original Windows project is maintained at
[Blur009/Blur-AutoClicker](https://github.com/Blur009/Blur-AutoClicker).

> [!WARNING]
> The Linux port is still a work in progress. Some input, hotkey, and window-management
> paths are represented by temporary no-op compatibility stubs while their X11
> implementations are being developed. A successful build does **not** mean that
> clicking or global hotkeys are fully functional yet. See
> [the Linux port plan](docs/linux-port-plan.md) for the implementation status.

## Prerequisites

On Ubuntu or Debian, install the native Tauri, WebKitGTK, and X11 development packages:

```bash
sudo apt install build-essential curl file libayatana-appindicator3-dev \
  libgdk-pixbuf-2.0-dev libglib2.0-dev libgtk-3-dev libssl-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev patchelf \
  libx11-dev libxext-dev libxi-dev libxrandr-dev libxtst-dev
```

You will also need:

- Node.js 24 or newer
- Rust 1.97.1 or newer (install through [rustup](https://rustup.rs/))
- npm

## Setup

```bash
git clone https://github.com/AramCZdev/Blur-AutoClicker-Linux.git
cd Blur-AutoClicker-Linux
npm ci
```

## Run in development

```bash
npm run dev
```

Run the application from an X11 session, or from an XWayland-compatible desktop session.
The current port does not implement native Wayland input.

## Build release packages

```bash
npm run build
```

Tauri is configured to produce Debian and AppImage bundles. The output is written under
`src-tauri/target/release/bundle/`.

## Validation

Run the same checks used by the Rust CI job:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked
```

The frontend checks are:

```bash
npm run lint
npm run format:check
npm run frontend:build
npm test
npm audit
```

Or run the repository's combined quality gate:

```bash
npm run check
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines. Please include the
Linux desktop environment (X11 or XWayland), relevant logs, and the exact validation
commands you ran when reporting a Linux-specific issue.
