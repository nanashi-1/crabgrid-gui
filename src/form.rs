use std::path::Path;

use crabgrid::canvas::{Columns, Dimensions, Gaps};
use eframe::egui::{self, Layout};
use rfd::FileDialog;

use crate::{
    CrabGridApp, Message, Page,
    compiler::{compile_groups, compile_images},
};

pub fn form_page(app: &mut CrabGridApp, ui: &mut eframe::egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Input: ");
        ui.text_edit_singleline(&mut app.input);

        if ui.button("Browse").clicked() {
            let directory = FileDialog::new().pick_folder();

            if let Some(directory) = directory {
                app.input = directory.into_string().unwrap();
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Output: ");
        ui.text_edit_singleline(&mut app.output);

        if ui.button("Browse").clicked() {
            let output = if app.is_multi_directory {
                FileDialog::new().pick_folder()
            } else {
                let o = FileDialog::new().save_file();

                o.map(|o| o.with_extension("jpg"))
            };

            if let Some(output) = output {
                app.output = output.into_string().unwrap();
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Columns: ");
        ui.add(egui::DragValue::new(&mut app.columns));
    });

    ui.horizontal(|ui| {
        ui.label("Gap: ");
        ui.add(egui::DragValue::new(&mut app.gaps));
    });

    ui.horizontal(|ui| {
        ui.label("Per image width: ");
        ui.add(egui::DragValue::new(&mut app.width));
    });
    ui.horizontal(|ui| {
        ui.label("Per image height: ");
        ui.add(egui::DragValue::new(&mut app.height));
    });

    ui.checkbox(&mut app.is_multi_directory, "Multi-directory");

    ui.with_layout(
        Layout::bottom_up(egui::Align::Min).with_main_align(egui::Align::Min),
        |ui| {
            if ui
                .add_enabled(
                    !(app.input.is_empty() || app.output.is_empty()),
                    egui::Button::new("Compile"),
                )
                .clicked()
            {
                let CrabGridApp {
                    input,
                    output,
                    is_multi_directory,
                    columns,
                    gaps,
                    width,
                    height,
                    sender,
                    ..
                } = app.clone();

                std::thread::spawn(move || {
                    let mut err = None;
                    if is_multi_directory {
                        let _ = compile_groups(
                            Path::new(&input),
                            Path::new(&output),
                            Dimensions { width, height },
                            Columns(columns),
                            Gaps(gaps),
                        )
                        .inspect_err(|e| err = Some(e.to_string()));
                    } else {
                        let _ = compile_images(
                            Path::new(&input),
                            Path::new(&output),
                            Dimensions { width, height },
                            Columns(columns),
                            Gaps(gaps),
                        )
                        .inspect_err(|e| err = Some(e.to_string()));
                    }

                    if let Some(err) = err {
                        sender.send(Message::Error(err)).unwrap();

                        return;
                    }

                    sender.send(Message::Done).unwrap();
                });

                app.page = Page::Compile;
            }
        },
    );
}
