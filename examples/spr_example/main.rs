//! SPR Sprite File Example
//!
//! This example demonstrates working with SPR sprite animation files.
//!
//! Features demonstrated:
//! - Loading SPR files
//! - Iterating over frames
//! - Accessing sprite and mask data separately
//! - Exporting frames as PGM images
//! - Creating new SPR files
//! - Encoding/decoding palette indices
//! - Modifying frame data

use log::info;
use std::path::{Path, PathBuf};

use dvine_internal::prelude::*;

fn main() {
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

	info!("=== SPR Sprite File Example ===\n");

	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = PathBuf::from(&cargo_root).join("bin");
	let spr_root = bin_root.join("spr_extract");

	// Example 1: Load and inspect an SPR file
	example_load_spr(&spr_root);

	// Example 2: Iterate over frames
	example_iterate_frames(&spr_root);

	// Example 3: Export frames
	example_export_frames(&spr_root);

	// Example 4: Create a new SPR file
	example_create_spr(&spr_root);

	// Example 5: Modify sprite data
	example_modify_sprite(&spr_root);

	// Example 6: Work with palettes
	example_palette_usage(&spr_root);

	info!("\n✓ SPR examples complete!");
}

/// Example 1: Load and inspect an SPR file
fn example_load_spr(spr_root: &Path) {
	info!("Example 1: Loading SPR file\n");

	let input_path = spr_root.join("AG");

	match SprFile::open(&input_path) {
		Ok(spr) => {
			info!("✓ Loaded: {}", input_path.display());
			info!("  Frame count: {}", spr.frame_count());
			info!("  File size: {} bytes", spr.to_bytes().len());

			// Show first frame info
			if let Some(frame) = spr.get_frame(0) {
				info!("\n  First frame details:");
				info!("    Dimensions: {}x{}", frame.width(), frame.height());
				info!("    Hotspot: ({}, {})", frame.hotspot_x(), frame.hotspot_y());
				info!("    Sprite data: {} bytes", frame.sprite_pixels().len());
				info!("    Mask data: {} bytes", frame.mask_pixels().len());

				// Show sample pixel values
				if let Some(raw_pixel) = frame.get_sprite_pixel(0, 0) {
					let decoded = SprFrame::decode_sprite_pixel(raw_pixel);
					info!(
						"    Sample pixel at (0,0): raw={}, decoded palette index={}",
						raw_pixel, decoded
					);
				}
			}
		}
		Err(e) => {
			info!("✗ Failed to load SPR file: {}", e);
			info!("  Expected location: {}", input_path.display());
		}
	}

	info!("");
}

/// Example 2: Iterate over all frames
fn example_iterate_frames(spr_root: &Path) {
	info!("Example 2: Iterating over frames\n");

	let input_path = spr_root.join("KATIA");

	match SprFile::open(&input_path) {
		Ok(spr) => {
			info!("✓ Loaded: {}", input_path.display());
			info!("  Iterating over {} frames:\n", spr.frame_count());

			for (index, frame) in spr.iter().enumerate().take(10) {
				info!("  Frame #{:3}: {}", index, frame);

				// Count opaque pixels in mask
				let opaque_count = frame.mask_pixels().iter().filter(|&&p| p >= 0x80).count();

				let total_pixels = frame.mask_pixels().len();
				let opacity_pct = (opaque_count as f32 / total_pixels as f32) * 100.0;

				info!(
					"    Opacity: {:.1}% ({}/{} pixels)",
					opacity_pct, opaque_count, total_pixels
				);
			}

			if spr.frame_count() > 10 {
				info!("  ... and {} more frames", spr.frame_count() - 10);
			}
		}
		Err(e) => {
			info!("✗ Failed to load SPR file: {}", e);
			info!("  Expected location: {}", input_path.display());
		}
	}

	info!("");
}

/// Example 3: Export frames as PGM images
fn example_export_frames(spr_root: &Path) {
	info!("Example 3: Exporting frames\n");

	let input_path = spr_root.join("AG");
	let output_dir = spr_root.join("spr_export");

	if let Err(e) = std::fs::create_dir_all(&output_dir) {
		info!("✗ Failed to create output directory: {}", e);
		return;
	}

	match SprFile::open(&input_path) {
		Ok(spr) => {
			info!("✓ Loaded: {}", input_path.display());
			info!("  Exporting frames to: {}\n", output_dir.display());

			let export_count = spr.frame_count().min(5);
			let mut success = 0;

			for i in 0..export_count as usize {
				if let Some(frame) = spr.get_frame(i) {
					// Export sprite
					let sprite_filename = format!("frame_{:03}_sprite.pgm", i);
					let sprite_path = output_dir.join(&sprite_filename);

					// Export mask
					let mask_filename = format!("frame_{:03}_mask.pgm", i);
					let mask_path = output_dir.join(&mask_filename);

					if std::fs::write(&sprite_path, frame.sprite_to_pgm()).is_ok()
						&& std::fs::write(&mask_path, frame.mask_to_pgm()).is_ok()
					{
						info!("  ✓ Frame #{}: {}x{}", i, frame.width(), frame.height());
						info!("      Sprite: {}", sprite_filename);
						info!("      Mask:   {}", mask_filename);
						success += 1;
					}
				}
			}

			info!("\n  ✓ Exported {} frames successfully", success);
		}
		Err(e) => {
			info!("✗ Failed to load SPR file: {}", e);
		}
	}

	info!("");
}

/// Example 4: Create a new SPR file from scratch
fn example_create_spr(spr_root: &Path) {
	info!("Example 4: Creating new SPR file\n");

	let mut spr = SprFile::new();

	// Create a simple 16x16 test frame with a cross pattern
	let width = 16u32;
	let height = 16u32;

	let mut sprite_pixels = vec![SprFrame::encode_sprite_pixel(0); (width * height) as usize];
	let mut mask_pixels = vec![0xFF; (width * height) as usize];

	// Draw a cross pattern
	for y in 0..height {
		for x in 0..width {
			let idx = (y * width + x) as usize;

			// Vertical and horizontal lines
			if x == width / 2 || y == height / 2 {
				sprite_pixels[idx] = SprFrame::encode_sprite_pixel(79); // Max palette index
				mask_pixels[idx] = 0xFF; // Opaque
			} else {
				sprite_pixels[idx] = SprFrame::encode_sprite_pixel(0);
				mask_pixels[idx] = 0x00; // Transparent
			}
		}
	}

	// Create frame entry and frame
	let entry = SprFrameEntry::new(0, 0, width, height, width / 2, height / 2);
	let frame = SprFrame::new(entry, sprite_pixels, mask_pixels);

	// Add frame to SPR file
	match spr.add_frame(frame) {
		Ok(_) => {
			info!("✓ Created frame: {}x{}", width, height);

			// Save the file
			let output_path = spr_root.join("test_sprite.spr");
			match spr.save(&output_path) {
				Ok(_) => {
					info!("✓ Saved new SPR file: {}", output_path.display());
					info!("  File size: {} bytes", spr.to_bytes().len());
				}
				Err(e) => {
					info!("✗ Failed to save SPR file: {}", e);
				}
			}
		}
		Err(e) => {
			info!("✗ Failed to add frame: {}", e);
		}
	}

	info!("");
}

/// Example 5: Modify sprite data
fn example_modify_sprite(spr_root: &Path) {
	info!("Example 5: Modifying sprite data\n");

	let input_path = spr_root.join("AG");
	let output_path = spr_root.join("AG_MODIFIED.SPR");

	match SprFile::open(&input_path) {
		Ok(mut spr) => {
			info!("✓ Loaded: {}", input_path.display());

			// Modify first frame - invert the mask
			if let Some(mut frame) = spr.get_frame(0) {
				info!("  Modifying frame 0: inverting mask...");

				// Invert mask pixels
				for mask_pixel in frame.mask_pixels_mut() {
					*mask_pixel = !*mask_pixel;
				}

				// Update frame back to file
				if spr.update_frame(0, frame.sprite_pixels(), frame.mask_pixels()) {
					info!("  ✓ Frame 0 updated");

					// Save modified file
					match spr.save(&output_path) {
						Ok(_) => {
							info!("✓ Saved modified SPR file: {}", output_path.display());
						}
						Err(e) => {
							info!("✗ Failed to save: {}", e);
						}
					}
				} else {
					info!("✗ Failed to update frame");
				}
			}
		}
		Err(e) => {
			info!("✗ Failed to load SPR file: {}", e);
		}
	}

	info!("");
}

/// Example 6: Work with palettes
fn example_palette_usage(spr_root: &Path) {
	info!("Example 6: Working with palettes\n");

	let palette_path = spr_root.join("SPR.PAL");
	let spr_path = spr_root.join("AG");

	// Load palette
	match SprPalette::from_file(&palette_path) {
		Ok(palette) => {
			info!("✓ Loaded palette: {}", palette_path.display());
			info!("  Colors defined: {}", SprPalette::SPR_PAL_COLOR_COUNT);

			// Show first few colors
			info!("\n  First 5 palette colors:");
			for i in 0..5 {
				let color = palette.get(i);
				info!("    Color {}: {}", i, color);
			}

			// Load SPR file and apply palette
			if let Ok(spr) = SprFile::open(&spr_path)
				&& let Some(frame) = spr.get_frame(0)
			{
				info!("\n  Applying palette to frame 0:");
				info!("    Frame size: {}x{}", frame.width(), frame.height());

				// Get RGB data
				let rgb_data = frame.apply_palette_rgb(&palette);
				info!("    RGB data size: {} bytes", rgb_data.len());

				// Get RGBA data with mask
				let rgba_data = frame.apply_palette_with_mask(&palette);
				info!("    RGBA data size: {} bytes", rgba_data.len());

				// Get color at specific position
				if let Some(color) = frame.get_color_at(0, 0, &palette) {
					info!("    Color at (0,0): {}", color);
				}

				// Iterate over color rows
				info!("\n  First row colors:");
				if let Some(first_row) = frame.color_rows(&palette).next() {
					for (x, color) in first_row.iter().enumerate().take(5) {
						info!("      Pixel {}: {}", x, color);
					}
				}
			}

			// Save palette back
			let output_palette_path = spr_root.join("test_palette.pal");
			if palette.save(&output_palette_path).is_ok() {
				info!("\n✓ Saved palette to: {}", output_palette_path.display());
			}
		}
		Err(e) => {
			info!("✗ Failed to load palette: {}", e);
			info!("  Expected location: {}", palette_path.display());
			info!("\n  Creating a grayscale palette instead...");

			let grayscale = SprPalette::grayscale();
			info!("✓ Created grayscale palette");

			// Save it
			let output_path = spr_root.join("grayscale.pal");
			if grayscale.save(&output_path).is_ok() {
				info!("✓ Saved grayscale palette: {}", output_path.display());
			}
		}
	}

	info!("");
}
