use eframe::egui::{self, Layout};

use crate::{CrabGridApp, Page};

pub fn done_page(app: &mut CrabGridApp, ui: &mut eframe::egui::Ui) {
    ui.label("Done Compiling");

    ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                app.page = Page::Form;
            }

            if ui.button("Done").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    });
}
