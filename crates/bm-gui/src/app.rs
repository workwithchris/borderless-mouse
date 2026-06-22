use std::sync::Arc;

use eframe::egui;

use crate::log_capture::{LogCollector, LogEntry};
use crate::runtime::{BackgroundEvent, BackgroundTask};
use tokio::sync::mpsc;

enum AppMode {
    Server,
    Client,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Server
    }
}

pub struct BorderlessApp {
    // Config
    mode: AppMode,
    bind_addr: String,
    port: String,
    connect_addr: String,
    secret: String,

    // State
    status: String,
    is_running: bool,
    show_password: bool,
    error_message: Option<String>,

    // Background
    runtime: Option<tokio::runtime::Runtime>,
    task: BackgroundTask,
    event_rx: Option<mpsc::Receiver<BackgroundEvent>>,
    event_tx: mpsc::Sender<BackgroundEvent>,

    // Logs
    log_collector: Arc<LogCollector>,
    pending_logs: Vec<LogEntry>,
}

impl Default for BorderlessApp {
    fn default() -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        let log_collector = Arc::new(LogCollector::new(1000));
        log_collector.init_as_global_subscriber();

        Self {
            mode: AppMode::Server,
            bind_addr: "0.0.0.0".into(),
            port: "24800".into(),
            connect_addr: "192.168.1.100".into(),
            secret: String::new(),
            status: "stopped".into(),
            is_running: false,
            show_password: false,
            error_message: None,
            runtime: None,
            task: BackgroundTask::new(),
            event_rx: Some(event_rx),
            event_tx,
            log_collector,
            pending_logs: Vec::new(),
        }
    }
}

impl eframe::App for BorderlessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        egui::TopBottomPanel::top("title_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🖱  borderless-mouse");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (label, color) = if self.is_running {
                        ("● Running", egui::Color32::GREEN)
                    } else {
                        ("● Stopped", egui::Color32::GRAY)
                    };
                    ui.colored_label(color, label);
                });
            });
        });

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                egui::widgets::global_theme_preference_buttons(ui);
                ui.separator();

                ui.label("Mode");
                let mut selected = match self.mode {
                    AppMode::Server => 0,
                    AppMode::Client => 1,
                };
                ui.vertical(|ui| {
                    if ui
                        .selectable_label(selected == 0, "🖥  Server")
                        .clicked()
                    {
                        selected = 0;
                        self.mode = AppMode::Server;
                    }
                    if ui
                        .selectable_label(selected == 1, "💻  Client")
                        .clicked()
                    {
                        selected = 1;
                        self.mode = AppMode::Client;
                    }
                });
                ui.separator();

                ui.label("Connection");
                match self.mode {
                    AppMode::Server => {
                        ui.horizontal(|ui| {
                            ui.label("Bind:");
                            ui.text_edit_singleline(&mut self.bind_addr);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Port:");
                            ui.text_edit_singleline(&mut self.port);
                        });
                    }
                    AppMode::Client => {
                        ui.horizontal(|ui| {
                            ui.label("Server:");
                            ui.text_edit_singleline(&mut self.connect_addr);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Port:");
                            ui.text_edit_singleline(&mut self.port);
                        });
                    }
                }

                ui.horizontal(|ui| {
                    ui.label("Secret:");
                    if self.show_password {
                        ui.text_edit_singleline(&mut self.secret);
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.secret)
                                .password(true),
                        );
                    }
                    let eye = if self.show_password { "👁" } else { "◯" };
                    if ui.button(eye).clicked() {
                        self.show_password = !self.show_password;
                    }
                });
                ui.separator();

                // Start / Stop buttons
                if self.is_running {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("■ Stop").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::RED)
                            .min_size(egui::vec2(ui.available_width(), 36.0)),
                        )
                        .clicked()
                    {
                        self.stop_background();
                    }
                } else {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("▶ Start")
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::DARK_GREEN)
                            .min_size(egui::vec2(ui.available_width(), 36.0)),
                        )
                        .clicked()
                    {
                        self.start_background();
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Status bar
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        let color = if self.is_running {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(color, &self.status);
                    });
                });

            ui.separator();

            // Error display
            if let Some(err) = &self.error_message {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(60, 20, 20))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.colored_label(egui::Color32::LIGHT_RED, err);
                    });
                ui.separator();
            }

            // Configuration section
            egui::CollapsingHeader::new("Configuration")
                .default_open(true)
                .show(ui, |ui| {
                    match self.mode {
                        AppMode::Server => {
                            egui::Grid::new("server_config")
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Bind address:");
                                    ui.label(&self.bind_addr);
                                    ui.end_row();

                                    ui.label("Port:");
                                    ui.label(&self.port);
                                    ui.end_row();

                                    ui.label("Mode:");
                                    ui.label("Server (captures local input)");
                                    ui.end_row();

                                    if !self.secret.is_empty() {
                                        ui.label("Auth:");
                                        ui.label("enabled");
                                        ui.end_row();
                                    }
                                });
                        }
                        AppMode::Client => {
                            egui::Grid::new("client_config")
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Server address:");
                                    ui.label(&self.connect_addr);
                                    ui.end_row();

                                    ui.label("Port:");
                                    ui.label(&self.port);
                                    ui.end_row();

                                    ui.label("Mode:");
                                    ui.label("Client (emulates remote input)");
                                    ui.end_row();
                                });
                        }
                    }
                });

            // Log section
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("log_scroll")
                .stick_to_bottom(true)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let logs = &self.pending_logs;
                    if logs.is_empty() {
                        ui.label("No log entries yet.");
                    } else {
                        for entry in logs.iter().rev().take(200).rev() {
                            let color = match entry.level.as_str() {
                                "ERROR" | "error" => egui::Color32::LIGHT_RED,
                                " WARN" | "warn" => egui::Color32::YELLOW,
                                _ => egui::Color32::LIGHT_GRAY,
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&entry.timestamp)
                                        .color(egui::Color32::GRAY)
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.level)
                                        .color(color)
                                        .monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.message)
                                        .monospace(),
                                );
                            });
                        }
                    }
                });
        });

        // Request continuous repaint while running
        if self.is_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl BorderlessApp {
    fn drain_log_collector(&mut self) {
        let new_entries = self.log_collector.drain();
        if !new_entries.is_empty() {
            self.pending_logs.extend(new_entries);
            if self.pending_logs.len() > 1000 {
                self.pending_logs.drain(0..self.pending_logs.len() - 1000);
            }
        }
    }

    fn poll_events(&mut self) {
        self.drain_log_collector();

        if let Some(rx) = &mut self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    BackgroundEvent::Started => {
                        self.is_running = true;
                    }
                    BackgroundEvent::Stopped => {
                        self.is_running = false;
                        self.status = "stopped".into();
                    }
                    BackgroundEvent::Error(e) => {
                        self.is_running = false;
                        self.status = "error".into();
                        self.error_message = Some(e.clone());
                        self.pending_logs.push(LogEntry {
                            timestamp: crate::log_capture::chrono_now(),
                            level: "ERROR".into(),
                            message: e,
                        });
                    }
                    BackgroundEvent::Status(s) => {
                        self.status = s;
                    }
                    BackgroundEvent::Log(level, message) => {
                        self.pending_logs.push(LogEntry {
                            timestamp: crate::log_capture::chrono_now(),
                            level,
                            message,
                        });
                        if self.pending_logs.len() > 1000 {
                            self.pending_logs.remove(0);
                        }
                    }
                }
            }
        }
    }

    fn start_background(&mut self) {
        self.error_message = None;
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        let _guard = rt.enter();
        let tx = self.event_tx.clone();

        match self.mode {
            AppMode::Server => {
                let bind = self.bind_addr.clone();
                let port: u16 = self.port.parse().unwrap_or(24800);
                let mut task = BackgroundTask::new();
                task.start_server(bind, port, tx);
                self.task = task;
            }
            AppMode::Client => {
                let addr = self.connect_addr.clone();
                let port: u16 = self.port.parse().unwrap_or(24800);
                let mut task = BackgroundTask::new();
                task.start_client(addr, port, tx);
                self.task = task;
            }
        }

        self.runtime = Some(rt);
        self.status = "starting...".into();
    }

    fn stop_background(&mut self) {
        if let Some(runtime) = &self.runtime {
            runtime.block_on(async {
                self.task.stop().await;
            });
        }
        self.runtime = None;
        self.is_running = false;
        self.status = "stopped".into();
    }
}
