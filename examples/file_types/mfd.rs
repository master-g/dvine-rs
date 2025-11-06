//! MFD (Mouse File Data) support test
//!
//! This module provides functionality to test and export MFD mouse cursor animation files.
//!
//! # Features
//!
//! - Load and inspect MFD files (`DXMSTEST.MFD`)
//! - Export all frames to PGM (Portable `GrayMap`) images
//! - Export all frames to PNG images
//! - Display frame metadata (dimensions, hotspot offsets)
//! - Render frames with ASCII art in console
//!
//! # Frame Export Format
//!
//! Frames are exported in two formats:
//!
//! ## `PGM` (Portable `GrayMap`)
//! - Transparent pixels (0) -> White (255)
//! - Outline pixels (1) -> Gray (128)
//! - Fill pixels (other values) -> Black (0)
//!
//! ## PNG
//! - Transparent pixels (0) -> White (RGB: 255, 255, 255)
//! - Outline pixels (1) -> Gray (RGB: 128, 128, 128)
//! - Fill pixels (other values) -> Black (RGB: 0, 0, 0)

use dvine_internal::prelude::*;
use image::{ImageBuffer, Rgb, RgbImage};
use log::info;

pub(super) fn test_mfd() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	let mfd_path = bin_root.join("DXMSTEST.MFD");

	info!("Loading MFD file...");

	let mfd = match MfdFile::open(&mfd_path) {
		Ok(file) => {
			info!("✓ Loaded MFD file: {}", mfd_path.display());
			info!("  Total frames: {}", file.frame_count());
			info!("  File size: {} bytes", file.to_bytes().len());
			file
		}
		Err(e) => {
			info!("✗ Failed to load MFD file: {}", e);
			info!("  Expected location: {}", mfd_path.display());
			info!("  Note: Place DXMSTEST.MFD in the bin/ directory to test");
			return;
		}
	};

	// Display frame information
	display_frame_info(&mfd);

	// Export frames to different formats
	export_frames_pgm(&mfd);
	export_frames_png(&mfd);

	// Display ASCII art for first few frames
	display_ascii_art(&mfd, 3);

	// Create a contact sheet of all frames
	create_contact_sheet(&mfd, "mfd_contact_sheet.png");

	info!("\n✓ MFD test complete");
}

/// Displays detailed information about all frames in the MFD file.
fn display_frame_info(mfd: &MfdFile) {
	info!("\nFrame Information:");
	info!("  {:-<80}", "");

	for (index, entry) in mfd.entries().iter().enumerate() {
		info!(
			"  Frame #{:2}: {}x{} px, hotspot=({:3},{:3}), offset=0x{:08X}",
			index, entry.width, entry.height, entry.x_offset, entry.y_offset, entry.bitmap_offset
		);
	}

	info!("  {:-<80}", "");
}

/// Exports all frames to `PGM` (Portable `GrayMap`) format.
///
/// PGM is a simple grayscale image format that can be opened by most image viewers.
fn export_frames_pgm(mfd: &MfdFile) {
	info!("\nExporting frames to PGM format...");

	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let output_dir = bin_root.join("mfd_frames_pgm");

	// Create output directory
	if let Err(e) = std::fs::create_dir_all(&output_dir) {
		info!("  ✗ Failed to create output directory: {}", e);
		return;
	}

	let mut success_count = 0;
	let mut fail_count = 0;

	for (index, frame) in mfd.iter().enumerate() {
		let filename = format!("frame_{:02}.pgm", index);
		let output_path = output_dir.join(&filename);

		let pgm_data = frame.to_pgm();

		match std::fs::write(&output_path, pgm_data) {
			Ok(_) => {
				success_count += 1;
			}
			Err(e) => {
				info!("  ✗ Failed to export {}: {}", filename, e);
				fail_count += 1;
			}
		}
	}

	info!("  ✓ Exported {} frames to: {}", success_count, output_dir.display());
	if fail_count > 0 {
		info!("  ⚠ Failed to export {} frames", fail_count);
	}
}

/// Exports all frames to PNG format.
fn export_frames_png(mfd: &MfdFile) {
	info!("\nExporting frames to PNG format...");

	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let output_dir = bin_root.join("mfd_frames_png");

	// Create output directory
	if let Err(e) = std::fs::create_dir_all(&output_dir) {
		info!("  ✗ Failed to create output directory: {}", e);
		return;
	}

	let mut success_count = 0;
	let mut fail_count = 0;

	for (index, frame) in mfd.iter().enumerate() {
		let filename = format!("frame_{:02}.png", index);
		let output_path = output_dir.join(&filename);

		// Create image buffer
		let width = frame.width() as u32;
		let height = frame.height() as u32;
		let mut img: RgbImage = ImageBuffer::new(width, height);

		// Define colors
		let white = Rgb([255, 255, 255]);
		let gray = Rgb([128, 128, 128]);
		let black = Rgb([0, 0, 0]);

		// Fill image with pixel data
		for (y, row) in frame.rows().enumerate() {
			for (x, &pixel) in row.iter().enumerate() {
				let color = match pixel {
					0 => white, // transparent
					1 => gray,  // outline
					_ => black, // fill
				};
				img.put_pixel(x as u32, y as u32, color);
			}
		}

		match img.save(&output_path) {
			Ok(_) => {
				success_count += 1;
			}
			Err(e) => {
				info!("  ✗ Failed to export {}: {}", filename, e);
				fail_count += 1;
			}
		}
	}

	info!("  ✓ Exported {} frames to: {}", success_count, output_dir.display());
	if fail_count > 0 {
		info!("  ⚠ Failed to export {} frames", fail_count);
	}
}

/// Displays ASCII art representation of the first N frames.
fn display_ascii_art(mfd: &MfdFile, count: usize) {
	info!("\nASCII Art Preview (first {} frames):", count);

	for (index, frame) in mfd.iter().take(count).enumerate() {
		info!("\n  Frame #{} ({}x{}):", index, frame.width(), frame.height());
		info!("  {:-<60}", "");

		let art = frame.to_ascii_art_default();
		for line in art.lines() {
			info!("  {}", line);
		}

		info!("  {:-<60}", "");
	}
}

/// Creates a contact sheet showing all frames in a grid layout.
///
/// # Arguments
/// * `mfd` - MFD file containing frames
/// * `output_filename` - Output filename (will be saved to bin/ directory)
fn create_contact_sheet(mfd: &MfdFile, output_filename: &str) {
	info!("\nCreating contact sheet: {}", output_filename);

	if mfd.frame_count() == 0 {
		info!("  ⚠ No frames to display");
		return;
	}

	// Calculate grid dimensions
	let frame_count = mfd.frame_count() as usize;
	let grid_cols = (frame_count as f64).sqrt().ceil() as usize;
	let grid_rows = frame_count.div_ceil(grid_cols);

	info!("  Grid layout: {}x{} cells", grid_cols, grid_rows);

	// Find maximum frame dimensions for uniform cell size
	let mut max_width = 0u16;
	let mut max_height = 0u16;

	for entry in mfd.entries() {
		max_width = max_width.max(entry.width);
		max_height = max_height.max(entry.height);
	}

	info!("  Max frame size: {}x{} pixels", max_width, max_height);

	// Calculate cell and image dimensions
	let cell_width = max_width as u32 + 4; // 2px padding on each side
	let cell_height = max_height as u32 + 4;
	let separator = 2u32;

	let img_width = grid_cols as u32 * cell_width + (grid_cols as u32 + 1) * separator;
	let img_height = grid_rows as u32 * cell_height + (grid_rows as u32 + 1) * separator;

	info!("  Image dimensions: {}x{} pixels", img_width, img_height);

	// Create image with light gray background
	let bg_color = Rgb([240, 240, 240]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, bg_color);

	// Define colors
	let white = Rgb([255, 255, 255]);
	let gray = Rgb([128, 128, 128]);
	let black = Rgb([0, 0, 0]);
	let separator_color = Rgb([200, 200, 200]);

	// Draw separators
	for col in 0..=grid_cols {
		let x = col as u32 * (cell_width + separator);
		for dy in 0..separator {
			for y in 0..img_height {
				if x + dy < img_width {
					img.put_pixel(x + dy, y, separator_color);
				}
			}
		}
	}

	for row in 0..=grid_rows {
		let y = row as u32 * (cell_height + separator);
		for dy in 0..separator {
			for x in 0..img_width {
				if y + dy < img_height {
					img.put_pixel(x, y + dy, separator_color);
				}
			}
		}
	}

	// Draw each frame
	for (index, frame) in mfd.iter().enumerate() {
		let grid_row = index / grid_cols;
		let grid_col = index % grid_cols;

		// Calculate cell position
		let cell_x = grid_col as u32 * (cell_width + separator) + separator;
		let cell_y = grid_row as u32 * (cell_height + separator) + separator;

		// Center frame within cell
		let frame_x = cell_x + (cell_width - frame.width() as u32) / 2;
		let frame_y = cell_y + (cell_height - frame.height() as u32) / 2;

		// Draw frame pixels
		for (y, row) in frame.rows().enumerate() {
			for (x, &pixel) in row.iter().enumerate() {
				let color = match pixel {
					0 => white, // transparent
					1 => gray,  // outline
					_ => black, // fill
				};

				let px = frame_x + x as u32;
				let py = frame_y + y as u32;

				if px < img_width && py < img_height {
					img.put_pixel(px, py, color);
				}
			}
		}
	}

	// Save the image to bin/ directory
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let output_path = bin_root.join(output_filename);

	match img.save(&output_path) {
		Ok(_) => info!("  ✓ Contact sheet saved: {}", output_path.display()),
		Err(e) => info!("  ✗ Failed to save contact sheet: {}", e),
	}
}
