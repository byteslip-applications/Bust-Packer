use eframe::egui;
use std::sync::mpsc::Receiver;

mod task;
mod path_cache;
mod worker;
mod top_bar;
mod license;
use task::{Job, TaskMsg};

pub fn run() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([820.0, 920.0])
            .with_min_inner_size([640.0, 700.0])
            .with_decorations(false),
        ..Default::default()
    };
    eframe::run_native(
        "Bust Packer",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(BustPackerApp::new()))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mono_candidates = [
        "/usr/share/fonts/truetype/recursive/Recursive_Monospace-Regular.ttf",
        "/usr/share/fonts/TTF/Recursive_Monospace-Regular.ttf",
        "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf",
        "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
    ];
    for path in &mono_candidates {
        if let Ok(data) = std::fs::read(path) {
            fonts.font_data.insert("mono".to_owned(), egui::FontData::from_owned(data).into());
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "mono".to_owned());
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "mono".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

struct CustomFilePicker {
    current_dir: std::path::PathBuf,
    items: Vec<(String, bool)>,
    search_filter: String,
    show_picker: bool,
    target_mode_dir: bool,
}

struct BustPackerApp {
    target_path: String,
    status: String,
    log: String,
    preview_stats: String,
    show_pack: bool,
    show_unpack: bool,
    exclude_patterns: Vec<String>,
    is_busy: bool,
    for_ai: bool,
    agreed_to_license: bool,
    license_text: String,
    last_output_file: String,
    picker: CustomFilePicker,
    job_tx: std::sync::mpsc::Sender<Job>,
    rx: Option<Receiver<TaskMsg>>,
}

impl BustPackerApp {
    pub fn new() -> Self {
        let last = path_cache::load_last_path();
        let agreed = path_cache::load_agreement_state();
        let (job_tx, job_rx) = std::sync::mpsc::channel::<Job>();
        let (result_tx, result_rx) = std::sync::mpsc::channel::<TaskMsg>();

        std::thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let progress_tx = result_tx.clone();
                let msg = match job {
                    Job::Pack { path, excludes, for_ai } => {
                        worker::do_pack_extended(&path, &excludes, for_ai, &progress_tx)
                    }
                    Job::Unpack(path) => worker::do_unpack(&path, &progress_tx),
                };
                let _ = result_tx.send(msg);
            }
        });

        let mut starting_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        if !last.is_empty() {
            let p = std::path::Path::new(&last);
            if p.exists() {
                starting_dir = if p.is_dir() { p.to_path_buf() } else { p.parent().unwrap_or(p).to_path_buf() };
            }
        }

        let license_data = license::get_bsl_text().to_string();

        let mut app = Self {
            target_path: last,
            status: "Ready".to_string(),
            log: String::new(),
            preview_stats: String::new(),
            show_pack: false,
            show_unpack: false,
            exclude_patterns: Vec::new(),
            is_busy: false,
            for_ai: true,
            agreed_to_license: agreed,
            license_text: license_data,
            last_output_file: String::new(),
            picker: CustomFilePicker {
                current_dir: starting_dir,
                items: Vec::new(),
                search_filter: String::new(),
                show_picker: false,
                target_mode_dir: true,
            },
            job_tx,
            rx: Some(result_rx),
        };
        app.update_picker_items();
        app
    }

    fn update_picker_items(&mut self) {
        self.picker.items.clear();
        if let Ok(entries) = std::fs::read_dir(&self.picker.current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = path.is_dir();
                
                if is_dir {
                    self.picker.items.push((name, true));
                } else if !self.picker.target_mode_dir && name.ends_with(".txt") {
                    self.picker.items.push((name, false));
                }
            }
        }
        self.picker.items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    }

    fn refresh(&mut self) {
        if self.is_busy { return; }
        self.status = "Ready".to_string();
        self.log.clear();
        self.preview_stats.clear();
        self.show_pack = false;
        self.show_unpack = false;
        self.last_output_file.clear();
    }

    fn poll_task(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    TaskMsg::Progress { files_done, files_total, .. } => {
                        self.status = format!("Working {}/{}", files_done, files_total);
                    }
                    TaskMsg::PackDone { files, output, log_append, .. } => {
                        self.status = format!("Packed {} files successfully", files);
                        self.log.push_str(&log_append);
                        self.log.push_str(&format!("\nOutput path: {}\n", output));
                        self.last_output_file = output.clone();
                        self.is_busy = false;
                    }
                    TaskMsg::UnpackDone { output, log_append, .. } => {
                        self.status = "Unpacked successfully".to_string();
                        self.log.push_str(&log_append);
                        self.log.push_str(&format!("\nOutput path: {}\n", output));
                        self.is_busy = false;
                    }
                    TaskMsg::Error(e) => {
                        self.status = format!("Error: {}", e);
                        self.log.push_str(&format!("\nERROR: {}\n", e));
                        self.is_busy = false;
                    }
                }
            }
        }
    }
}

impl BustPackerApp {
    fn run_inspection(&mut self) {
        let path = self.target_path.trim().to_string();
        if path.is_empty() || !std::path::Path::new(&path).exists() {
            self.preview_stats = "Path does not exist.".to_string();
            self.show_pack = false;
            self.show_unpack = false;
            self.status = "Invalid path".to_string();
            return;
        }
        let p = std::path::Path::new(&path);

        if p.is_dir() {
            self.show_pack = true;
            self.show_unpack = false;
            
            let gitignore_patterns = worker::load_gitignore_patterns_public(p);
            let mut text_files_count = 0;
            let mut binary_files_count = 0;
            let mut total_bytes = 0;
            let mut total_lines = 0;
            let mut largest_file_name = String::new();
            let mut largest_file_size = 0;

            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(rel) = path.strip_prefix(p) {
                            if worker::is_ignored_path_public(rel, &gitignore_patterns) {
                                binary_files_count += 1;
                                continue;
                            }
                            if let Ok(data) = std::fs::read(&path) {
                                if crate::packer::is_binary_file(&path, &data) {
                                    binary_files_count += 1;
                                } else {
                                    text_files_count += 1;
                                    let size = data.len();
                                    total_bytes += size;
                                    if size > largest_file_size {
                                        largest_file_size = size;
                                        largest_file_name = rel.to_string_lossy().to_string();
                                    }
                                    let content_str = String::from_utf8_lossy(&data);
                                    total_lines += content_str.lines().count();
                                }
                            }
                        }
                    }
                }
            }

            self.preview_stats = format!(
                "=================================================================\n\
                 DIRECTORY INSPECTION REPORT\n\
                 =================================================================\n\
                 TARGET PATH          : {}\n\
                 PROJECT NAME         : {}\n\
                 TOTAL SOURCE FILES   : {}\n\
                 TOTAL LINES OF CODE  : {}\n\
                 TOTAL ESTIMATED SIZE : {:.2} KB\n\
                 BINARY/IGNORED FILES : {}\n\
                 HAS `.gitignore`     : {}\n\
                 -----------------------------------------------------------------\n\
                 LARGEST SOURCE FILE  : {}\n\
                 LARGEST FILE SIZE    : {:.2} KB\n\
                 =================================================================",
                path,
                p.file_name().and_then(|n| n.to_str()).unwrap_or("project"),
                text_files_count,
                total_lines,
                (total_bytes as f64) / 1024.0,
                binary_files_count,
                if p.join(".gitignore").exists() { "YES" } else { "NO" },
                if largest_file_name.is_empty() { "N/A".to_string() } else { largest_file_name },
                (largest_file_size as f64) / 1024.0
            );
        } else if p.is_file() {
            self.show_pack = false;
            self.show_unpack = true;
            
            if let Ok(content) = std::fs::read_to_string(p) {
                let lines: Vec<&str> = content.lines().collect();
                let mut app_name = "Unknown".to_string();
                let mut packed_files_count = 0;
                
                if lines.iter().any(|l| l.contains("=== BUSTPACKER:MONOLITHIC_TXT_V1 ===")) {
                    for line in &lines {
                        if line.starts_with("ROOT_NAME: ") {
                            app_name = line["ROOT_NAME: ".len()..].trim().to_string();
                        }
                        if line.starts_with("=== FILE: ") {
                            packed_files_count += 1;
                        }
                    }
                    self.preview_stats = format!(
                        "=================================================================\n\
                         MONOLITHIC SNAPSHOT REPORT\n\
                         =================================================================\n\
                         TARGET PACK FILE     : {}\n\
                         ARCHIVE PROJECT NAME : {}\n\
                         TOTAL PACKED FILES   : {}\n\
                         ARCHIVE SNAPSHOT SIZE: {:.2} KB\n\
                         =================================================================",
                        path, app_name, packed_files_count, (content.len() as f64) / 1024.0
                    );
                } else {
                    self.preview_stats = "Error: Selected text file format does not contain valid BustPacker snapshot headers.".to_string();
                    self.show_unpack = false;
                }
            } else {
                self.preview_stats = "Error: Failed to safely parse text token content data from the disk target path.".to_string();
                self.show_unpack = false;
            }
        }
    }
}

impl eframe::App for BustPackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_task();

        if self.picker.show_picker {
            let mut close_requested = false;
            egui::Window::new(if self.picker.target_mode_dir { "Select Target Directory" } else { "Select Target File" })
                .collapsible(false)
                .resizable(true)
                .default_size([550.0, 450.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Current:");
                        ui.code(self.picker.current_dir.to_string_lossy().to_string());
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button("Parent Dir").clicked() {
                            if let Some(parent) = self.picker.current_dir.parent() {
                                self.picker.current_dir = parent.to_path_buf();
                                self.update_picker_items();
                            }
                        }
                        ui.label("Filter:");
                        ui.text_edit_singleline(&mut self.picker.search_filter);
                    });
                    ui.add_space(6.0);

                    let filter_lower = self.picker.search_filter.to_lowercase();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let items = self.picker.items.clone();
                        for (name, is_dir) in items {
                            if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower) {
                                continue;
                            }
                            let prefix = if is_dir { "📁 " } else { "📄 " };
                            if ui.selectable_label(false, format!("{}{}", prefix, name)).clicked() {
                                let target = self.picker.current_dir.join(&name);
                                if is_dir {
                                    self.picker.current_dir = target;
                                    self.update_picker_items();
                                } else {
                                    self.target_path = target.to_string_lossy().to_string();
                                    close_requested = true;
                                }
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if self.picker.target_mode_dir {
                            if ui.button("Select Current Directory").clicked() {
                                self.target_path = self.picker.current_dir.to_string_lossy().to_string();
                                close_requested = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            close_requested = true;
                        }
                    });
                });

            if close_requested {
                self.picker.show_picker = false;
            }
        }

        egui::TopBottomPanel::top("custom_title_bar")
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                top_bar::render_custom_bar(ui, "Bust Packer — AI Source Code Organizer", &mut self.picker.show_picker);
            });

        if !self.agreed_to_license {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Bust Packer — End User License Agreement");
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::LIGHT_YELLOW, "⚠️ Action Required: This tool is distributed under a source-available enterprise model.");
                ui.add_space(8.0);

                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.license_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(18)
                            .lock_focus(true)
                    );
                });

                ui.add_space(16.0);
                if ui.button("I Accept & Acknowledge the BSL 1.1 Non-Commercial Terms").clicked() {
                    self.agreed_to_license = true;
                    path_cache::save_last_path(&self.target_path, true);
                }
            });
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Target Path:");
                ui.text_edit_singleline(&mut self.target_path);
                
                if ui.button("📁 Browse Folder").clicked() {
                    self.picker.target_mode_dir = true;
                    self.picker.show_picker = true;
                    self.update_picker_items();
                }
                if ui.button("📄 Browse File").clicked() {
                    self.picker.target_mode_dir = false;
                    self.picker.show_picker = true;
                    self.update_picker_items();
                }
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.is_busy, |ui| {
                    if ui.button("Run Inspect").clicked() {
                        self.run_inspection();
                    }
                    if ui.button("Clear Log").clicked() {
                        self.refresh();
                    }
                });
                
                ui.checkbox(&mut self.for_ai, "For AI");
            });

            ui.add_space(12.0);
            ui.label("Status: ");
            ui.colored_label(egui::Color32::LIGHT_BLUE, &self.status);
            
            ui.add_space(8.0);
            ui.label("Inspection Detailed Report:");
            
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .id_salt("inspect_scroll")
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.preview_stats)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(12)
                            .lock_focus(true)
                    );
                });

            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if self.show_pack && !self.is_busy {
                    let btn_label = if self.for_ai { "Pack to Monolithic Text File (-Prompt.txt)" } else { "Pack to Monolithic Text File (.txt)" };
                    if ui.button(btn_label).clicked() {
                        self.is_busy = true;
                        let _ = self.job_tx.send(Job::Pack {
                            path: self.target_path.clone(),
                            excludes: self.exclude_patterns.clone(),
                            for_ai: self.for_ai,
                        });
                    }
                }

                if self.show_unpack && !self.is_busy {
                    if ui.button("Unpack Project Monolithic File").clicked() {
                        self.is_busy = true;
                        let _ = self.job_tx.send(Job::Unpack(self.target_path.clone()));
                    }
                }

                if !self.last_output_file.is_empty() {
                    if ui.button("📋 Copy to Clipboard").clicked() {
                        if let Ok(content) = std::fs::read_to_string(&self.last_output_file) {
                            ui.ctx().copy_text(content);
                            self.status = "Copied context to clipboard successfully!".to_string();
                        }
                    }
                    
                    if ui.button("📂 Open Output Folder").clicked() {
                        let target_dir = worker::get_bpacks_dir();
                        let _ = if cfg!(target_os = "windows") {
                            std::process::Command::new("explorer").arg(&target_dir).spawn()
                        } else if cfg!(target_os = "macos") {
                            std::process::Command::new("open").arg(&target_dir).spawn()
                        } else {
                            std::process::Command::new("xdg-open").arg(&target_dir).spawn()
                        };
                    }
                }
            });

            ui.add_space(12.0);
            ui.label("Operation Logs:");
            egui::ScrollArea::vertical()
                .id_salt("log_scroll")
                .show(ui, |ui| {
                    ui.label(&self.log);
                });
        });
    }
}
