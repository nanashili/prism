use crate::settings::research::Category;

pub(super) fn categories(name: &str, metadata: &str) -> Vec<Category> {
    let text = format!("{name} {metadata}").to_lowercase();
    let words: Vec<_> = text.split(|c: char| !c.is_alphanumeric()).collect();
    let has = |terms: &[&str]| terms.iter().any(|term| words.contains(term));
    let mut categories = Vec::new();

    for (category, terms) in [
        (
            Category::Binaries,
            &[
                "binary",
                "binaries",
                "disassembly",
                "ghidra",
                "ida",
                "windows",
                "linux",
                "macos",
                "obfuscation",
                "unpacking",
            ][..],
        ),
        (
            Category::Firmware,
            &[
                "firmware",
                "uefi",
                "efi",
                "dma",
                "fpga",
                "embedded",
                "kernel",
                "driver",
                "drivers",
                "hypervisor",
            ],
        ),
        (
            Category::Web,
            &["web", "browser", "javascript", "wasm", "webassembly"],
        ),
        (
            Category::Mobile,
            &[
                "android", "ios", "apk", "jadx", "zygisk", "kernelsu", "mobile",
            ],
        ),
        (
            Category::Network,
            &[
                "network",
                "networks",
                "networking",
                "protocol",
                "protocols",
                "packet",
                "packets",
                "traffic",
            ],
        ),
        (
            Category::GameEngines,
            &[
                "unreal", "unity", "il2cpp", "ue4", "ue5", "uobject", "netvars", "assets",
            ],
        ),
        (
            Category::GameRuntime,
            &[
                "hook",
                "hooks",
                "hooking",
                "injection",
                "injector",
                "offset",
                "offsets",
                "memory",
                "runtime",
            ],
        ),
        (
            Category::GameSecurity,
            &[
                "anticheat",
                "vanguard",
                "battleye",
                "eac",
                "vac",
                "faceit",
                "vgk",
            ],
        ),
        (
            Category::Graphics,
            &[
                "shader",
                "shaders",
                "gpu",
                "rendering",
                "directx",
                "vulkan",
                "metal",
                "graphics",
                "pytorch",
                "safetensors",
                "gguf",
                "coreml",
            ],
        ),
    ] {
        if has(terms) {
            categories.push(category);
        }
    }
    if text.contains("game-engine") && !categories.contains(&Category::GameEngines) {
        categories.push(Category::GameEngines);
    }
    if text.contains("anti-cheat") && !categories.contains(&Category::GameSecurity) {
        categories.push(Category::GameSecurity);
    }
    categories
}

pub(super) fn built_in(id: &str) -> Vec<Category> {
    match id {
        "shader-check" => vec![Category::Graphics],
        "network-map" => vec![Category::Network, Category::Web, Category::Mobile],
        _ => vec![
            Category::Binaries,
            Category::Firmware,
            Category::Mobile,
            Category::GameEngines,
            Category::GameRuntime,
            Category::GameSecurity,
        ],
    }
}
