# Prism logo

`prism-logo.png` is the 1024-pixel square, opaque sRGB master for Windows, Linux and the shared UI. The glass prism sits on pure black with safety padding and no external light beams. Native `sips` export added the final padding, resolution and sRGB colour metadata.

`prism-macos.png` is the dedicated 1024-pixel RGBA artwork for the macOS bundle. It contains a rounded black tile with transparent exterior padding and a larger glass mark relative to the tile. This silhouette is part of the artwork: the existing ICNS bundle path does not apply an icon mask. The built-in image generation tool edited the shared master; native resizing preserves its exterior alpha.

Run `python3 scripts/app-icons.py` on macOS to regenerate every derived asset. The script uses macOS `sips` and `iconutil` plus Python's standard library. Generated assets are kept with source, so ordinary Rust builds need no image tools.

| Consumer | Asset | Resolutions |
| --- | --- | --- |
| Shared UI and Windows/Linux native window | `prism-icon.png` | 256 × 256 |
| macOS runtime Dock icon | `prism-macos-icon.png` | 256 × 256 |
| macOS bundle | `Prism.icns` | 16, 32, 128, 256 and 512 points, each at 1× and 2× |
| Windows executable | `Prism.ico` | 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 128 and 256 pixels |
| Linux desktop | `linux/hicolor/<size>x<size>/apps/io.github.nanashili.prism.png` | 16, 24, 32, 48, 64, 128, 256, 512 and 1024 pixels |

The macOS bundle script installs `Prism.icns` and declares `CFBundleIconFile`. The desktop entrypoint selects `prism-macos-icon.png` on macOS because eframe also sets the application icon at runtime; it must preserve the same rounded silhouette as the bundle. Windows/Linux retain their existing compact window PNG. The Windows desktop build embeds `Prism.ico` into the `prism` executable through `resources/windows/prism.rc`; it requires an available Windows resource compiler and fails explicitly if embedding fails. The MCP-only build skips this resource.

Linux packagers should install the contents of `linux/hicolor/` under their prefix's `share/icons/hicolor/`, and `../linux/io.github.nanashili.prism.desktop` under `share/applications/`. The launcher, icon name and window application ID all use `io.github.nanashili.prism`; its `Exec` expects `prism` on `PATH`. The host supplies the hicolor theme definition. These are desktop application assets, not MSIX/Store tiles or mobile launch icons.

Requirements: [Apple icon sets](https://developer.apple.com/library/archive/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/Optimizing/Optimizing.html), [Windows desktop icon sizes](https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-construction), [freedesktop icon theme](https://specifications.freedesktop.org/icon-theme/latest/) and [desktop entry](https://specifications.freedesktop.org/desktop-entry/latest-single/) specifications. Windows and Linux builds remain unverified on this Mac.

The original generated edits and exact prompts are retained under `assets/branding/prism-logo-black-v4.*` and `assets/branding/prism-logo-macos-v5.*`. Only the compact PNG is embedded by the UI; the other platform assets add no runtime image cache.
