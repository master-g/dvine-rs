//! MFD Frame Modifier Example (Improved Version)
//!
//! This example demonstrates the improved ergonomic API for modifying MFD files.
//!
//! # Improvements over the original version:
//!
//! 1. **Direct frame updates**: Use `update_frame()` to modify frames in place
//! 2. **Convenient save method**: Use `save()` instead of manual file writing
//! 3. **Pattern helpers**: Use `map_pixels_with_coords()`, `fill()`, `fill_rect()` for easier pattern creation
//! 4. **Functional style**: Use `map()` for creating transformed copies
//!
//! The modifications include various patterns:
//! - Checkerboard pattern (using `map_pixels_with_coords`)
//! - Border pattern (using `fill_rect`)
//! - Diagonal stripes (functional approach)
//! - Concentric circles (coordinate-based mapping)
//! - Spiral pattern (mathematical transformation)
//! - Cross pattern (combining `fill_rect`)
//! - Diamond pattern (distance-based)
//! - Gradient pattern (coordinate-based)
//! - Random dots (mapping with PRNG)
//! - Inverted original (simple `map`)

use dvine_internal::prelude::*;
use log::info;
use std::path::PathBuf;

fn main() {
	env_logger::init();

	info!("=== MFD Frame Modifier Example (Improved API) ===\n");

	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = PathBuf::from(&cargo_root).join("bin");
	let input_path = bin_root.join("DXMSTEST.MFD");
	let output_path = bin_root.join("DXMSTEST_IMPROVED.MFD");

	// Load the MFD file
	info!("Loading MFD file: {}", input_path.display());
	let mut mfd = match MfdFile::open(&input_path) {
		Ok(file) => {
			info!("✓ Loaded successfully");
			info!("  Total frames: {}", file.frame_count());
			info!("  File size: {} bytes\n", file.to_bytes().len());
			file
		}
		Err(e) => {
			info!("✗ Failed to load MFD file: {}", e);
			info!("  Expected location: {}", input_path.display());
			return;
		}
	};

	// Modify the first 10 frames using the improved API
	info!("Modifying first 10 frames with improved API...\n");
	modify_frames_improved(&mut mfd, 10);

	// Save using the convenient save() method
	info!("Saving modified MFD file: {}", output_path.display());
	match mfd.save(&output_path) {
		Ok(_) => {
			info!("✓ Modified file saved successfully");
			info!("  Output: {}\n", output_path.display());
		}
		Err(e) => {
			info!("✗ Failed to save modified file: {}", e);
			return;
		}
	}

	// Export preview of modified frames
	export_preview(&mfd, 10);

	// Show before/after comparison
	show_comparison(&input_path, &mfd);

	info!("\n✓ MFD modification complete!");
	info!("\n📊 API Improvements Demonstrated:");
	info!("  • update_frame() - Direct in-place frame updates");
	info!("  • save() - Convenient file saving");
	info!("  • map_pixels_with_coords() - Coordinate-based pixel mapping");
	info!("  • fill() / fill_rect() - Bulk fill operations");
	info!("  • map() - Functional pixel transformations");
}

/// Modifies frames using the improved ergonomic API
fn modify_frames_improved(mfd: &mut MfdFile, n: usize) {
	let frame_count = mfd.frame_count().min(n as u32);

	for i in 0..frame_count as usize {
		if let Some(mut frame) = mfd.get_frame(i) {
			info!(
				"  Frame #{}: Applying {} pattern ({}x{})",
				i,
				get_pattern_name(i),
				frame.width(),
				frame.height()
			);

			// Apply pattern using improved API
			match i % 10 {
				0 => pattern_checkerboard_improved(&mut frame),
				1 => pattern_border_improved(&mut frame),
				2 => pattern_diagonal_stripes_improved(&mut frame),
				3 => pattern_concentric_circles_improved(&mut frame),
				4 => pattern_spiral_improved(&mut frame),
				5 => pattern_cross_improved(&mut frame),
				6 => pattern_diamond_improved(&mut frame),
				7 => pattern_gradient_improved(&mut frame),
				8 => pattern_random_dots_improved(&mut frame),
				9 => pattern_inverted_improved(&mut frame),
				_ => {}
			}

			// Update the frame back into the MFD file
			mfd.update_frame(i, frame.pixels());
		}
	}

	info!("");
}

fn get_pattern_name(index: usize) -> &'static str {
	match index % 10 {
		0 => "checkerboard",
		1 => "border",
		2 => "diagonal stripes",
		3 => "concentric circles",
		4 => "spiral",
		5 => "cross",
		6 => "diamond",
		7 => "gradient",
		8 => "random dots",
		9 => "inverted",
		_ => "unknown",
	}
}

// Pattern 0: Checkerboard using map_pixels_with_coords
fn pattern_checkerboard_improved(frame: &mut MfdFrame) {
	frame.map_pixels_with_coords(|x, y, _| {
		if (x + y) % 2 == 0 {
			2
		} else {
			1
		}
	});
}

// Pattern 1: Border using fill_rect and fill
fn pattern_border_improved(frame: &mut MfdFrame) {
	let border_width = 2;

	// Fill entire frame with transparent
	frame.fill(0);

	// Draw borders using fill_rect
	let w = frame.width();
	let h = frame.height();

	// Top border
	frame.fill_rect(0, 0, w, border_width, 1);
	// Bottom border
	if h >= border_width {
		frame.fill_rect(0, h - border_width, w, border_width, 1);
	}
	// Left border
	frame.fill_rect(0, 0, border_width, h, 1);
	// Right border
	if w >= border_width {
		frame.fill_rect(w - border_width, 0, border_width, h, 1);
	}
}

// Pattern 2: Diagonal stripes using map_pixels_with_coords
fn pattern_diagonal_stripes_improved(frame: &mut MfdFrame) {
	frame.map_pixels_with_coords(|x, y, _| {
		if (x + y) % 4 < 2 {
			2
		} else {
			0
		}
	});
}

// Pattern 3: Concentric circles using map_pixels_with_coords
fn pattern_concentric_circles_improved(frame: &mut MfdFrame) {
	let cx = frame.width() as f32 / 2.0;
	let cy = frame.height() as f32 / 2.0;

	frame.map_pixels_with_coords(|x, y, _| {
		let dx = x as f32 - cx;
		let dy = y as f32 - cy;
		let dist = (dx * dx + dy * dy).sqrt();
		if (dist as u16) % 4 < 2 {
			1
		} else {
			0
		}
	});
}

// Pattern 4: Spiral using map_pixels_with_coords
fn pattern_spiral_improved(frame: &mut MfdFrame) {
	let cx = frame.width() as f32 / 2.0;
	let cy = frame.height() as f32 / 2.0;

	frame.map_pixels_with_coords(|x, y, _| {
		let dx = x as f32 - cx;
		let dy = y as f32 - cy;
		let angle = dy.atan2(dx);
		let dist = (dx * dx + dy * dy).sqrt();
		let spiral_val = (angle * 2.0 + dist * 0.5) as i32;
		if spiral_val % 3 == 0 {
			2
		} else {
			0
		}
	});
}

// Pattern 5: Cross using fill_rect
fn pattern_cross_improved(frame: &mut MfdFrame) {
	let cx = frame.width() / 2;
	let cy = frame.height() / 2;
	let thickness = 3;

	frame.fill(0);

	// Vertical bar
	if cx >= thickness {
		frame.fill_rect(cx - thickness, 0, thickness * 2 + 1, frame.height(), 1);
	}

	// Horizontal bar
	if cy >= thickness {
		frame.fill_rect(0, cy - thickness, frame.width(), thickness * 2 + 1, 1);
	}
}

// Pattern 6: Diamond using map_pixels_with_coords
fn pattern_diamond_improved(frame: &mut MfdFrame) {
	let cx = frame.width() as i32 / 2;
	let cy = frame.height() as i32 / 2;

	frame.map_pixels_with_coords(|x, y, _| {
		let dx = (x as i32 - cx).abs();
		let dy = (y as i32 - cy).abs();
		let diamond_dist = dx + dy;
		if diamond_dist % 4 < 2 {
			2
		} else {
			0
		}
	});
}

// Pattern 7: Gradient using map_pixels_with_coords
fn pattern_gradient_improved(frame: &mut MfdFrame) {
	let width = frame.width();
	frame.map_pixels_with_coords(|x, _y, _| {
		let val = (x as f32 / width as f32 * 3.0) as u8;
		match val {
			0 => 0,
			1 => 1,
			_ => 2,
		}
	});
}

// Pattern 8: Random dots using map_pixels
fn pattern_random_dots_improved(frame: &mut MfdFrame) {
	let pixel_count = frame.pixels().len();
	let mut new_pixels = Vec::with_capacity(pixel_count);
	let mut seed = 12345u32;
	for _ in 0..pixel_count {
		seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
		new_pixels.push(if (seed >> 16).is_multiple_of(5) {
			1
		} else {
			0
		});
	}
	for (i, pixel) in frame.pixels_mut().iter_mut().enumerate() {
		*pixel = new_pixels[i];
	}
}

// Pattern 9: Inverted using map
fn pattern_inverted_improved(frame: &mut MfdFrame) {
	frame.map_pixels(|p| match p {
		0 => 2,
		1 => 0,
		_ => 1,
	});
}

/// Exports a preview of modified frames as PGM
fn export_preview(mfd: &MfdFile, count: usize) {
	info!("Exporting preview of first {} modified frames...", count);

	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = PathBuf::from(&cargo_root).join("bin");
	let output_dir = bin_root.join("mfd_improved_preview");

	if let Err(e) = std::fs::create_dir_all(&output_dir) {
		info!("  ✗ Failed to create preview directory: {}", e);
		return;
	}

	let mut success = 0;
	for (i, frame) in mfd.iter().take(count).enumerate() {
		let filename = format!("frame_{:02}_{}.pgm", i, get_pattern_name(i));
		let output_path = output_dir.join(&filename);

		if std::fs::write(&output_path, frame.to_pgm()).is_ok() {
			success += 1;
		}
	}

	info!("  ✓ Exported {} preview frames to: {}", success, output_dir.display());
}

/// Shows a before/after comparison of the first frame
fn show_comparison(original_path: &PathBuf, modified_mfd: &MfdFile) {
	info!("\n📸 Before/After Comparison (Frame 0):\n");

	// Load original
	let Ok(original) = MfdFile::open(original_path) else {
		return;
	};

	// Get both frames
	let Some(original_frame) = original.get_frame(0) else {
		return;
	};

	let Some(modified_frame) = modified_mfd.get_frame(0) else {
		return;
	};

	// Show original
	info!("  BEFORE (Original):");
	info!("  {}", "-".repeat(40));
	for line in original_frame.to_ascii_art_default().lines().take(10) {
		info!("  {}", line);
	}
	info!("  {}", "-".repeat(40));

	// Show modified
	info!("\n  AFTER (Checkerboard Pattern):");
	info!("  {}", "-".repeat(40));
	for line in modified_frame.to_ascii_art_default().lines().take(10) {
		info!("  {}", line);
	}
	info!("  {}", "-".repeat(40));
}
