//! Embeds the desktop resources required by the Windows application bundle.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/windows/prism.rc");
    println!("cargo:rerun-if-changed=resources/branding/Prism.ico");

    #[cfg(feature = "desktop")]
    if std::env::var("CARGO_CFG_TARGET_OS")? == "windows" {
        embed_resource::compile_for(
            "resources/windows/prism.rc",
            ["prism"],
            embed_resource::ParamsIncludeDirs([env!("CARGO_MANIFEST_DIR")]),
        )
        .manifest_required()?;
    }

    Ok(())
}
