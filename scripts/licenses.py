"""Collect the license notices for the active target's normal Cargo dependency tree."""
import json
from pathlib import Path
import re
import shutil
import subprocess


def cargo_json(root):
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=root, check=True, capture_output=True, text=True,
    )
    return json.loads(result.stdout)


def active_packages(root):
    result = subprocess.run(
        ["cargo", "tree", "--locked", "--edges", "normal", "--prefix", "none", "--format", "{p}"],
        cwd=root, check=True, capture_output=True, text=True,
    )
    names = set()
    for line in result.stdout.splitlines():
        match = re.match(r"(\S+) v(\S+)", line)
        if match:
            names.add((match[1], match[2]))
    root_manifest = (root / "Cargo.toml").resolve()
    return [
        package for package in cargo_json(root)["packages"]
        if (package["name"], package["version"]) in names
        and Path(package["manifest_path"]).resolve() != root_manifest
    ]


def notice_files(package):
    root = Path(package["manifest_path"]).parent
    paths = set()
    declared = package.get("license_file")
    if declared:
        paths.add((root / declared).resolve())
    for path in root.rglob("*"):
        parts = path.relative_to(root).parts
        if len(parts) > 4 or not path.is_file():
            continue
        name = path.name.upper()
        notice_name = name.startswith((
            "LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE",
            "THIRD_PARTY", "UNLICENSE", "OFL",
        ))
        license_directory = any(part.upper() in ("LICENSES", "LICENCES") for part in parts[:-1])
        if notice_name or license_directory:
            paths.add(path)
    return root, sorted(paths)


def package_notices(root, package):
    source, paths = notice_files(package)
    if paths:
        return source, paths
    name = f"{package['name']}-{package['version']}"
    source = root / "notices" / name
    paths = sorted(path for path in source.glob("*") if path.is_file())
    if not paths or not any(path.name.startswith("LICENSE") for path in paths):
        raise FileNotFoundError(f"No license notice was found for {name}")
    return source, paths


def collect(root, destination):
    packages = active_packages(root)
    shutil.copytree(root / "notices/licenses", destination / "license-texts")
    index = ["# Cargo dependency notices", "", "These packages are in the active platform's normal dependency tree. Build-only tools are omitted.", ""]
    for package in sorted(packages, key=lambda package: (package["name"], package["version"])):
        name = f"{package['name']}-{package['version']}"
        source, paths = package_notices(root, package)
        index.append(f"- {name}: {package.get('license') or 'See package notices'}")
        metadata = {
            field: package.get(field)
            for field in ("name", "version", "license", "authors", "repository")
        }
        package_destination = destination / name
        package_destination.mkdir(parents=True, exist_ok=True)
        (package_destination / "PACKAGE.json").write_text(
            json.dumps(metadata, indent=2) + "\n", encoding="utf-8",
        )
        for path in paths:
            relative = path.relative_to(source) if path.is_relative_to(source) else Path(path.name)
            target = destination / name / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
    (destination / "INDEX.md").write_text("\n".join(index) + "\n", encoding="utf-8")
