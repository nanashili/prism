#!/usr/bin/env python3
"""Bundle Prism with Java Ghidra through REA, or opt in to experimental Rust REA."""
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


def release_binaries(skip_build, rust_backend):
    target = target_directory()
    if skip_build:
        return verify_recorded_build(target, rust_backend)

    paths = build_release(rust_backend)
    binaries = identify_binaries(paths)
    identity = release_identity(target, binaries, rust_backend)
    (target / BUILD_RECEIPT).write_text(
        json.dumps(identity, indent=2, sort_keys=True) + "\n", encoding="utf-8",
    )
    return binaries


def build_release(rust_backend):
    feature = "rea-rust-ghidra" if rust_backend else "prism-ghidra-rea"
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
    other = "prism-ghidra-rea" if feature == "rea-rust-ghidra" else "rea-rust-ghidra"
    for message in messages:
        if message.get("reason") != "compiler-artifact":
            continue
        name = message["target"]["name"]
        if name not in BINARY_NAMES or message.get("executable") is None:
            continue
        features = message["features"]
        if feature not in features or other in features:
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


def release_identity(target, binaries, rust_backend):
    return {
        "schema_version": 1,
        "provider": "ghidra-rust" if rust_backend else "ghidra",
        "binaries": {
            path.name: {"path": str(path.relative_to(target)), "sha256": digest}
            for path, digest in binaries.items()
        },
    }


def verify_recorded_build(target, rust_backend):
    receipt = target / BUILD_RECEIPT
    try:
        recorded = json.loads(receipt.read_text(encoding="utf-8"))
        paths = recorded_binary_paths(target, recorded)
        binaries = identify_binaries(paths)
        if recorded != release_identity(target, binaries, rust_backend):
            raise ValueError("The selected provider or executable hashes do not match")
        return binaries
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise ValueError(
            f"Cannot reuse release build receipt {receipt}: {error}. "
            "Run this bundle command without --skip-build to build the selected provider."
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
    providers = parser.add_mutually_exclusive_group()
    providers.add_argument(
        "--rea-rust-ghidra", dest="rust_backend", action="store_true",
        help="Opt in to the experimental Rust REA provider and its bundled runtime",
    )
    providers.add_argument(
        "--java-ghidra", dest="rust_backend", action="store_false",
        help="Select the default Java Ghidra support with external REA, Node, Java and Ghidra",
    )
    parser.set_defaults(rust_backend=False)
    parser.add_argument(
        "--rea-package", type=Path,
        help="Built REA package with installed runtime dependencies",
    )
    parser.add_argument(
        "--node-runtime", type=Path,
        help="Portable Node distribution containing bin/node and LICENSE",
    )
    parser.add_argument("--rust-ghidra-worker", type=Path, help="REA's compiled native worker")
    parser.add_argument(
        "--worker-notices", type=Path,
        help="Prepared license notices for the worker and its native dependencies",
    )
    parser.add_argument("--output", type=Path, default=ROOT / "build/Prism.app")
    parser.add_argument(
        "--matlab-loader", action="store_true",
        help="Use a MATLAB-prefixed executable for dyld's private weak-symbol lookup mode; may be slower",
    )
    return parser


def runtime_inputs(parser, arguments):
    supplied = (
        arguments.rea_package, arguments.node_runtime,
        arguments.rust_ghidra_worker, arguments.worker_notices,
    )
    if not arguments.rust_backend:
        if any(path is not None for path in supplied):
            parser.error(
                "Rust runtime inputs require --rea-rust-ghidra and cannot be "
                "combined with --java-ghidra"
            )
        return None
    if any(path is None for path in supplied):
        parser.error(
            "The Rust REA bundle requires --rea-package, --node-runtime, "
            "--rust-ghidra-worker and --worker-notices. "
            "Use --java-ghidra for a bundle that relies on externally configured "
            "REA, Node, Java and Ghidra."
        )
    return rea_bundle.Inputs.resolve(arguments)


def main():
    parser = argument_parser()
    arguments = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("macOS is required to create this application bundle")
    try:
        runtime = runtime_inputs(parser, arguments)
        binaries = release_binaries(arguments.skip_build, arguments.rust_backend)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))

    destination = arguments.output.expanduser().resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    executable_name = "MATLAB-Prism" if arguments.matlab_loader else "prism"
    with tempfile.TemporaryDirectory(prefix="prism-bundle-", dir=destination.parent) as temporary:
        bundle = Path(temporary) / destination.name
        copied = populate(bundle, binaries, executable_name, runtime)
        bundle_native.sign_code(bundle, executable_name, arguments.rust_backend)
        if runtime is not None:
            rea_bundle.write_manifest(bundle / "Contents/Resources", runtime, copied)
        bundle_native.sign_application(bundle)
        publish(bundle, destination)
    size = sum(
        path.stat().st_size for path in destination.rglob("*")
        if path.is_file() and not path.is_symlink()
    )
    prerequisites = (
        "includes Rust REA runtime; macOS Swift tools remain required"
        if runtime is not None else
        "configure external REA, Node, Java and Ghidra in Settings > Engines"
    )
    print(f"{destination}\n{size / (1024 * 1024):.2f} MiB; {prerequisites}")


if __name__ == "__main__":
    main()
