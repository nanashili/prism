"""Validate relocatable Mach-O inputs and sign staged application code."""
import subprocess


MACHO_MAGIC = {
    bytes.fromhex(value) for value in (
        "feedface", "cefaedfe", "feedfacf", "cffaedfe",
        "cafebabe", "bebafeca", "cafebabf", "bfbafeca",
    )
}
DYLIB_COMMANDS = {
    "LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LOAD_UPWARD_DYLIB",
}


def architectures(path):
    result = subprocess.run(
        ["/usr/bin/lipo", "-archs", str(path)], check=True, capture_output=True, text=True,
    )
    return set(result.stdout.split())


def require_architectures(path, required):
    available = architectures(path)
    if not required.issubset(available):
        raise ValueError(f"{path} has {sorted(available)}; the app requires {sorted(required)}")


def native_files(directory):
    result = []
    for path in sorted(directory.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        with path.open("rb") as source:
            if source.read(4) in MACHO_MAGIC:
                result.append(path)
    return result


def load_paths(path):
    result = subprocess.run(
        ["/usr/bin/otool", "-l", str(path)], check=True, capture_output=True, text=True,
    )
    dependencies = []
    runpaths = []
    command = None
    for line in result.stdout.splitlines():
        text = line.strip()
        if text.startswith("cmd "):
            command = text.removeprefix("cmd ")
        elif command in DYLIB_COMMANDS and text.startswith("name "):
            dependencies.append(text.removeprefix("name ").rsplit(" (offset ", 1)[0])
        elif command == "LC_RPATH" and text.startswith("path "):
            runpaths.append(text.removeprefix("path ").rsplit(" (offset ", 1)[0])
    return dependencies, runpaths


def relative_library(value, loader, executable):
    for prefix, directory in (("@loader_path/", loader), ("@executable_path/", executable)):
        if value.startswith(prefix):
            return directory / value.removeprefix(prefix)
    return None


def validate_libraries(path, bundle):
    dependencies, runpaths = load_paths(path)
    executable = executable_directory(path, bundle)
    for dependency in dependencies:
        if dependency.startswith(("/usr/lib/", "/System/Library/")):
            continue
        candidate = relative_library(dependency, path.parent, executable)
        candidates = [] if candidate is None else [candidate]
        if dependency.startswith("@rpath/"):
            for runpath in runpaths:
                directory = relative_library(runpath + "/", path.parent, executable)
                if directory is not None:
                    candidates.append(directory / dependency.removeprefix("@rpath/"))
        if any(
            candidate.is_file() and candidate.resolve().is_relative_to(bundle)
            for candidate in candidates
        ):
            continue
        raise ValueError(
            f"Native input depends on a library outside this bundle: {path}: {dependency}"
        )


def executable_directory(path, bundle):
    resources = bundle / "Contents/Resources"
    worker = resources / "rea/bin/rea-ghidra-rust-worker"
    if path == worker:
        return worker.parent
    if path.is_relative_to(resources):
        return resources / "node/bin"
    return bundle / "Contents/MacOS"


def sign_code(bundle, executable_name, rust_backend):
    main = bundle / "Contents/MacOS" / executable_name
    required = architectures(main)
    resources = bundle / "Contents/Resources"
    if rust_backend:
        for relative in ("node/bin/node", "rea/bin/rea-ghidra-rust-worker"):
            require_architectures(resources / relative, required)
    require_architectures(bundle / "Contents/MacOS/prism-mcp", required)
    for path in native_files(bundle):
        validate_libraries(path, bundle)
        subprocess.run(["codesign", "--force", "--sign", "-", str(path)], check=True)


def sign_application(bundle):
    subprocess.run(["codesign", "--force", "--sign", "-", str(bundle)], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(bundle)], check=True)
