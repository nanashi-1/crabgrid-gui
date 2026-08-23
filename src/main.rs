use crabgrid::canvas::{Columns, Dimensions, Gaps};
use eframe::{
    App, NativeOptions,
    egui::{self, Layout},
};
use rfd::FileDialog;
use std::{path::Path, sync::mpsc};

use crate::compiler::{compile_groups, compile_images};

mod compiler;

#[derive(Default, Clone)]
enum Page {
    #[default]
    Form,
    Compile,
    Done,
    Error(String),
}

#[derive(Clone)]
enum Message {
    Done,
    Error(String),
}

struct CrabGridApp {
    page: Page,
    input: String,
    output: String,
    is_multi_directory: bool,

    columns: u32,
    gaps: u32,
    width: u32,
    height: u32,

    receiver: Option<mpsc::Receiver<Message>>,
    sender: mpsc::Sender<Message>,
}

impl Clone for CrabGridApp {
    fn clone(&self) -> Self {
        Self {
            page: self.page.clone(),
            input: self.input.clone(),
            output: self.output.clone(),
            is_multi_directory: self.is_multi_directory,
            columns: self.columns,
            gaps: self.gaps,
            width: self.width,
            height: self.height,
            receiver: None,
            sender: self.sender.clone(),
        }
    }
}

impl CrabGridApp {
    fn form_page(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Input: ");
            ui.text_edit_singleline(&mut self.input);

            if ui.button("Browse").clicked() {
                let directory = FileDialog::new().pick_folder();

                if let Some(directory) = directory {
                    self.input = directory.into_string().unwrap();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Output: ");
            ui.text_edit_singleline(&mut self.output);

            if ui.button("Browse").clicked() {
                let output = if self.is_multi_directory {
                    FileDialog::new().pick_folder()
                } else {
                    FileDialog::new()
                        .add_filter("Image", &["jpg", "jpeg", "png", "webp"])
                        .save_file()
                };

                if let Some(output) = output {
                    self.output = output.into_string().unwrap();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Columns: ");
            ui.add(egui::DragValue::new(&mut self.columns));
        });

        ui.horizontal(|ui| {
            ui.label("Gap: ");
            ui.add(egui::DragValue::new(&mut self.gaps));
        });

        ui.horizontal(|ui| {
            ui.label("Per image width: ");
            ui.add(egui::DragValue::new(&mut self.width));
        });
        ui.horizontal(|ui| {
            ui.label("Per image height: ");
            ui.add(egui::DragValue::new(&mut self.height));
        });

        ui.checkbox(&mut self.is_multi_directory, "Multi-directory");

        ui.with_layout(
            Layout::bottom_up(egui::Align::Min).with_main_align(egui::Align::Min),
            |ui| {
                if ui.button("Compile").clicked() {
                    let (tx, rx) = mpsc::channel();

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
                    } = self.clone();

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

                    self.page = Page::Compile;
                }
            },
        );
    }

    fn compile_page(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Compiling");
        });
    }

    fn done_page(&mut self, ui: &mut eframe::egui::Ui) {
        ui.label("Done Compiling");

        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    self.page = Page::Form;
                }

                if ui.button("Done").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    fn error_page(&mut self, ui: &mut eframe::egui::Ui, err: &str) {
        ui.label("An error has occurred");

        ui.weak(err);

        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                if ui.button("Back").clicked() {
                    self.page = Page::Form;
                }

                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

impl Default for CrabGridApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            columns: 5,
            gaps: 2,
            width: 300,
            height: 300,
            page: Page::default(),
            input: String::default(),
            output: String::default(),
            is_multi_directory: false,
            receiver: Some(receiver),
            sender,
        }
    }
}

impl App for CrabGridApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_theme(egui::Theme::Light);
        ui.ctx().set_pixels_per_point(1.4);

        egui::CentralPanel::default().show(ui, |ui| match self.page.clone() {
            Page::Form => self.form_page(ui),
            Page::Compile => self.compile_page(ui),
            Page::Done => self.done_page(ui),
            Page::Error(err) => self.error_page(ui, &err),
        });

        if let Some(ref receiver) = self.receiver
            && let Ok(message) = receiver.try_recv()
        {
            match message {
                Message::Done => self.page = Page::Done,
                Message::Error(err) => self.page = Page::Error(err),
            }
        }
    }
}

fn main() {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 300.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "CrabGrid GUI",
        options,
        Box::new(|_cc| Ok(Box::new(CrabGridApp::default()))),
    )
    .unwrap();
}
