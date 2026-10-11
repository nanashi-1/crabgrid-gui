# crabgrid-gui

A modern, user-friendly graphical interface (GUI) for the [`crabgrid`](https://crates.io/crates/crabgrid) library, built in Rust using [Slint](https://slint.dev/) with Fluent Design style. 

It allows you to easily stitch a collection of individual images into a single grid collage (sprite sheet/contact sheet) with customizable grid sizes, dimensions, and gaps.

## Features

- **Fluent Design Interface**: Modern, native-feeling UI with clear cards, typography, and controls.
- **Adjustable Parameters**:
  - **Columns**: Customize the number of grid columns.
  - **Gap**: Configure the space (in pixels) between images.
  - **Per-Image Dimensions**: Set custom width and height (in pixels) for the individual grid elements.
- **Two Processing Modes**:
  - **Standard Mode (Single Directory)**: Select an input folder containing images (`.jpg`, `.jpeg`, `.png`, `.webp`) and output a single compiled collage image.
  - **Multi-directory Mode**: Select a parent folder containing multiple subfolders. The app will compile each subfolder's images into its own collage and save them all in the specified output directory.
- **Asynchronous Execution**: The compilation runs in the background on a separate thread, keeping the GUI responsive during rendering.
- **Seamless Explorer Integration**: Conveniently open output destinations directly from the completion screen.

## Getting Started

### Prerequisites

To build and run this application, you must have Rust and Cargo installed. If you don't have them, you can install them via [rustup](https://rustup.rs/).

#### Linux Dependencies
On Ubuntu/Debian, you can install GUI development packages using:

```bash
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libfontconfig1-dev libssl-dev
```

### Installation

Clone this repository and run the application using Cargo:

```bash
# Clone the repository
git clone https://github.com/nanashi-1/crabgrid-gui.git
cd crabgrid-gui

cargo install --path .
```

### Windows

The Windows installer is in the releases page.

## How to Use

1. **Set Input Directory**: Click **Browse** next to **Input** to select the directory containing your source images.
2. **Set Output Path**:
   - In standard mode, click **Browse** next to **Output** to choose the filename and path of the output collage (e.g., `collage.jpg`).
   - In **Multi-directory** mode, check the **Multi-directory** checkbox, then browse to select a target output folder where all generated collages will be saved.
3. **Configure Grid**:
   - Set the number of **Columns** for the layout.
   - Adjust the **Gap** between the images.
   - Set the **Per image width** and **Per image height** to define how large each individual image should scale within the grid.
4. **Compile**: Click the **Compile** button at the bottom. A loader will appear while processing.
5. **Finish**: Once completed, click **Done** to close the application or **Back** to run another collage.

## License

This project is licensed under the [GPL-3.0 License](LICENSE).
