use eframe::egui::{self, Layout};

use crate::{CrabGridApp, Page};

pub fn error_page(app: &mut CrabGridApp, ui: &mut eframe::egui::Ui, err: &str) {
    ui.label("An error has occurred");

    ui.weak(err);

    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                app.page = Page::Form;
            }

            if ui.button("Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    });
}
