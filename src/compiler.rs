use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Context;
use crabgrid::canvas::{Canvas, Columns, Dimensions, Gaps, ImageCount};
use image::DynamicImage;

#[derive(Clone, Debug)]
pub struct ProcessedImage {
    pub path: PathBuf,
    pub rotation: i32,
}

#[derive(Clone)]
pub struct ImageEditorItem {
    pub id: usize,
    pub initial_index: usize,
    pub file_name: String,
    pub current_path: PathBuf,
    pub rotation: i32,
    pub base_thumbnail: DynamicImage,
}

impl ImageEditorItem {
    pub fn current_thumbnail(&self) -> slint::Image {
        let rotated = apply_rotation(self.base_thumbnail.clone(), self.rotation);
        let rgb = rotated.to_rgb8();
        let (width, height) = (rgb.width(), rgb.height());
        let raw = rgb.into_raw();
        let buffer =
            slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(&raw, width, height);
        slint::Image::from_rgb8(buffer)
    }
}

pub fn load_and_orient_image(path: &Path) -> anyhow::Result<DynamicImage> {
    let image = image::open(path)
        .with_context(|| format!("Failed to open image file: {}", path.display()))?;

    if let Ok(file) = File::open(path) {
        let mut buf = BufReader::new(file);
        if let Ok(exif_data) = exif::Reader::new().read_from_container(&mut buf) {
            if let Some(field) = exif_data.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
                if let Some(orientation) = field.value.get_uint(0) {
                    let oriented = match orientation {
                        2 => image.fliph(),
                        3 => image.rotate180(),
                        4 => image.flipv(),
                        5 => image.rotate90().fliph(),
                        6 => image.rotate90(),
                        7 => image.rotate270().fliph(),
                        8 => image.rotate270(),
                        _ => image,
                    };
                    return Ok(oriented);
                }
            }
        }
    }

    Ok(image)
}

pub fn apply_rotation(img: DynamicImage, degrees: i32) -> DynamicImage {
    let deg = degrees.rem_euclid(360);
    match deg {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    }
}

pub fn compile_custom_images(
    images: &[ProcessedImage],
    output: &Path,
    dimensions: Dimensions,
    columns: Columns,
    gaps: Gaps,
) -> anyhow::Result<()> {
    if images.is_empty() {
        anyhow::bail!("No images provided for collage compilation.");
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create output directory: {}", parent.display())
            })?;
        }
    }

    let mut canvas = Canvas::new(columns, dimensions, ImageCount(images.len() as u32), gaps);

    for img_info in images {
        let loaded = load_and_orient_image(&img_info.path)?;
        let rotated = apply_rotation(loaded, img_info.rotation);
        canvas.append_image(&rotated);
    }

    canvas
        .write_to_file(output)
        .with_context(|| format!("Failed to save collage image to: {}", output.display()))?;

    Ok(())
}

pub fn strip_existing_numeric_prefix(name: &str) -> &str {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && (bytes[i] == b'_' || bytes[i] == b'-' || bytes[i] == b' ') {
        &name[i + 1..]
    } else {
        name
    }
}

pub fn persist_group_changes(
    items: &mut [ImageEditorItem],
    directory: &Path,
) -> anyhow::Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let mut modified_count = 0;

    // Phase 1: Persist rotations to original image files
    for item in items.iter_mut() {
        if item.rotation % 360 != 0 {
            let loaded = load_and_orient_image(&item.current_path)?;
            let rotated = apply_rotation(loaded, item.rotation);
            rotated.save(&item.current_path).with_context(|| {
                format!(
                    "Failed to save rotated image to {}",
                    item.current_path.display()
                )
            })?;

            item.base_thumbnail = rotated.thumbnail(160, 160);
            item.rotation = 0;
            modified_count += 1;
        }
    }

    // Phase 2: Persist reordering by renaming files with ordered numeric prefixes
    let count = items.len();
    let pad_width = if count >= 100 { 3 } else { 2 };

    let temp_tag = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut temp_paths = Vec::with_capacity(count);
    for (i, item) in items.iter().enumerate() {
        let temp_name = format!(".crabgrid_tmp_{}_{}_{}", temp_tag, i, item.file_name);
        let temp_path = directory.join(&temp_name);
        std::fs::rename(&item.current_path, &temp_path)
            .with_context(|| format!("Failed to create temporary file for {}", item.file_name))?;
        temp_paths.push(temp_path);
    }

    for (i, (item, temp_path)) in items.iter_mut().zip(temp_paths).enumerate() {
        let clean_name = strip_existing_numeric_prefix(&item.file_name);
        let new_name = format!("{:0width$}_{}", i + 1, clean_name, width = pad_width);
        let final_path = directory.join(&new_name);

        std::fs::rename(&temp_path, &final_path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                final_path.display()
            )
        })?;

        if item.current_path != final_path {
            modified_count += 1;
        }
        item.current_path = final_path;
        item.file_name = new_name;
    }

    Ok(modified_count)
}

pub fn compile_groups(
    main_directory: &Path,
    output: &Path,
    dimensions: Dimensions,
    columns: Columns,
    gaps: Gaps,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    let mut found_subdir = false;
    for entry in main_directory
        .read_dir()
        .with_context(|| format!("Failed to read directory: {}", main_directory.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            found_subdir = true;
            let dir_path = entry.path();
            let dir_name = entry.file_name().into_string().unwrap_or_default();
            let output_path = output.join(format!("{}.jpg", dir_name));

            compile_images(
                &dir_path,
                &output_path,
                Dimensions {
                    width: dimensions.width,
                    height: dimensions.height,
                },
                Columns(columns.0),
                Gaps(gaps.0),
            )?;
        }
    }

    if !found_subdir {
        anyhow::bail!(
            "No subdirectories found in: {}. Multi-directory mode requires subfolders with images.",
            main_directory.display()
        );
    }

    Ok(())
}

pub fn compile_images(
    input: &Path,
    output: &Path,
    dimensions: Dimensions,
    columns: Columns,
    gaps: Gaps,
) -> anyhow::Result<()> {
    let mut image_list: Vec<_> = std::fs::read_dir(input)
        .with_context(|| format!("Failed to read input directory: {}", input.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_image_file(path))
        .collect();

    if image_list.is_empty() {
        anyhow::bail!(
            "No supported images (.jpg, .jpeg, .png, .webp) found in: {}",
            input.display()
        );
    }

    image_list.sort();

    let processed: Vec<_> = image_list
        .into_iter()
        .map(|path| ProcessedImage { path, rotation: 0 })
        .collect();

    compile_custom_images(&processed, output, dimensions, columns, gaps)
}

pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext_str| {
            let lower = ext_str.to_lowercase();
            matches!(lower.as_str(), "jpg" | "jpeg" | "png" | "webp")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("photo.jpg")));
        assert!(is_image_file(Path::new("photo.JPG")));
        assert!(is_image_file(Path::new("photo.jpeg")));
        assert!(is_image_file(Path::new("photo.png")));
        assert!(is_image_file(Path::new("photo.webp")));
        assert!(!is_image_file(Path::new("photo.txt")));
        assert!(!is_image_file(Path::new("photo.pdf")));
        assert!(!is_image_file(Path::new("photo")));
    }

    #[test]
    fn test_strip_existing_numeric_prefix() {
        assert_eq!(strip_existing_numeric_prefix("01_photo.jpg"), "photo.jpg");
        assert_eq!(strip_existing_numeric_prefix("005_photo.jpg"), "photo.jpg");
        assert_eq!(strip_existing_numeric_prefix("12-test.png"), "test.png");
        assert_eq!(strip_existing_numeric_prefix("3 image.webp"), "image.webp");
        assert_eq!(strip_existing_numeric_prefix("photo.jpg"), "photo.jpg");
    }
}
