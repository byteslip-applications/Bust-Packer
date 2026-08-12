//! top_bar.rs — Custom frame titlebar graphics rendering using viewport hooks.
use eframe::egui;

/// Renders a responsive window control banner with integrated title and window control triggers.
pub fn render_custom_bar(ui: &mut egui::Ui, title: &str, show_picker: &mut bool) {
    let bar_height = 32.0;
    
    // Allocate space for the custom top window decoration row
    let (rect, response) = ui.allocate_at_least(
        egui::vec2(ui.available_width(), bar_height),
        egui::Sense::click_and_drag(),
    );

    // Render smooth slate context background for our custom window banner layout
    let bg_color = egui::Color32::from_rgb(30, 34, 42);
    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, bg_color);

    // Handle drag operations to move the application natively
    if response.dragged() && !*show_picker {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    // Layout buttons and branding text inside the allocated banner area
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            
            // Branding Title Anchor text
            ui.label(egui::RichText::new(title).strong().color(egui::Color32::from_rgb(200, 210, 230)));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                
                let btn_style = |text: &str, color: egui::Color32| {
                    egui::Button::new(egui::RichText::new(text).strong().size(13.0))
                        .fill(color)
                        .stroke(egui::Stroke::NONE)
                };

                // Close Button Trigger Action
                if ui.add(btn_style("✕", egui::Color32::from_rgb(180, 60, 60))).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
                
                ui.add_space(4.0);

                // Minimize Button Trigger Action
                if ui.add(btn_style("⎯", egui::Color32::from_rgb(60, 70, 85))).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
        });
    });
}
