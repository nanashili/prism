#!/usr/bin/env python3
"""Export desktop icons from Prism's platform-specific artwork on macOS."""
from pathlib import Path
import struct
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
BRANDING = ROOT / "resources/branding"
APP_ID = "io.github.nanashili.prism"
WINDOWS_SIZES = (16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 128, 256)
LINUX_SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)


def resize(source, destination, size):
    destination.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["sips", "-z", str(size), str(size), str(source), "--out", str(destination)],
        check=True,
        capture_output=True,
        text=True,
    )


def macos_icon(source, temporary):
    iconset = temporary / "Prism.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            suffix = "@2x" if scale == 2 else ""
            destination = iconset / f"icon_{size}x{size}{suffix}.png"
            resize(source, destination, size * scale)

    subprocess.run(
        ["iconutil", "-c", "icns", str(iconset), "-o", str(BRANDING / "Prism.icns")],
        check=True,
    )
    (BRANDING / "prism-macos-icon.png").write_bytes(
        (iconset / "icon_128x128@2x.png").read_bytes()
    )


def windows_icon(images):
    frames = [images[size].read_bytes() for size in WINDOWS_SIZES]
    directory = bytearray(struct.pack("<HHH", 0, 1, len(frames)))
    offset = 6 + 16 * len(frames)

    for size, frame in zip(WINDOWS_SIZES, frames):
        dimension = size if size < 256 else 0
        directory.extend(
            struct.pack("<BBBBHHII", dimension, dimension, 0, 0, 1, 32, len(frame), offset)
        )
        offset += len(frame)

    (BRANDING / "Prism.ico").write_bytes(directory + b"".join(frames))


def linux_icons(images):
    hicolor = BRANDING / "linux/hicolor"
    for size in LINUX_SIZES:
        destination = hicolor / f"{size}x{size}/apps/{APP_ID}.png"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(images[size].read_bytes())


def main():
    if sys.platform != "darwin":
        raise SystemExit("Icon export requires macOS sips and iconutil; use the pre-generated assets for Rust builds.")

    source = BRANDING / "prism-logo.png"
    sizes = sorted(set(WINDOWS_SIZES) | set(LINUX_SIZES))
    with tempfile.TemporaryDirectory(prefix="prism-icons-") as temporary:
        directory = Path(temporary)
        images = {size: directory / f"{size}.png" for size in sizes}
        for size, destination in images.items():
            resize(source, destination, size)

        macos_icon(BRANDING / "prism-macos.png", directory)
        windows_icon(images)
        linux_icons(images)
        (BRANDING / "prism-icon.png").write_bytes(images[256].read_bytes())

    print("Exported Prism UI, macOS ICNS, Windows ICO and Linux hicolor icons.")


if __name__ == "__main__":
    main()
