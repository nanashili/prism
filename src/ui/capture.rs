use super::app::Prism;
use crate::error::{Result, failure};
use eframe::egui;
use image::ImageEncoder;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

pub(super) struct Capture {
    path: PathBuf,
    answer: SyncSender<Result<Value>>,
    lifetime: crate::worker::RequestLifetime,
}

impl Prism {
    pub(super) fn capture_window(
        &mut self,
        context: &egui::Context,
        args: &Value,
        answer: SyncSender<Result<Value>>,
        lifetime: crate::worker::RequestLifetime,
    ) {
        let result = self.begin_capture(context, args, answer.clone(), lifetime);
        if let Err(error) = result
            && answer.send(Err(error)).is_err()
        {
            self.status = "The screenshot request closed".into();
        }
    }

    fn begin_capture(
        &mut self,
        context: &egui::Context,
        args: &Value,
        answer: SyncSender<Result<Value>>,
        lifetime: crate::worker::RequestLifetime,
    ) -> Result<()> {
        if self.capture.is_some() {
            return Err(failure("A screenshot is already being saved"));
        }
        let path = PathBuf::from(args["path"].as_str().ok_or("Missing PNG path")?);
        if !path.is_absolute() {
            return Err(failure("The PNG path must be absolute"));
        }
        if let Some(project) = &self.workspace.project {
            crate::project::files::preserve_original(&project.target.path, &path)?;
        }
        self.capture = Some(Capture {
            path,
            answer,
            lifetime,
        });
        context.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        Ok(())
    }

    pub(super) fn finish_capture(&mut self, context: &egui::Context) {
        let image = context.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        let Some(image) = image else {
            return;
        };
        let Some(capture) = self.capture.take() else {
            return;
        };
        let result = capture
            .lifetime
            .check()
            .and_then(|()| save(&capture.path, &image))
            .map(|()| json!({"path":capture.path,"width":image.width(),"height":image.height()}));
        if capture.answer.send(result).is_err() {
            self.status = "The screenshot request closed".into();
        }
    }
}

fn save(path: &std::path::Path, image: &egui::ColorImage) -> Result<()> {
    let pixels: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(eframe::egui::Color32::to_srgba_unmultiplied)
        .collect();
    crate::storage::atomic::write(path, |file| {
        image::codecs::png::PngEncoder::new(file).write_image(
            &pixels,
            u32::try_from(image.width())?,
            u32::try_from(image.height())?,
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(())
    })
}
