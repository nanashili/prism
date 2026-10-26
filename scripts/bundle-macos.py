#!/usr/bin/env python3
"""Bundle Prism with the native rea-rust provider."""
import argparse
import json
import plistlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import bundle_native
import licenses
import rea_bundle

ROOT = Path(__file__).resolve().parents[1]
BUNDLE_ID = "io.github.nanashili.prism"
BINARY_NAMES = ("prism", "prism-mcp")
BUILD_RECEIPT = ".prism-bundle-build.json"


def release_binaries(skip_build):
    target = target_directory()
    if skip_build:
        return verify_recorded_build(target)

    paths = build_release()
    binaries = identify_binaries(paths)
    identity = release_identity(target, binaries)
    (target / BUILD_RECEIPT).write_text(
        json.dumps(identity, indent=2, sort_keys=True) + "\n", encoding="utf-8",
    )
    return binaries


def build_release():
    feature = "rea-rust-ghidra"
    command = [
        "cargo", "build", "--release", "--locked", "--jobs", "2",
        "--no-default-features", "--features", f"desktop,{feature}",
        "--bins", "--message-format=json",
    ]
    result = subprocess.run(
        command, cwd=ROOT, stdout=subprocess.PIPE, text=True, check=False,
    )
    messages = [json.loads(line) for line in result.stdout.splitlines()]
    for message in messages:
        if message.get("reason") == "compiler-message":
            rendered = message["message"].get("rendered")
            if rendered:
                print(rendered, file=sys.stderr, end="")
    result.check_returncode()
    return build_artifacts(messages, feature)


def build_artifacts(messages, feature):
    binaries = {}
    for message in messages:
        if message.get("reason") != "compiler-artifact":
            continue
        name = message["target"]["name"]
        if name not in BINARY_NAMES or message.get("executable") is None:
            continue
        features = message["features"]
        if feature not in features:
            raise ValueError(f"Cargo built {name} with an unexpected provider feature set")
        path = Path(message["executable"]).resolve(strict=True)
        if name in binaries and binaries[name] != path:
            raise ValueError(f"Cargo reported multiple release executables for {name}")
        binaries[name] = path
    if set(binaries) != set(BINARY_NAMES):
        raise ValueError("Cargo did not report both Prism release executables")
    return [binaries[name] for name in BINARY_NAMES]


def target_directory():
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    )
    return Path(json.loads(metadata.stdout)["target_directory"]).resolve()


def identify_binaries(paths):
    binaries = {}
    for binary in paths:
        rea_bundle.require_macos_binary(binary)
        binaries[binary] = rea_bundle.file_digest(binary)
    return binaries


def release_identity(target, binaries):
    return {
        "schema_version": 1,
        "provider": "ghidra-rust",
        "binaries": {
            path.name: {"path": str(path.relative_to(target)), "sha256": digest}
            for path, digest in binaries.items()
        },
    }


def verify_recorded_build(target):
    receipt = target / BUILD_RECEIPT
    try:
        recorded = json.loads(receipt.read_text(encoding="utf-8"))
        paths = recorded_binary_paths(target, recorded)
        binaries = identify_binaries(paths)
        if recorded != release_identity(target, binaries):
            raise ValueError("The native provider or executable hashes do not match")
        return binaries
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise ValueError(
            f"Cannot reuse release build receipt {receipt}: {error}. "
            "Run this bundle command without --skip-build to build the native provider."
        ) from error


def recorded_binary_paths(target, recorded):
    if (
        not isinstance(recorded, dict)
        or set(recorded.get("binaries", {})) != set(BINARY_NAMES)
    ):
        raise ValueError("The receipt must identify both Prism release executables")

    paths = []
    for name in BINARY_NAMES:
        relative = Path(recorded["binaries"][name]["path"])
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError(
                f"The receipt path must stay inside Cargo's target directory: {relative}"
            )
        path = (target / relative).resolve(strict=True)
        if (
            path.name != name
            or path.parent.name != "release"
            or not path.is_relative_to(target)
        ):
            raise ValueError(
                f"The receipt does not name the expected release executable: {relative}"
            )
        paths.append(path)
    return paths


def install_binaries(directory, binaries, executable_name):
    directory.mkdir(parents=True)
    for binary, expected_digest in binaries.items():
        name = executable_name if binary.name == "prism" else binary.name
        destination = directory / name
        copied = rea_bundle.copy_file(binary, destination)
        if copied["sha256"] != expected_digest:
            raise RuntimeError(f"Release binary changed after build verification: {binary}")
        destination.chmod(0o755)

    if executable_name != "prism":
        (directory / "prism").symlink_to(executable_name)


def populate(bundle, binaries, executable_name, runtime):
    contents = bundle / "Contents"
    install_binaries(contents / "MacOS", binaries, executable_name)
    install_resources(contents / "Resources")
    write_info(contents, executable_name)
    if runtime is not None:
        return rea_bundle.install(contents / "Resources", runtime)
    return None


def install_resources(resources):
    resources.mkdir()
    shutil.copytree(ROOT / "skills", resources / "skills")
    shutil.copy2(ROOT / "resources/branding/Prism.icns", resources / "Prism.icns")
    notices = resources / "licenses"
    notices.mkdir()
    shutil.copy2(ROOT / "LICENSE", notices / "Prism-LICENSE.txt")
    shutil.copy2(ROOT / "NOTICE", notices / "NOTICE.txt")
    shutil.copy2(ROOT / "resources/upstream/LICENSE-APACHE", notices / "Upstream-LICENSE.txt")
    shutil.copy2(ROOT / "resources/upstream/fonts/JetBrainsMono-OFL.txt", notices / "JetBrainsMono-OFL.txt")
    shutil.copy2(ROOT / "THIRD_PARTY.md", notices / "THIRD_PARTY.md")
    licenses.collect(ROOT, notices / "cargo")


def write_info(contents, executable_name):
    info = {
        "CFBundleName": "Prism",
        "CFBundleDisplayName": "Prism",
        "CFBundleExecutable": executable_name,
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleVersion": "1",
        "CFBundleIconFile": "Prism.icns",
        "LSMinimumSystemVersion": "12.0",
        "NSHighResolutionCapable": True,
        "NSSupportsAutomaticGraphicsSwitching": True,
        "NSAutoFillRequiresTextContentTypeForOneTimeCodeOnMac": True,
        "CFBundleDocumentTypes": [{
            "CFBundleTypeName": "Prism project",
            "CFBundleTypeExtensions": ["prism"],
            "CFBundleTypeRole": "Editor",
            "LSHandlerRank": "Alternate",
        }],
    }
    with (contents / "Info.plist").open("wb") as file:
        plistlib.dump(info, file)


def publish(staged, destination):
    backup = destination.with_name(destination.name + ".previous")
    if backup.exists():
        raise FileExistsError(f"A previous bundle needs to be recovered or removed: {backup}")
    if destination.exists():
        with (destination / "Contents/Info.plist").open("rb") as file:
            identity = plistlib.load(file).get("CFBundleIdentifier")
        if identity != BUNDLE_ID:
            raise ValueError(f"Refusing to replace a different application: {destination}")
        destination.rename(backup)
    try:
        staged.rename(destination)
    except OSError as original:
        if backup.exists():
            try:
                backup.rename(destination)
            except OSError as recovery:
                raise RuntimeError(f"{original}; recovery failed: {recovery}. Bundle retained at {backup}") from original
        raise
    if backup.exists():
        shutil.rmtree(backup)


def argument_parser():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--skip-build", action="store_true",
        help="Reuse release executables verified by a matching bundle-build receipt",
    )
    parser.add_argument(
        "--rea-package", type=Path,
        help="Optional extracted rea-rust release to copy into the app bundle",
    )
    parser.add_argument("--output", type=Path, default=ROOT / "build/Prism.app")
    parser.add_argument(
        "--matlab-loader", action="store_true",
        help="Use a MATLAB-prefixed executable for dyld's private weak-symbol lookup mode; may be slower",
    )
    return parser


def main():
    parser = argument_parser()
    arguments = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("macOS is required to create this application bundle")
    try:
        runtime = (
            None if arguments.rea_package is None else rea_bundle.Inputs.resolve(arguments)
        )
        binaries = release_binaries(arguments.skip_build)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))

    destination = arguments.output.expanduser().resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    executable_name = "MATLAB-Prism" if arguments.matlab_loader else "prism"
    with tempfile.TemporaryDirectory(prefix="prism-bundle-", dir=destination.parent) as temporary:
        bundle = Path(temporary) / destination.name
        copied = populate(bundle, binaries, executable_name, runtime)
        bundle_native.sign_code(bundle, executable_name)
        if runtime is not None:
            rea_bundle.write_manifest(bundle / "Contents/Resources", runtime, copied)
        bundle_native.sign_application(bundle)
        publish(bundle, destination)
    size = sum(
        path.stat().st_size for path in destination.rglob("*")
        if path.is_file() and not path.is_symlink()
    )
    if runtime is not None:
        prerequisites = "includes the supplied native rea-rust release"
    else:
        prerequisites = "configure native rea-rust in Settings > Engines"
    print(f"{destination}\n{size / (1024 * 1024):.2f} MiB; {prerequisites}")


if __name__ == "__main__":
    main()
