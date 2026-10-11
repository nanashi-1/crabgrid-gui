#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crabgrid::canvas::{Columns, Dimensions, Gaps};
use rfd::FileDialog;
use slint::{ComponentHandle, ModelRc, VecModel};

slint::include_modules!();

mod compiler;
use compiler::{ImageEditorItem, ProcessedImage};

#[derive(Clone)]
struct GroupState {
    name: String,
    folder_path: PathBuf,
    items: Vec<ImageEditorItem>,
    initial_items: Vec<ImageEditorItem>,
}

struct EditorState {
    groups: Vec<GroupState>,
    active_group: usize,
    columns: u32,
    gaps: u32,
    tile_width: u32,
    tile_height: u32,
    output_path: PathBuf,
    is_multi: bool,
}

fn sync_editor_ui(ui: &MainWindow, state: &EditorState) {
    if state.groups.is_empty() {
        return;
    }
    let group = &state.groups[state.active_group];
    let total_images = group.items.len() as i32;
    let columns = state.columns.max(1) as i32;
    let total_rows = (total_images as f32 / columns as f32).ceil().max(1.0) as i32;

    let card_width = 120.0f32;
    let card_height = 145.0f32;
    let gap = 8.0f32;

    let grid_canvas_width = (columns as f32) * (card_width + gap) + gap;
    let grid_canvas_height = (total_rows as f32) * (card_height + gap) + gap;

    ui.set_total_images(total_images);
    ui.set_total_rows(total_rows);
    ui.set_columns(columns);
    ui.set_tile_card_width(card_width);
    ui.set_tile_card_height(card_height);
    ui.set_tile_gap(gap);
    ui.set_grid_canvas_width(grid_canvas_width);
    ui.set_grid_canvas_height(grid_canvas_height);

    ui.set_has_multiple_folders(state.is_multi);
    ui.set_folder_name(group.name.clone().into());
    ui.set_folder_index(state.active_group as i32);
    ui.set_total_folders(state.groups.len() as i32);

    let items_model: Vec<ImageItem> = group
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let row = (idx as i32) / columns;
            let col = (idx as i32) % columns;
            ImageItem {
                id: item.id as i32,
                index: idx as i32,
                name: item.file_name.clone().into(),
                path: item.current_path.to_string_lossy().to_string().into(),
                thumbnail: item.current_thumbnail(),
                rotation: item.rotation,
                col,
                row,
            }
        })
        .collect();

    let model = Rc::new(VecModel::from(items_model));
    ui.set_images(ModelRc::from(model));
}

fn update_selection(ui: &MainWindow, state: &EditorState, selected: i32) {
    if state.groups.is_empty() {
        ui.set_selected_index(-1);
        ui.set_selected_image_name("".into());
        return;
    }
    let group = &state.groups[state.active_group];
    if selected >= 0 && (selected as usize) < group.items.len() {
        ui.set_selected_index(selected);
        ui.set_selected_image_name(group.items[selected as usize].file_name.clone().into());
    } else {
        ui.set_selected_index(-1);
        ui.set_selected_image_name("".into());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;
    let editor_state: Arc<Mutex<Option<EditorState>>> = Arc::new(Mutex::new(None));

    // Browse Input Folder Callback
    let ui_weak = main_window.as_weak();
    main_window.on_browse_input(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut dialog = FileDialog::new();
        let current_input = ui.get_input_path().to_string();
        if !current_input.is_empty() {
            let path = Path::new(&current_input);
            if path.exists() {
                dialog = dialog.set_directory(path);
            }
        }

        if let Some(folder) = dialog.pick_folder() {
            ui.set_input_path(folder.to_string_lossy().to_string().into());
        }
    });

    // Browse Output Path Callback
    let ui_weak = main_window.as_weak();
    main_window.on_browse_output(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let is_multi = ui.get_is_multi_directory();
        let current_output = ui.get_output_path().to_string();

        let mut dialog = FileDialog::new();
        if !current_output.is_empty() {
            let path = Path::new(&current_output);
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    dialog = dialog.set_directory(parent);
                }
            }
        }

        let selected: Option<PathBuf> = if is_multi {
            dialog.pick_folder()
        } else {
            dialog
                .add_filter("JPEG Image (*.jpg, *.jpeg)", &["jpg", "jpeg"])
                .save_file()
                .map(|mut path| {
                    if path.extension().is_none() {
                        path.set_extension("jpg");
                    }
                    path
                })
        };

        if let Some(output) = selected {
            ui.set_output_path(output.to_string_lossy().to_string().into());
        }
    });

    // Proceed to Edit Grid Callback (with async loading screen)
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_proceed_to_edit(move || {
        let Some(ui) = ui_weak.upgrade() else { return };

        let input_str = ui.get_input_path().to_string();
        let output_str = ui.get_output_path().to_string();
        let is_multi = ui.get_is_multi_directory();
        let columns = ui.get_columns().max(1) as u32;
        let gaps = ui.get_gaps().max(0) as u32;
        let width = ui.get_tile_width().max(10) as u32;
        let height = ui.get_tile_height().max(10) as u32;

        if input_str.is_empty() || output_str.is_empty() {
            return;
        }

        ui.set_loading_text("Scanning directory and loading images, please wait...".into());
        ui.set_current_page(AppState::Loading);

        let thread_ui_weak = ui_weak.clone();
        let thread_state = Arc::clone(&state_clone);
        std::thread::spawn(move || {
            let input_path = PathBuf::from(&input_str);
            let output_path = PathBuf::from(&output_str);

            if !input_path.is_dir() {
                let err_msg = format!("Input path is not a directory: {}", input_str);
                let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_error_message(err_msg.into());
                    ui.set_current_page(AppState::Error);
                });
                return;
            }

            let mut groups: Vec<GroupState> = Vec::new();

            if is_multi {
                let read_res = std::fs::read_dir(&input_path);
                let entries = match read_res {
                    Ok(entries) => entries,
                    Err(err) => {
                        let err_msg = format!("Failed to read input directory: {}", err);
                        let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_error_message(err_msg.into());
                            ui.set_current_page(AppState::Error);
                        });
                        return;
                    }
                };

                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        let folder_name_clone = folder_name.clone();
                        let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_loading_text(format!("Loading folder: {}...", folder_name_clone).into());
                        });

                        let mut img_paths: Vec<_> = match std::fs::read_dir(&path) {
                            Ok(dir) => dir
                                .filter_map(|e| e.ok())
                                .map(|e| e.path())
                                .filter(|p| compiler::is_image_file(p))
                                .collect(),
                            Err(_) => continue,
                        };

                        if img_paths.is_empty() {
                            continue;
                        }

                        img_paths.sort();

                        let mut items = Vec::new();
                        for (idx, p) in img_paths.into_iter().enumerate() {
                            if let Ok(loaded) = compiler::load_and_orient_image(&p) {
                                let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                                let base_thumb = loaded.thumbnail(160, 160);
                                items.push(ImageEditorItem {
                                    id: idx,
                                    initial_index: 0,
                                    file_name: fname,
                                    current_path: p,
                                    rotation: 0,
                                    base_thumbnail: base_thumb,
                                });
                            }
                        }

                        if !items.is_empty() {
                            let initial = items.clone();
                            groups.push(GroupState {
                                name: folder_name,
                                folder_path: path,
                                items,
                                initial_items: initial,
                            });
                        }
                    }
                }

                if groups.is_empty() {
                    let err_msg = format!(
                        "No subdirectories with supported images found in: {}. Multi-directory mode requires subfolders with images.",
                        input_str
                    );
                    let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_error_message(err_msg.into());
                        ui.set_current_page(AppState::Error);
                    });
                    return;
                }
            } else {
                let mut img_paths: Vec<_> = match std::fs::read_dir(&input_path) {
                    Ok(dir) => dir
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| compiler::is_image_file(p))
                        .collect(),
                    Err(err) => {
                        let err_msg = format!("Failed to read directory: {}", err);
                        let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_error_message(err_msg.into());
                            ui.set_current_page(AppState::Error);
                        });
                        return;
                    }
                };

                if img_paths.is_empty() {
                    let err_msg = format!("No supported images (.jpg, .jpeg, .png, .webp) found in: {}", input_str);
                    let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_error_message(err_msg.into());
                        ui.set_current_page(AppState::Error);
                    });
                    return;
                }

                img_paths.sort();
                let total_imgs = img_paths.len();

                let mut items = Vec::new();
                for (idx, p) in img_paths.into_iter().enumerate() {
                    if idx % 5 == 0 || idx == total_imgs - 1 {
                        let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_loading_text(format!("Loading images ({} of {})...", idx + 1, total_imgs).into());
                        });
                    }

                    if let Ok(loaded) = compiler::load_and_orient_image(&p) {
                        let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let base_thumb = loaded.thumbnail(160, 160);
                        items.push(ImageEditorItem {
                            id: idx,
                            initial_index: idx,
                            file_name: fname,
                            current_path: p,
                            rotation: 0,
                            base_thumbnail: base_thumb,
                        });
                    }
                }

                let folder_name = input_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let initial = items.clone();
                groups.push(GroupState {
                    name: folder_name,
                    folder_path: input_path,
                    items,
                    initial_items: initial,
                });
            }

            let new_state = EditorState {
                groups,
                active_group: 0,
                columns,
                gaps,
                tile_width: width,
                tile_height: height,
                output_path,
                is_multi,
            };

            let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                *thread_state.lock().unwrap() = Some(new_state);
                let guard = thread_state.lock().unwrap();
                if let Some(state) = guard.as_ref() {
                    sync_editor_ui(&ui, state);
                    update_selection(&ui, state, -1);
                }
                ui.set_persist_status("".into());
                ui.set_current_page(AppState::EditGrid);
            });
        });
    });

    // Select Image Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_select_image(move |index| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            update_selection(&ui, state, index);
        }
    });

    // Deselect Image Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_deselect(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_ref() {
            update_selection(&ui, state, -1);
        }
    });

    // Rotate Image Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_rotate_image(move |index, delta_deg| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            if let Some(item) = state.groups[active].items.get_mut(index as usize) {
                item.rotation = (item.rotation + delta_deg).rem_euclid(360);
            }
            sync_editor_ui(&ui, state);
            update_selection(&ui, state, index);
        }
    });

    // Rotate All Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_rotate_all(move |delta_deg| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            for item in &mut state.groups[active].items {
                item.rotation = (item.rotation + delta_deg).rem_euclid(360);
            }
            sync_editor_ui(&ui, state);
            let sel = ui.get_selected_index();
            update_selection(&ui, state, sel);
        }
    });

    // Move Image Callback (delta)
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_move_image(move |from_idx, delta| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            let total = state.groups[active].items.len() as i32;
            if from_idx < 0 || from_idx >= total {
                return;
            }
            let to_idx = (from_idx + delta).clamp(0, total - 1);
            if from_idx != to_idx {
                let item = state.groups[active].items.remove(from_idx as usize);
                state.groups[active].items.insert(to_idx as usize, item);
                sync_editor_ui(&ui, state);
                update_selection(&ui, state, to_idx);
            }
        }
    });

    // Move To Start Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_move_to_start(move |from_idx| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            let total = state.groups[active].items.len() as i32;
            if from_idx > 0 && from_idx < total {
                let item = state.groups[active].items.remove(from_idx as usize);
                state.groups[active].items.insert(0, item);
                sync_editor_ui(&ui, state);
                update_selection(&ui, state, 0);
            }
        }
    });

    // Move To End Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_move_to_end(move |from_idx| {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            let total = state.groups[active].items.len() as i32;
            if from_idx >= 0 && from_idx < total - 1 {
                let item = state.groups[active].items.remove(from_idx as usize);
                let last = state.groups[active].items.len();
                state.groups[active].items.insert(last, item);
                sync_editor_ui(&ui, state);
                update_selection(&ui, state, total - 1);
            }
        }
    });

    // Reset Images Order & Rotation Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_reset_images(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            state.groups[active].items = state.groups[active].initial_items.clone();
            sync_editor_ui(&ui, state);
            update_selection(&ui, state, -1);
            ui.set_persist_status("Reset to initial order.".into());
        }
    });

    // Previous Folder Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_prev_folder(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            if state.active_group > 0 {
                state.active_group -= 1;
                sync_editor_ui(&ui, state);
                update_selection(&ui, state, -1);
            }
        }
    });

    // Next Folder Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_next_folder(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            if state.active_group + 1 < state.groups.len() {
                state.active_group += 1;
                sync_editor_ui(&ui, state);
                update_selection(&ui, state, -1);
            }
        }
    });

    // Persist Changes Now Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_persist_changes_now(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let mut state_guard = state_clone.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            let active = state.active_group;
            let group = &mut state.groups[active];
            match compiler::persist_group_changes(&mut group.items, &group.folder_path) {
                Ok(count) => {
                    group.initial_items = group.items.clone();
                    sync_editor_ui(&ui, state);
                    ui.set_persist_status(
                        format!("✔ Saved changes to disk ({} items updated).", count).into(),
                    );
                }
                Err(err) => {
                    ui.set_persist_status(format!("⚠ Failed to save changes: {}", err).into());
                }
            }
        }
    });

    // Back to Form Callback
    let ui_weak = main_window.as_weak();
    main_window.on_back_to_form(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_page(AppState::Form);
        }
    });

    // Start Compiling Callback
    let ui_weak = main_window.as_weak();
    let state_clone = Arc::clone(&editor_state);
    main_window.on_start_compiling(move || {
        let Some(ui) = ui_weak.upgrade() else { return };
        let state_guard = state_clone.lock().unwrap();
        let Some(state) = state_guard.as_ref() else {
            return;
        };

        let persist_changes = ui.get_persist_changes();
        let is_multi = state.is_multi;
        let mut groups = state.groups.clone();
        let output_path = state.output_path.clone();
        let columns = state.columns;
        let gaps = state.gaps;
        let width = state.tile_width;
        let height = state.tile_height;
        drop(state_guard);

        ui.set_current_page(AppState::Compiling);

        let thread_ui_weak = ui_weak.clone();
        std::thread::spawn(move || {
            let dimensions = Dimensions { width, height };

            if persist_changes {
                for group in &mut groups {
                    if let Err(err) =
                        compiler::persist_group_changes(&mut group.items, &group.folder_path)
                    {
                        let err_msg = format!("Failed to persist changes to files: {}", err);
                        let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                            ui.set_error_message(err_msg.into());
                            ui.set_current_page(AppState::Error);
                        });
                        return;
                    }
                }
            }

            let result = if is_multi {
                if let Err(err) = std::fs::create_dir_all(&output_path) {
                    Err(anyhow::anyhow!(
                        "Failed to create output directory: {}",
                        err
                    ))
                } else {
                    let mut compile_err = None;
                    for group in &groups {
                        let out_file = output_path.join(format!("{}.jpg", group.name));
                        let processed: Vec<_> = group
                            .items
                            .iter()
                            .map(|i| ProcessedImage {
                                path: i.current_path.clone(),
                                rotation: i.rotation,
                            })
                            .collect();

                        if let Err(e) = compiler::compile_custom_images(
                            &processed,
                            &out_file,
                            Dimensions { width, height },
                            Columns(columns),
                            Gaps(gaps),
                        ) {
                            compile_err = Some(e);
                            break;
                        }
                    }
                    match compile_err {
                        Some(e) => Err(e),
                        None => Ok(()),
                    }
                }
            } else if let Some(group) = groups.first() {
                let processed: Vec<_> = group
                    .items
                    .iter()
                    .map(|i| ProcessedImage {
                        path: i.current_path.clone(),
                        rotation: i.rotation,
                    })
                    .collect();
                compiler::compile_custom_images(
                    &processed,
                    &output_path,
                    dimensions,
                    Columns(columns),
                    Gaps(gaps),
                )
            } else {
                Err(anyhow::anyhow!("No images to compile."))
            };

            match result {
                Ok(()) => {
                    let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_current_page(AppState::Done);
                    });
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    let _ = thread_ui_weak.upgrade_in_event_loop(move |ui| {
                        ui.set_error_message(err_msg.into());
                        ui.set_current_page(AppState::Error);
                    });
                }
            }
        });
    });

    // Reset / Back to Form Callback
    let ui_weak = main_window.as_weak();
    main_window.on_reset(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_current_page(AppState::Form);
        }
    });

    // Open Output Folder Callback
    let ui_weak = main_window.as_weak();
    main_window.on_open_output(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let output = ui.get_output_path().to_string();
            let is_multi = ui.get_is_multi_directory();
            open_in_file_manager(&output, is_multi);
        }
    });

    // Close Application Callback
    main_window.on_close_app(|| {
        let _ = slint::quit_event_loop();
    });

    main_window.run()
}

fn open_in_file_manager(path_str: &str, is_dir: bool) {
    if path_str.is_empty() {
        return;
    }

    let path = Path::new(path_str);
    let target = if is_dir || path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| path.to_path_buf())
    };

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&target).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&target).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&target).spawn();
    }
}
