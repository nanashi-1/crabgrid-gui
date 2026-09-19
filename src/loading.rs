pub fn compile_page(ui: &mut eframe::egui::Ui) {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label("Compiling");
    });
}
