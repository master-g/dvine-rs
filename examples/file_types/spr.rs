//! SPR (Sprite) support test
//!
//! This module provides functionality to test and export SPR sprite animation files.
//!
//! # Features
//!
//! - Process all SPR files in `bin/spr_extract/`
//! - Export individual frames (sprite + mask) to PNG
//! - Generate sprite sheets for all frames
//! - Generate separate mask sheets
//! - Filter out empty/invalid frames automatically
//!
//! # Output Structure
//!
//! ```text
//! bin/spr_extract/
//!   ├── ADORIA/
//!   │   ├── frames/
//!   │   │   ├── frame_001_sprite.png
//!   │   │   ├── frame_001_mask.png
//!   │   │   └── ...
//!   │   ├── spritesheet.png
//!   │   └── masksheet.png
//!   ├── AG/
//!   └── ...
//! ```

use dvine_internal::prelude::*;
use image::{ImageBuffer, RgbImage, Rgba, RgbaImage};
use log::info;
use std::path::PathBuf;

pub(super) fn test() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let spr_root = bin_root.join("spr_extract");
	let output_root = bin_root.join("spr_parse");

	// Load palette first
	let palette_path = bin_root.join("SPR.PAL");
	let palette = match SprPalette::from_file(&palette_path) {
		Ok(pal) => {
			info!("✓ Loaded SPR palette: {}", palette_path.display());
			info!("  Total colors: {}", pal.colors().len());
			pal
		}
		Err(e) => {
			info!("✗ Failed to load SPR palette: {}", e);
			info!("  Expected location: {}", palette_path.display());
			return;
		}
	};

	// Check if spr_extract directory exists
	if !spr_root.exists() {
		info!("✗ Directory not found: {}", spr_root.display());
		info!("  Please create bin/spr_extract/ and place SPR files there");
		return;
	}

	// Find all SPR files (files without extension in spr_extract)
	let spr_files = match find_spr_files(&spr_root) {
		Ok(files) => files,
		Err(e) => {
			info!("✗ Failed to read directory: {}", e);
			return;
		}
	};

	if spr_files.is_empty() {
		info!("✗ No SPR files found in: {}", spr_root.display());
		info!("  Place SPR files (without extension) in bin/spr_extract/");
		return;
	}

	info!("\n📂 Found {} SPR files", spr_files.len());

	let mut success_count = 0;
	let mut fail_count = 0;

	// Process each SPR file
	for spr_path in spr_files {
		match process_spr_file(&spr_path, &palette, &output_root) {
			Ok(_) => success_count += 1,
			Err(e) => {
				info!("✗ Failed to process {}: {}", spr_path.display(), e);
				fail_count += 1;
			}
		}
	}

	info!("\n✅ Processing complete: {} succeeded, {} failed", success_count, fail_count);
}

/// Find all SPR files (files without extension) in the given directory
fn find_spr_files(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
	let mut spr_files = Vec::new();

	for entry in std::fs::read_dir(dir)? {
		let entry = entry?;
		let path = entry.path();

		// Only process files (not directories)
		if !path.is_file() {
			continue;
		}

		// Only process files without extension
		if path.extension().is_none() {
			spr_files.push(path);
		}
	}

	// Sort by filename for consistent ordering
	spr_files.sort();

	Ok(spr_files)
}

/// Process a single SPR file
fn process_spr_file(
	spr_path: &std::path::Path,
	palette: &SprPalette,
	output_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
	let filename = spr_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

	info!("\n📄 Processing: {}", filename);

	// Load SPR file
	let spr = SprFile::open(spr_path)?;

	info!("  Total frames: {}", spr.frame_count());

	// Count valid frames
	let valid_frames: Vec<_> = spr.iter().enumerate().filter(|(_, f)| f.is_valid()).collect();
	let valid_count = valid_frames.len();

	info!("  Valid frames: {}", valid_count);

	if valid_count == 0 {
		info!("  ⚠ No valid frames to export");
		return Ok(());
	}

	// Create output directory: bin/spr_extract/[SPR_NAME]/
	let output_dir = output_path.join(filename);

	// Remove existing path (file or directory) if it exists
	if output_dir.exists() {
		if output_dir.is_dir() {
			std::fs::remove_dir_all(&output_dir)?;
		} else {
			std::fs::remove_file(&output_dir)?;
		}
	}

	std::fs::create_dir_all(&output_dir)?;

	// Create frames subdirectory
	let frames_dir = output_dir.join("frames");
	std::fs::create_dir_all(&frames_dir)?;

	// Export individual frames
	export_individual_frames(&valid_frames, palette, &frames_dir)?;

	// Create sprite sheet and mask sheet
	create_sprite_sheets(&valid_frames, palette, &output_dir, filename)?;

	info!("  ✓ Exported to: {}", output_dir.display());

	Ok(())
}

/// Export individual sprite and mask frames
fn export_individual_frames(
	valid_frames: &[(usize, SprFrame)],
	palette: &SprPalette,
	output_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
	info!("  Exporting individual frames...");

	for (index, frame) in valid_frames {
		let frame_num = index + 1;

		// Export sprite (with palette colors)
		let sprite_filename = format!("frame_{:03}_sprite.png", frame_num);
		let sprite_path = output_dir.join(&sprite_filename);

		let sprite_rgb = frame.apply_palette_rgb(palette);
		let sprite_img: RgbImage = ImageBuffer::from_raw(frame.width(), frame.height(), sprite_rgb)
			.ok_or("Failed to create sprite image")?;

		sprite_img.save(&sprite_path)?;

		// Export mask (grayscale)
		let mask_filename = format!("frame_{:03}_mask.png", frame_num);
		let mask_path = output_dir.join(&mask_filename);

		let mask_data = frame.mask_pixels();
		let mask_img: image::GrayImage =
			ImageBuffer::from_raw(frame.width(), frame.height(), mask_data.to_vec())
				.ok_or("Failed to create mask image")?;

		mask_img.save(&mask_path)?;
	}

	info!("    ✓ Exported {} frame pairs", valid_frames.len());

	Ok(())
}

/// Create sprite sheet and mask sheet
fn create_sprite_sheets(
	valid_frames: &[(usize, SprFrame)],
	palette: &SprPalette,
	output_dir: &std::path::Path,
	_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	info!("  Creating sprite sheets...");

	if valid_frames.is_empty() {
		return Ok(());
	}

	// Find maximum frame dimensions
	let mut max_width = 0u32;
	let mut max_height = 0u32;

	for (_, frame) in valid_frames {
		max_width = max_width.max(frame.width());
		max_height = max_height.max(frame.height());
	}

	// Calculate grid layout
	let frame_count = valid_frames.len();
	let grid_cols = (frame_count as f64).sqrt().ceil() as usize;
	let grid_rows = frame_count.div_ceil(grid_cols);

	info!(
		"    Grid: {}x{} cells, cell size: {}x{} px",
		grid_cols, grid_rows, max_width, max_height
	);

	let padding = 2u32;
	let cell_width = max_width + padding * 2;
	let cell_height = max_height + padding * 2;

	let sheet_width = grid_cols as u32 * cell_width;
	let sheet_height = grid_rows as u32 * cell_height;

	// Create sprite sheet (RGB with checkerboard background)
	let mut sprite_sheet: RgbaImage = ImageBuffer::new(sheet_width, sheet_height);

	// Fill with checkerboard pattern
	for y in 0..sheet_height {
		for x in 0..sheet_width {
			let checker = ((x / 8) + (y / 8)) % 2 == 0;
			let gray = if checker {
				200
			} else {
				220
			};
			sprite_sheet.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
		}
	}

	// Create mask sheet (grayscale)
	let mut mask_sheet: image::GrayImage = ImageBuffer::from_pixel(
		sheet_width,
		sheet_height,
		image::Luma([0u8]), // Black background
	);

	// Place each frame in the grid
	for (grid_idx, (_, frame)) in valid_frames.iter().enumerate() {
		let grid_row = grid_idx / grid_cols;
		let grid_col = grid_idx % grid_cols;

		let cell_x = grid_col as u32 * cell_width;
		let cell_y = grid_row as u32 * cell_height;

		// Center frame in cell
		let frame_x = cell_x + (cell_width - frame.width()) / 2;
		let frame_y = cell_y + (cell_height - frame.height()) / 2;

		// Get sprite data (RGBA with mask)
		let rgba_data = frame.apply_palette_with_mask(palette);

		// Copy sprite pixels
		for y in 0..frame.height() {
			for x in 0..frame.width() {
				let pixel_idx = ((y * frame.width() + x) * 4) as usize;
				let r = rgba_data[pixel_idx];
				let g = rgba_data[pixel_idx + 1];
				let b = rgba_data[pixel_idx + 2];
				let a = rgba_data[pixel_idx + 3];

				let px = frame_x + x;
				let py = frame_y + y;

				if px < sheet_width && py < sheet_height {
					// Sprite sheet: output color directly if alpha is 0, since SPR mask uses 0 for opaque
					if a != 0 {
						sprite_sheet.put_pixel(px, py, Rgba([r, g, b, 255]));
					}
				}
			}
		}

		// Copy mask pixels
		let mask_data = frame.mask_pixels();
		for y in 0..frame.height() {
			for x in 0..frame.width() {
				let pixel_idx = (y * frame.width() + x) as usize;
				let mask_value = mask_data[pixel_idx];

				let px = frame_x + x;
				let py = frame_y + y;

				if px < sheet_width && py < sheet_height {
					mask_sheet.put_pixel(px, py, image::Luma([mask_value]));
				}
			}
		}
	}

	// Save sprite sheet
	let sprite_sheet_path = output_dir.join("spritesheet.png");
	sprite_sheet.save(&sprite_sheet_path)?;
	info!("    ✓ Sprite sheet: spritesheet.png ({}x{})", sheet_width, sheet_height);

	// Save mask sheet
	let mask_sheet_path = output_dir.join("masksheet.png");
	mask_sheet.save(&mask_sheet_path)?;
	info!("    ✓ Mask sheet: masksheet.png ({}x{})", sheet_width, sheet_height);

	Ok(())
}
