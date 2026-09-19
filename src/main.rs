use eframe::{
    App, NativeOptions,
    egui::{self},
};
use std::sync::mpsc;

use crate::{done::done_page, error::error_page, form::form_page, loading::compile_page};

mod compiler;
mod done;
mod error;
mod form;
mod loading;

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
            Page::Form => form_page(self, ui),
            Page::Compile => compile_page(ui),
            Page::Done => done_page(self, ui),
            Page::Error(err) => error_page(self, ui, &err),
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
        ..Default::default()
    };

    eframe::run_native(
        "CrabGrid GUI",
        options,
        Box::new(|_cc| Ok(Box::new(CrabGridApp::default()))),
    )
    .unwrap();
}
