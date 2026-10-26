"""Validate and stage an explicit native rea-rust release for macOS."""

import hashlib
import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

import bundle_native


MACOS_ARCHITECTURES = {"arm64", "x86_64"}


@dataclass(frozen=True)
class Inputs:
    """Validated native REA release inputs."""

    package: Path
    launcher: Path
    worker: Path
    mcp_server: Path | None
    package_version: str

    @classmethod
    def resolve(cls, arguments):
        supplied = arguments.rea_package.expanduser()
        if supplied.is_symlink():
            raise ValueError(f"--rea-package must be a real release directory: {supplied}")
        package = supplied.resolve(strict=True)
        if not package.is_dir():
            raise ValueError(f"--rea-package must be a real release directory: {package}")

        launcher = executable(package, "rea")
        worker = executable(package, "rea-ghidra-rust-worker")
        mcp_server = optional_executable(package, "rea-mcp-server")
        validate_release_metadata(package)
        for path in (launcher, worker, mcp_server):
            if path is not None:
                require_macos_binary(path)

        return cls(
            package=package,
            launcher=launcher,
            worker=worker,
            mcp_server=mcp_server,
            package_version=release_version(package),
        )


def executable(package, name):
    path = optional_executable(package, name)
    if path is None:
        raise FileNotFoundError(
            f"The rea-rust release is missing an executable at bin/{name}: {package}"
        )
    return path


def optional_executable(package, name):
    candidate = package / "bin" / name
    if candidate.is_file():
        require_executable(candidate)
        return candidate
    return None


def validate_release_metadata(package):
    require_file(package / "LICENSE")
    manifest = package / "release.json"
    if not manifest.is_file():
        return

    try:
        metadata = json.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"Cannot read rea-rust release manifest {manifest}: {error}") from error
    if metadata.get("name") != "rea-rust":
        raise ValueError(f"The release manifest does not describe rea-rust: {manifest}")
    if not isinstance(metadata.get("version"), str) or not metadata["version"]:
        raise ValueError(f"The release manifest has no version: {manifest}")


def release_version(package):
    version_file = package / "VERSION"
    if version_file.is_file():
        version = version_file.read_text(encoding="utf-8").strip()
        if version:
            return version
    manifest = package / "release.json"
    if manifest.is_file():
        metadata = json.loads(manifest.read_text(encoding="utf-8"))
        if isinstance(metadata.get("version"), str):
            return metadata["version"]
    return "unknown"


def require_file(path):
    if not path.is_file():
        raise FileNotFoundError(f"Required runtime file is missing: {path}")


def require_executable(path):
    require_file(path)
    if path.is_symlink():
        raise ValueError(f"Runtime input must be a regular executable: {path}")
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
    """Copy a release binary and verify that its content did not change."""
    expected = file_digest(source)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    if file_digest(destination) != expected:
        raise RuntimeError(f"Runtime input changed while copying: {source}")
    return {"sha256": expected}


def install(resources, inputs):
    destination = resources / "rea-rust"
    copied = {}
    for relative, identity in copy_tree(inputs.package, destination).items():
        copied[f"rea-rust/{relative}"] = identity
    return copied


def write_manifest(resources, inputs, source_files):
    signed = {
        f"rea-rust/{path}": identity
        for path, identity in entries(resources / "rea-rust").items()
    }
    manifest = {
        "schema_version": 2,
        "provider": "ghidra-rust",
        "runtime": "rea-rust",
        "rea_version": inputs.package_version,
        "source_files_before_signing": source_files,
        "bundled_files_after_signing": signed,
        "native_mcp": True,
        "runtime_prerequisites": [],
    }
    (resources / "rea-runtime-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8",
    )
