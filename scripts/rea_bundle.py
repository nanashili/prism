"""Stage explicit REA, Node and Rust-worker inputs for the macOS application."""
import hashlib
import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

import bundle_native


RUNTIME_TREES = ("dist", "node_modules", "bridge/process", "licenses")
MACOS_ARCHITECTURES = {"arm64", "x86_64"}
RUNTIME_FILES = (
    "scripts/rea.mjs", "package.json", "package-lock.json", ".nvmrc",
    "LICENSE", "THIRD_PARTY_NOTICES.md",
)
PROVIDER_FILES = (
    "dist/generatedPackageMetadata.js", "dist/mcp/main.js",
    "dist/ghidra-rust/GhidraRustProvider.js",
    "dist/ghidra-rust/GhidraRustProtocol.js",
    "dist/contracts/batchDecompileOutput.js", "dist/contracts/enhancedInputs.js",
    "dist/domain/analysisErrorProjection.js", "dist/application/EnhancedTools.js",
    "dist/application/FunctionAnalysisEvidence.js", "dist/application/CapabilityInventory.js",
    "dist/process/OwnedProcessResidentMemory.js",
    "dist/application/binary/BinarySession.js",
    "dist/application/binary/BinarySessionExecution.js",
    "dist/composition/binary.js", "dist/server/registerEnhancedTools.js",
    "bridge/process/ProcessRunTokenReader.swift",
    "bridge/process/ProcessRunTokenReaderMain.swift",
)


@dataclass(frozen=True)
class Inputs:
    package: Path
    node: Path
    worker: Path
    worker_notices: Path
    package_version: str
    node_version: str

    @classmethod
    def resolve(cls, arguments):
        package = arguments.rea_package.expanduser().resolve(strict=True)
        node = arguments.node_runtime.expanduser().resolve(strict=True)
        worker = arguments.rust_ghidra_worker.expanduser().resolve(strict=True)
        notices = arguments.worker_notices.expanduser().resolve(strict=True)
        metadata = validate_package(package)
        require_macos_binary(worker)
        require_macos_binary(node / "bin/node")
        require_file(node / "LICENSE")
        validate_worker_notices(notices)
        node_version = pinned_node_version(package, node / "bin/node")
        return cls(package, node, worker, notices, metadata["version"], node_version)


def validate_worker_notices(notices):
    require_tree(notices)
    license_files = (
        path for path in notices.rglob("*")
        if path.is_file() and path.name.upper().startswith(("LICENSE", "COPYING"))
    )
    if next(license_files, None) is None:
        raise ValueError(
            "--worker-notices must contain the worker's dependency license notices"
        )


def validate_package(package):
    for relative in RUNTIME_FILES + PROVIDER_FILES:
        require_file(package / relative)
    for relative in RUNTIME_TREES:
        require_tree(package / relative)
    metadata = json.loads((package / "package.json").read_text(encoding="utf-8"))
    if metadata.get("name") != "rea-agents" or not isinstance(metadata.get("version"), str):
        raise ValueError("--rea-package must be a built rea-agents package")
    lock = json.loads((package / "package-lock.json").read_text(encoding="utf-8"))
    if (lock.get("name"), lock.get("version")) != (metadata["name"], metadata["version"]):
        raise ValueError("REA package.json and package-lock.json must describe the same release")
    for name in metadata.get("dependencies", {}):
        manifest = package / "node_modules" / name / "package.json"
        require_file(manifest)
        installed = json.loads(manifest.read_text(encoding="utf-8"))
        expected = (
            lock.get("packages", {}).get(f"node_modules/{name}", {}).get("version")
        )
        if expected is None or installed.get("version") != expected:
            raise ValueError(f"Installed REA dependency does not match package-lock.json: {name}")
    return metadata


def require_file(path):
    if not path.is_file():
        raise FileNotFoundError(f"Required runtime file is missing: {path}")


def require_executable(path):
    require_file(path)
    if not os.access(path, os.X_OK):
        raise ValueError(f"Runtime input is not executable: {path}")


def require_macos_binary(path):
    require_executable(path)
    try:
        available = bundle_native.architectures(path)
    except subprocess.CalledProcessError as error:
        raise ValueError(f"A bundled executable must be a Mach-O binary: {path}") from error
    if not available or not available.issubset(MACOS_ARCHITECTURES):
        raise ValueError(
            "Bundled executables must support macOS arm64 or x86_64: "
            f"{path} has {sorted(available)}"
        )


def require_tree(path):
    if not path.is_dir() or path.is_symlink():
        raise ValueError(f"Supply a real runtime directory, not an external directory link: {path}")


def pinned_node_version(package, executable):
    expected = (package / ".nvmrc").read_text(encoding="utf-8").strip().removeprefix("v")
    if re.fullmatch(r"\d+\.\d+\.\d+", expected) is None:
        raise ValueError("REA's .nvmrc must pin an exact Node version for bundling")
    result = subprocess.run(
        [str(executable), "--version"], check=True, capture_output=True,
        text=True, timeout=10,
    )
    actual = result.stdout.strip().removeprefix("v")
    if actual != expected:
        raise ValueError(f"REA pins Node {expected}; --node-runtime supplies {actual!r}")
    return actual


def file_digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def entries(root):
    result = {}
    for path in sorted(root.rglob("*")):
        relative = str(path.relative_to(root))
        if path.is_symlink():
            target = path.readlink()
            if target.is_absolute() or not path.resolve(strict=True).is_relative_to(root):
                raise ValueError(f"Runtime link must stay inside its copied directory: {path}")
            result[relative] = {"symlink": str(target)}
        elif path.is_file():
            result[relative] = {"sha256": file_digest(path)}
        elif not path.is_dir():
            raise ValueError(f"Unsupported runtime filesystem entry: {path}")
    return result


def copy_tree(source, destination):
    expected = entries(source)
    shutil.copytree(source, destination, symlinks=True)
    if entries(destination) != expected:
        raise RuntimeError(f"Runtime input changed while copying: {source}")
    return expected


def copy_file(source, destination):
    expected = {"sha256": file_digest(source)}
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    if file_digest(destination) != expected["sha256"]:
        raise RuntimeError(f"Runtime input changed while copying: {source}")
    return expected


def install(resources, inputs):
    copied = {}
    package = resources / "rea"
    for relative in RUNTIME_TREES:
        target = package / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        for path, identity in copy_tree(inputs.package / relative, target).items():
            copied[f"rea/{relative}/{path}"] = identity
    for relative in RUNTIME_FILES:
        copied[f"rea/{relative}"] = copy_file(inputs.package / relative, package / relative)
    copied["rea/bin/rea-ghidra-rust-worker"] = copy_file(
        inputs.worker, package / "bin/rea-ghidra-rust-worker",
    )
    copied["node/bin/node"] = copy_file(
        inputs.node / "bin/node", resources / "node/bin/node",
    )
    copied["node/LICENSE"] = copy_file(
        inputs.node / "LICENSE", resources / "node/LICENSE",
    )
    notices = resources / "licenses/rust-ghidra-worker"
    for path, identity in copy_tree(inputs.worker_notices, notices).items():
        copied[f"licenses/rust-ghidra-worker/{path}"] = identity
    return copied


def write_manifest(resources, inputs, source_files):
    signed = {}
    for relative in ("rea", "node", "licenses/rust-ghidra-worker"):
        for path, identity in entries(resources / relative).items():
            signed[f"{relative}/{path}"] = identity
    manifest = {
        "schema_version": 1,
        "provider": "ghidra-rust",
        "rea_version": inputs.package_version,
        "node_version": inputs.node_version,
        "source_files_before_signing": source_files,
        "bundled_files_after_signing": signed,
        "runtime_prerequisites": ["Apple Swift compiler and macOS SDK via xcrun"],
        "worker_protocol_validation": "REA validates the worker during session preparation",
    }
    (resources / "rea-runtime-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8",
    )
