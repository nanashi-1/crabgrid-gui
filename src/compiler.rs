use std::path::Path;

use anyhow::Context;
use crabgrid::canvas::{Canvas, Columns, Dimensions, Gaps, ImageCount};

pub fn compile_groups(
    main_directory: &Path,
    output: &Path,
    dimensions: Dimensions,
    columns: Columns,
    gaps: Gaps,
) -> anyhow::Result<()> {
    // Iterate through all subdirectories in the main directory.
    for entry in main_directory.read_dir()? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_path = entry.path();
            let dir_name = entry.file_name().into_string().unwrap_or_default();
            let output_path = Path::new(output).join(format!("{}.jpg", dir_name));

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

    Ok(())
}

pub fn compile_images(
    input: &Path,
    output: &Path,
    dimensions: Dimensions,
    columns: Columns,
    gaps: Gaps,
) -> anyhow::Result<()> {
    // Collect all supported image files in the directory and sort them alphabetically.
    let mut image_list: Vec<_> = std::fs::read_dir(input)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_image_file(path))
        .collect();

    image_list.sort();

    let mut canvas = Canvas::new(
        columns,
        dimensions,
        ImageCount(image_list.len() as u32),
        gaps,
    );

    // Append each image to the canvas grid sequentially.
    for image_path in image_list {
        let image = image::open(image_path).context("Failed to open image file")?;
        canvas.append_image(&image);
    }

    // Write the compiled grid collage to the filesystem.
    canvas
        .write_to_file(output)
        .context("Failed to save the collage image")?;

    Ok(())
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext_str| {
            let lower = ext_str.to_lowercase();
            matches!(lower.as_str(), "jpg" | "jpeg" | "png" | "webp")
        })
        .unwrap_or(false)
}
