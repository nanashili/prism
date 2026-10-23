use super::{icons::Icon, theme::Palette};
use eframe::egui::{self, Color32, Rect, TextureHandle};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CAPACITY: usize = 64;

#[derive(Clone, Default)]
struct Cache(VecDeque<(PathBuf, Option<TextureHandle>)>);

pub(super) fn paint(ui: &egui::Ui, path: &Path, rect: Rect) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    if let Some((uri, svg, mono)) = format_logo(path) {
        paint_format(ui, rect, uri, svg, mono);
        return;
    }
    let source = path
        .ancestors()
        .find(|path| path.extension().is_some_and(|ext| ext == "app"))
        .unwrap_or(path);
    let texture = file_texture(ui.ctx(), source);
    if let Some(texture) = texture {
        egui::Image::new(&texture).paint_at(ui, rect);
    } else {
        let p = Palette::for_context(ui.ctx());
        ui.painter().rect_filled(rect, rect.width() / 4.5, p.fill2);
        Icon::Binary.paint(ui, rect.shrink(rect.width() * 0.23), p.muted);
    }
}

fn file_texture(context: &egui::Context, source: &Path) -> Option<TextureHandle> {
    let cache_id = egui::Id::new("file-icons");
    let cached = context.data_mut(|data| {
        let cache = data.get_temp_mut_or_default::<Cache>(cache_id);
        let index = cache.0.iter().position(|(path, _)| path == source)?;
        let entry = cache.0.remove(index).expect("a located icon exists");
        let texture = entry.1.clone();
        cache.0.push_back(entry);
        Some(texture)
    });
    if let Some(texture) = cached {
        return texture;
    }

    let texture = super::native_file_icon::load(source).map(|image| {
        context.load_texture(
            format!("file-icon:{}", source.display()),
            image,
            egui::TextureOptions::LINEAR,
        )
    });
    context.data_mut(|data| {
        let cache = data.get_temp_mut_or_default::<Cache>(cache_id);
        if cache.0.len() == CAPACITY {
            cache.0.pop_front();
        }
        cache.0.push_back((source.to_path_buf(), texture.clone()));
    });
    texture
}

fn paint_format(ui: &egui::Ui, rect: Rect, uri: &'static str, svg: &'static [u8], mono: bool) {
    let p = Palette::for_context(ui.ctx());
    let tile = Rect::from_center_size(rect.center(), rect.size() * 0.805);
    let logo = if rect.width() < 24.0 {
        tile
    } else {
        ui.painter().rect(
            tile,
            tile.width() * 0.225,
            p.raised,
            egui::Stroke::new(1.0, p.line3),
            egui::StrokeKind::Inside,
        );
        Rect::from_center_size(tile.center(), tile.size() * 0.74)
    };
    let logo = if mono {
        Rect::from_center_size(
            logo.center(),
            egui::vec2(logo.width(), logo.width() / 4.193),
        )
    } else {
        logo
    };
    let image = if mono {
        let bytes = ui.ctx().data_mut(|data| {
            data.get_temp_mut_or_insert_with(egui::Id::new(uri), || {
                Arc::<[u8]>::from(
                    String::from_utf8_lossy(svg)
                        .replace("<path ", "<path fill=\"white\" ")
                        .into_bytes(),
                )
            })
            .clone()
        });
        egui::Image::from_bytes(uri, bytes)
    } else {
        egui::Image::from_bytes(uri, svg)
    };
    image
        .tint(if mono { p.text } else { Color32::WHITE })
        .paint_at(ui, logo);
}

fn format_logo(path: &Path) -> Option<(&'static str, &'static [u8], bool)> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    macro_rules! logo {
        ($name:literal, $mono:literal) => {
            Some((
                concat!("bytes://formats/", $name, ".svg"),
                include_bytes!(concat!("../../resources/upstream/formats/", $name, ".svg"))
                    .as_slice(),
                $mono,
            ))
        };
    }
    match extension.as_str() {
        "apk" | "aab" | "xapk" | "apks" | "dex" => logo!("android", false),
        "ptx" | "cubin" | "fatbin" => logo!("nvidia", false),
        "hipfb" | "hsaco" | "co" => logo!("amd", true),
        "spv" => logo!("vulkan", false),
        "gguf" => logo!("llama-cpp", false),
        "onnx" => logo!("onnx", false),
        "pt" | "pth" | "ckpt" => logo!("pytorch", false),
        "safetensors" => logo!("huggingface", false),
        "tflite" => logo!("tensorflow", false),
        _ => None,
    }
}
