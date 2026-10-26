//! Native desktop and managed headless entry point for Prism.

mod agent;
mod analysis;
mod analysis_repair;
mod bridge;
mod comparison;
mod debug;
mod decompilation;
mod error;
mod events;
mod executable_paths;
mod headless;
mod investigation;
mod model;
mod profiling;
mod project;
mod protocol;
mod reconstruction;
mod settings;
mod storage;
mod ui;
mod worker;

fn main() {
    if let Err(error) = run() {
        eprintln!("Prism: {error}");
        std::process::exit(1);
    }
}

fn run() -> error::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let first = arguments.next();
    let command = first
        .as_ref()
        .and_then(|argument| argument.to_str())
        .map(str::to_owned);

    let is_mcp = matches!(
        command.as_deref(),
        Some("mcp" | "--prism-mcp" | "--rea-mcp")
    );
    let _profiling = if is_mcp || command.as_deref() == Some("--headless") {
        profiling::start_mcp()
    } else {
        profiling::start()
    };

    if command.as_deref() == Some("--headless") {
        let path = arguments
            .next()
            .ok_or("Usage: prism --headless <project-path>")?;
        if arguments.next().is_some() {
            return Err(error::failure("Usage: prism --headless <project-path>"));
        }
        return headless::run(std::path::PathBuf::from(path));
    }

    if is_mcp {
        let mode = match command.as_deref() {
            Some("--prism-mcp") => Some(std::ffi::OsString::from("--window")),
            Some("--rea-mcp") => Some(std::ffi::OsString::from("--analysis")),
            _ => None,
        };
        return Ok(prism::mcp::run_cli(mode.into_iter().chain(arguments))?);
    }

    let initial = first.map(std::path::PathBuf::from);
    let _profile_lock = storage::lock_profile(&settings::support_dir())?;
    eframe::run_native(
        "Prism",
        native_options()?,
        Box::new(move |context| Ok(Box::new(ui::Prism::new(context, initial)?))),
    )
    .map_err(|error| error::failure(error.to_string()))?;
    Ok(())
}

fn native_options() -> error::Result<eframe::NativeOptions> {
    #[cfg(target_os = "macos")]
    let icon_bytes = include_bytes!("../resources/branding/prism-macos-icon.png").as_slice();
    #[cfg(not(target_os = "macos"))]
    let icon_bytes = include_bytes!("../resources/branding/prism-icon.png").as_slice();

    let icon = eframe::icon_data::from_png_bytes(icon_bytes)?;
    Ok(eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Prism")
            .with_icon(icon)
            .with_app_id("io.github.nanashili.prism")
            .with_inner_size([1440.0, 940.0])
            .with_min_inner_size([900.0, 600.0])
            .with_fullsize_content_view(true)
            .with_transparent(cfg!(target_os = "macos"))
            .with_title_shown(false)
            .with_titlebar_shown(false),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    })
}
