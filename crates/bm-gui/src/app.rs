use std::sync::Arc;

use bm_core::config::ScreenPosition;
use bm_core::network::{apply_config_via_gui, detect_interfaces, generate_commands, NetworkConfig};
use eframe::egui;
use egui::Color32;

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

    // Screen layout
    screen_positions: Vec<ScreenPosition>,
    selected_screen: Option<usize>,
    new_screen_name: String,

    // Network setup
    net_ip: String,
    net_mask: String,
    net_detected_ifaces: String,
    net_last_commands: String,
    net_apply_result: String,

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
            screen_positions: vec![ScreenPosition {
                name: hostname(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }],
            selected_screen: Some(0),
            new_screen_name: String::new(),
            net_ip: "192.168.2.1".into(),
            net_mask: "255.255.255.0".into(),
            net_detected_ifaces: String::new(),
            net_last_commands: String::new(),
            net_apply_result: String::new(),
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

            // Screen layout section
            ui.separator();
            egui::CollapsingHeader::new("Screen Layout (Deskflow-style)")
                .default_open(true)
                .show(ui, |ui| {
                    self.ui_screen_layout(ui);
                });

            // Network setup section
            ui.separator();
            egui::CollapsingHeader::new("Network Setup (Direct Link)")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("network_grid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("IP Address:");
                            ui.text_edit_singleline(&mut self.net_ip);
                            ui.end_row();

                            ui.label("Subnet Mask:");
                            ui.text_edit_singleline(&mut self.net_mask);
                            ui.end_row();
                        });

                    ui.horizontal(|ui| {
                        if ui.button("🔍 Detect Interfaces").clicked() {
                            let ifaces = detect_interfaces();
                            self.net_detected_ifaces = if ifaces.is_empty() {
                                "none found".into()
                            } else {
                                ifaces.join(", ")
                            };
                        }
                        ui.label(&self.net_detected_ifaces);
                    });

                    if ui.button("📋 Show Commands").clicked() {
                        let config = NetworkConfig {
                            ip_address: Some(self.net_ip.clone()),
                            subnet_mask: Some(self.net_mask.clone()),
                        };
                        let cmds = generate_commands(&config);
                        self.net_last_commands = if cmds.is_empty() {
                            "no commands — check IP".into()
                        } else {
                            cmds.join("\n")
                        };
                    }
                    if !self.net_last_commands.is_empty() {
                        let mut frame = egui::Frame::group(ui.style());
                        frame = frame.inner_margin(egui::Margin::same(4));
                        frame.show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(&self.net_last_commands)
                                    .monospace()
                                    .size(12.0),
                            );
                        });
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("⚡ Apply Now").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::DARK_BLUE)
                            .min_size(egui::vec2(ui.available_width(), 28.0)),
                        )
                        .clicked()
                    {
                        let config = NetworkConfig {
                            ip_address: Some(self.net_ip.clone()),
                            subnet_mask: Some(self.net_mask.clone()),
                        };
                        match apply_config_via_gui(&config) {
                            Ok(msg) => {
                                self.net_apply_result = msg;
                                self.pending_logs.push(LogEntry {
                                    timestamp: crate::log_capture::chrono_now(),
                                    level: " INFO".into(),
                                    message: "network config applied successfully".into(),
                                });
                            }
                            Err(e) => {
                                self.net_apply_result = e.clone();
                                self.pending_logs.push(LogEntry {
                                    timestamp: crate::log_capture::chrono_now(),
                                    level: "ERROR".into(),
                                    message: format!("network apply failed: {e}"),
                                });
                            }
                        }
                    }
                    if !self.net_apply_result.is_empty() {
                        ui.label(
                            egui::RichText::new(&self.net_apply_result)
                                .color(if self.net_apply_result.contains("failed")
                                    || self.net_apply_result.contains("not found")
                                {
                                    egui::Color32::LIGHT_RED
                                } else {
                                    egui::Color32::LIGHT_GREEN
                                }),
                        );
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

    fn ui_screen_layout(&mut self, ui: &mut egui::Ui) {
        ui.label("Arrange screens by position. Cursor exits one screen edge and enters the adjacent screen.");

        // --- Visual canvas ---
        if !self.screen_positions.is_empty() {
            let (min_x, min_y, max_x, max_y) = self.screen_positions.iter().fold(
                (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
                |(mx, my, max_x, max_y), s| {
                    (
                        mx.min(s.x),
                        my.min(s.y),
                        max_x.max(s.x + s.width as i32),
                        max_y.max(s.y + s.height as i32),
                    )
                },
            );
            let grid_w = (max_x - min_x).max(100) as f32;
            let grid_h = (max_y - min_y).max(100) as f32;
            let scale = (ui.available_width() / grid_w).min(200.0 / grid_h).min(2.0);

            let (resp, painter) = ui.allocate_painter(
                egui::vec2(grid_w * scale + 4.0, grid_h * scale + 4.0),
                egui::Sense::click(),
            );
            let origin = resp.rect.min + egui::vec2(2.0, 2.0);

            for (i, screen) in self.screen_positions.iter().enumerate() {
                let rx = origin.x + (screen.x - min_x) as f32 * scale;
                let ry = origin.y + (screen.y - min_y) as f32 * scale;
                let rw = screen.width as f32 * scale;
                let rh = screen.height as f32 * scale;
                let rect = egui::Rect::from_min_size(egui::pos2(rx, ry), egui::vec2(rw, rh));

                let is_selected = self.selected_screen == Some(i);
                let colors = SCREEN_COLORS[i % SCREEN_COLORS.len()];
                let fill = if is_selected { colors.highlight } else { colors.fill };
                painter.rect_filled(rect, 4.0, fill);
                painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, colors.stroke), egui::StrokeKind::Outside);

                painter.text(
                    egui::pos2(rect.center().x, rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    &screen.name,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );

                if resp.clicked() && rect.contains(resp.interact_pointer_pos().unwrap_or(egui::Pos2::ZERO)) {
                    self.selected_screen = Some(i);
                }
            }
        }

        // --- Edit fields ---
        ui.separator();
        if let Some(idx) = self.selected_screen {
            if idx < self.screen_positions.len() {
                let screen = &mut self.screen_positions[idx];
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut screen.name);
                });
                ui.horizontal(|ui| {
                    ui.label("X:");
                    let mut x = screen.x;
                    if ui.add(egui::DragValue::new(&mut x)).changed() {
                        screen.x = x;
                    }
                    ui.label("Y:");
                    let mut y = screen.y;
                    if ui.add(egui::DragValue::new(&mut y)).changed() {
                        screen.y = y;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("W:");
                    let mut w = screen.width;
                    if ui.add(egui::DragValue::new(&mut w).range(1..=7680)).changed() {
                        screen.width = w;
                    }
                    ui.label("H:");
                    let mut h = screen.height;
                    if ui.add(egui::DragValue::new(&mut h).range(1..=4320)).changed() {
                        screen.height = h;
                    }
                });
                if ui.button("Remove screen").clicked() {
                    self.screen_positions.remove(idx);
                    self.selected_screen = self.screen_positions.len().checked_sub(1);
                }
            }
        }

        // --- Add screen ---
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("New screen:");
            ui.text_edit_singleline(&mut self.new_screen_name);
            if ui.button("Add").clicked() && !self.new_screen_name.is_empty() {
                let offset = self.screen_positions.len() as i32 * 200;
                self.screen_positions.push(ScreenPosition {
                    name: self.new_screen_name.clone(),
                    x: offset,
                    y: 0,
                    width: 1920,
                    height: 1080,
                });
                self.selected_screen = Some(self.screen_positions.len() - 1);
                self.new_screen_name.clear();
            }
        });
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "unknown".into())
}

#[derive(Clone, Copy)]
struct ScreenColors {
    fill: Color32,
    highlight: Color32,
    stroke: Color32,
}

const SCREEN_COLORS: &[ScreenColors] = &[
    ScreenColors { fill: Color32::from_rgb(30, 80, 160), highlight: Color32::from_rgb(60, 140, 220), stroke: Color32::from_rgb(100, 180, 255) },
    ScreenColors { fill: Color32::from_rgb(160, 60, 30), highlight: Color32::from_rgb(220, 100, 60), stroke: Color32::from_rgb(255, 140, 100) },
    ScreenColors { fill: Color32::from_rgb(30, 130, 60), highlight: Color32::from_rgb(60, 200, 100), stroke: Color32::from_rgb(100, 255, 140) },
    ScreenColors { fill: Color32::from_rgb(130, 30, 110), highlight: Color32::from_rgb(200, 60, 180), stroke: Color32::from_rgb(255, 100, 220) },
];
