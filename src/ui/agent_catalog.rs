use crate::agent::catalog::Entry;
use eframe::egui;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};

enum Update {
    Entries(Vec<Entry>, HashSet<String>),
    Models(crate::agent::models::Known),
    Icon(String, egui::ColorImage),
    Error(String),
}

#[derive(Default)]
pub(super) struct Catalog {
    pub entries: Vec<Entry>,
    pub installed: HashSet<String>,
    pub models: crate::agent::models::Known,
    pub models_revision: u64,
    pub query: String,
    pub status: String,
    receiver: Option<Receiver<Update>>,
    cancelled: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    started: bool,
}

impl Catalog {
    pub fn start(&mut self, context: &egui::Context) {
        if self.started {
            return;
        }
        self.entries = crate::agent::catalog::fallback();
        let (sender, receiver) = mpsc::sync_channel(4);
        let context = context.clone();
        let cancelled = self.cancelled.clone();
        let launched = std::thread::Builder::new()
            .name("agent-catalog".into())
            .spawn(move || load_catalog(sender, context, &cancelled));
        match launched {
            Ok(thread) => {
                self.receiver = Some(receiver);
                self.thread = Some(thread);
                self.started = true;
            }
            Err(error) => self.status = format!("Could not load the agent list: {error}"),
        }
    }

    pub fn rescan(&mut self) {
        self.installed = self
            .entries
            .iter()
            .filter(|entry| crate::agent::launch::installed(entry))
            .map(|entry| entry.id.clone())
            .collect();
    }

    pub fn poll(&mut self, context: &egui::Context) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        while let Ok(update) = receiver.try_recv() {
            match update {
                Update::Entries(entries, installed) => {
                    self.entries = entries;
                    self.installed = installed;
                }
                Update::Models(models) => {
                    for (key, choices) in models {
                        self.models.entry(key).or_insert(choices);
                    }
                    self.models_revision += 1;
                }
                Update::Icon(id, image) => {
                    let texture = context.load_texture(
                        format!("agent-logo/{id}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    context.data_mut(|data| {
                        data.insert_temp(egui::Id::new(("agent-logo", id)), texture)
                    });
                }
                Update::Error(error) => {
                    self.status = format!("Using the offline agent list: {error}")
                }
            }
        }
    }
}

fn load_catalog(sender: SyncSender<Update>, context: egui::Context, cancelled: &AtomicBool) {
    match crate::agent::models::load() {
        Ok(models) => {
            if sender.send(Update::Models(models)).is_err() {
                return;
            }
            context.request_repaint();
        }
        Err(error) => eprintln!("Prism: couldn't load saved models: {error}"),
    }
    if !publish_entries(&sender, crate::agent::catalog::fallback()) {
        return;
    }
    context.request_repaint();

    let entries = match crate::agent::catalog::load(cancelled) {
        Ok(entries) => entries,
        Err(error) => {
            if cancelled.load(Ordering::Relaxed)
                || sender.send(Update::Error(error.to_string())).is_err()
            {
                return;
            }
            crate::agent::catalog::fallback()
        }
    };
    if !publish_entries(&sender, entries.clone()) {
        return;
    }
    context.request_repaint();

    for entry in entries.iter().take(128) {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        if super::agent_logos::shipped(&entry.id, false).is_some() {
            continue;
        }
        if let Some(image) = registry_icon(entry, cancelled) {
            if sender.send(Update::Icon(entry.id.clone(), image)).is_err() {
                return;
            }
            context.request_repaint();
        }
    }
}

fn publish_entries(sender: &SyncSender<Update>, entries: Vec<Entry>) -> bool {
    let installed = entries
        .iter()
        .filter(|entry| crate::agent::launch::installed(entry))
        .map(|entry| entry.id.clone())
        .collect();
    sender.send(Update::Entries(entries, installed)).is_ok()
}

impl Drop for Catalog {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.receiver.take();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("The agent catalog worker stopped unexpectedly");
        }
    }
}

fn registry_icon(entry: &Entry, cancelled: &AtomicBool) -> Option<egui::ColorImage> {
    let file = crate::settings::support_dir()
        .join("agents/icons")
        .join(format!("{}.svg", entry.id));
    if !file.is_file() {
        if !entry
            .icon
            .starts_with("https://cdn.agentclientprotocol.com/")
        {
            return None;
        }
        if let Err(error) = crate::agent::catalog::download_icon(&entry.icon, &file, cancelled) {
            eprintln!("Prism: could not load {}'s icon: {error}", entry.name);
            return None;
        }
    }
    let bytes = std::fs::read(file).ok()?;
    if bytes.len() > 128 * 1024 {
        return None;
    }
    let mut image = egui_extras::image::load_svg_bytes_with_size(
        &bytes,
        egui::load::SizeHint::Size {
            width: 64,
            height: 64,
            maintain_aspect_ratio: true,
        },
        &Default::default(),
    )
    .ok()?;
    for pixel in &mut image.pixels {
        *pixel = egui::Color32::from_white_alpha(pixel.a());
    }
    Some(image)
}
