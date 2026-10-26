use super::theme;
use crate::agent::Agent;
use crate::error::Result;
use crate::events::{Event, Sink};
use crate::model::{ChatSummary, Recent, Screen};
use crate::settings::Settings;
use crate::storage::Store;
use crate::worker::{Job, Worker};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[path = "chat_tabs.rs"]
pub(super) mod chat_tabs;

pub(super) use super::state::{Chat, Permission, Workspace};

pub(crate) struct Prism {
    pub(super) events: Option<Receiver<Event>>,
    pub(super) worker: Option<Worker>,
    pub(super) agent: Option<Agent>,
    pub(super) bridge: Option<crate::bridge::Bridge>,
    pub(super) store: Store,
    pub(super) settings: Settings,
    pub(super) settings_ui: super::settings_navigation::SettingsUi,
    pub(super) agent_catalog: super::agent_catalog::Catalog,
    pub(super) composer_menu_cache: super::agent_options::MenuCache,
    pub(super) playbooks: super::playbooks::Catalog,
    pub(super) window_effects: super::window_effects::WindowEffects,
    pub(super) splash: Option<super::splash::Splash>,
    pub(super) screen: Screen,
    pub(super) workspace: Workspace,
    pub(super) chat: Chat,
    pub(super) transcript: super::chat::transcript::TranscriptView,
    pub(super) chat_tabs: chat_tabs::Tabs,
    pub(super) landing: super::landing_content::Rotation,
    pub(super) recent: Vec<Recent>,
    pub(super) chats: Vec<ChatSummary>,
    pub(super) chat_list: super::chat_list::ChatList,
    pub(super) status: String,
    pub(super) analysis: Option<crate::events::AnalysisProgress>,
    pub(super) investigation: super::investigation::Panel,
    pub(super) error: Option<String>,
    pub(super) notifications: super::notifications::Center,
    pub(super) pending: usize,
    pub(super) palette: bool,
    pub(super) command_filter: String,
    pub(super) tool_name: String,
    pub(super) tool_args: String,
    pub(super) debug: super::debug::DebugForm,
    pub(super) reconstruction: super::reconstruction::ReconstructionForm,
    pub(super) reconstruction_root: Option<PathBuf>,
    pub(super) capture: Option<super::capture::Capture>,
    pub(super) chat_rename: Option<(i64, String)>,
    pub(super) news: Option<usize>,
}

impl Prism {
    pub fn new(context: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Result<Self> {
        let settings = Settings::load()?;
        theme::fonts(&context.egui_ctx);
        theme::apply(&context.egui_ctx, &settings);
        egui_extras::install_image_loaders(&context.egui_ctx);
        let store = Store::open(&crate::settings::support_dir().join("prism.db"))?;
        let recent = store.recent(settings.recent_limit)?;
        let chats = store.chats()?;
        let initial = initial.or_else(|| {
            (settings.desktop.launch == "The last project")
                .then(|| recent.first().map(|entry| entry.path.clone()))
                .flatten()
        });
        let screen = if !settings.research.setup_complete && initial.is_none() {
            Screen::Setup
        } else {
            Screen::Home
        };
        let debug_args = settings.tools.program_args.clone();
        let (sink, events) = Sink::channel(&context.egui_ctx);
        let worker = Worker::start(settings.clone(), sink.clone());
        let bridge = crate::bridge::Bridge::start(worker.port()?, sink.clone(), worker.runtime())?;
        let chat_tabs = chat_tabs::Tabs::new(sink);
        let splash = (!settings.desktop.intro_seen)
            .then(|| super::splash::Splash::new(context))
            .transpose()?;
        let mut app = Self {
            events: Some(events),
            worker: Some(worker),
            agent: None,
            bridge: Some(bridge),
            store,
            settings,
            settings_ui: Default::default(),
            agent_catalog: Default::default(),
            composer_menu_cache: Default::default(),
            playbooks: Default::default(),
            window_effects: super::window_effects::WindowEffects::new(context),
            splash,
            screen,
            workspace: Workspace::default(),
            chat: Chat::default(),
            transcript: Default::default(),
            chat_tabs,
            landing: Default::default(),
            recent,
            chats,
            chat_list: Default::default(),
            status: String::new(),
            analysis: None,
            investigation: Default::default(),
            error: None,
            notifications: Default::default(),
            pending: 0,
            palette: false,
            command_filter: String::new(),
            tool_name: "analyze_function".into(),
            tool_args: "{}".into(),
            debug: super::debug::DebugForm {
                thread: 1,
                args: debug_args,
                ..Default::default()
            },
            reconstruction: Default::default(),
            reconstruction_root: None,
            capture: None,
            chat_rename: None,
            news: None,
        };
        app.agent_catalog.start(&context.egui_ctx);
        if let Some(path) = initial {
            app.enqueue(Job::Open(path));
        } else if let Err(error) = app.restore_saved_chat(None) {
            app.error = Some(error.to_string());
        }
        Ok(app)
    }

    pub(super) fn enqueue(&mut self, job: Job) {
        if let Err(error) = self.submit(job) {
            self.error = Some(error.to_string());
        }
    }

    pub(super) fn submit(&mut self, job: Job) -> Result<()> {
        if matches!(job, Job::Open(_)) && (self.chat_tabs.any_busy(&self.chat) || self.pending > 0)
        {
            return Err(crate::error::failure(
                "Wait for the current operation before opening another file",
            ));
        }
        if matches!(job, Job::Open(_)) {
            self.save_inspector_note()?;
            self.save_all_chat_drafts()?;
        }
        self.worker
            .as_ref()
            .ok_or("The analysis worker has closed")?
            .submit(job)?;
        self.pending += 1;
        Ok(())
    }

    pub(super) fn content(&mut self, ui: &mut egui::Ui) {
        match self.screen {
            Screen::Home => self.home(ui),
            Screen::Workbench => self.workbench(ui),
            Screen::Settings => self.settings_screen(ui),
            Screen::Setup => self.setup_screen(ui),
        }
    }

    fn dropped_files(&mut self, context: &egui::Context) {
        let dropped = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .find_map(|file| file.path.clone())
        });
        if let Some(path) = dropped {
            self.enqueue(Job::Open(path));
        }
    }
}

impl eframe::App for Prism {
    fn update(&mut self, context: &egui::Context, frame: &mut eframe::Frame) {
        self.finish_capture(context);
        self.receive(context);
        self.update_inspector(context);
        self.chat_tabs.retired.retain(|agent| !agent.has_exited());
        if !self.chat_tabs.retired.is_empty() {
            context.request_repaint_after(std::time::Duration::from_millis(80));
        }
        self.agent_catalog.poll(context);
        self.poll_skill_libraries(context);
        super::composer_drop::update_position(context, frame);
        self.flush_settings(context);

        if self.show_splash(context) {
            self.landing.pause();
            return;
        }
        self.shortcuts(context);

        if self.screen != Screen::Home || !self.chat.messages.is_empty() {
            self.landing.pause();
        }
        self.shell(context);
        if self.playbooks.loading() {
            context.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.dropped_files(context);
        self.command_palette(context);
        self.rename_chat_dialog(context);
        self.whats_new(context);
        self.show_notifications(context);
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        if let Err(error) = self.save_inspector_note() {
            eprintln!("Prism: couldn't save the Inspector note while closing: {error}");
        }
        if let Err(error) = self.save_chats_on_exit() {
            eprintln!("Prism: couldn't save open chats while closing: {error}");
        }
        if self.settings_ui.pending_save.take().is_some()
            && let Err(error) = self.settings.save()
        {
            eprintln!("Prism: couldn't save settings while closing: {error}");
        }

        if let Some(agent) = &self.agent {
            agent.cancel();
        }
        if let Some(worker) = &self.worker {
            worker.cancel();
        }
        self.events.take();
        self.bridge.take();
        self.agent.take();
        self.chat_tabs.parked.clear();
        self.chat_tabs.retired.clear();
        self.worker.take();
    }
}
