//! Font support test
//!
//! This module provides functionality to test and export font files.
//!
//! # Features
//!
//! - Load and inspect font files (SYSTEM.FNT, RUBI.FNT)
//! - Export all glyphs to PNG images with visual grid layout
//! - Test Shift-JIS character encoding and lookup
//!
//! # Font Dump Format
//!
//! The `dump_font_to_image()` function exports fonts with:
//! - White background (RGB: 255, 255, 255)
//! - Black glyph pixels (RGB: 0, 0, 0)
//! - Green 1px separators between glyphs (RGB: 0, 255, 0)
//! - Square grid layout for optimal viewing

use dvine_internal::prelude::*;
use image::{ImageBuffer, Rgb, RgbImage};
use log::info;

pub(super) fn test_fonts() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	let sys_font_path = bin_root.join("system.fnt");
	let rubi_font_path = bin_root.join("rubi.fnt");

	info!("Loading font files...");

	let sys_font = match FntFile::open(&sys_font_path) {
		Ok(font) => {
			info!("✓ Loaded system font: {}", sys_font_path.display());
			info!("  Font size: {}", font.font_size());
			info!("  Bytes per glyph: {}", font.bytes_per_glyph());
			info!("  Number of glyphs: {}", font.num_of_glyphs());
			font
		}
		Err(e) => {
			info!("✗ Failed to load system font: {}", e);
			return;
		}
	};

	let rubi_font = match FntFile::open(&rubi_font_path) {
		Ok(font) => {
			info!("✓ Loaded rubi font: {}", rubi_font_path.display());
			info!("  Font size: {}", font.font_size());
			info!("  Bytes per glyph: {}", font.bytes_per_glyph());
			info!("  Number of glyphs: {}", font.num_of_glyphs());
			font
		}
		Err(e) => {
			info!("✗ Failed to load rubi font: {}", e);
			return;
		}
	};

	// Comment out ASCII art printing to reduce output
	// sys_font.iter().for_each(|g| {
	// 	let b: GlyphBitmap = g.into();
	// 	let art = b.to_ascii_art();
	// 	println!("{art}");
	// });

	// test_jis_encoding(&sys_font, &rubi_font);

	dump_font_to_image(&sys_font, "system_font_dump.png");
	dump_font_to_image(&rubi_font, "rubi_font_dump.png");

	info!("\n✓ Font test complete");
}

#[allow(dead_code)]
fn test_jis_encoding(sys_font: &FntFile, rubi_font: &FntFile) {
	// Test encoding Japanese text to Shift-JIS
	// Shift-JIS is a character encoding for Japanese text used in many legacy systems
	info!("\nTesting Shift-JIS encoding...");
	let test_text = "あいうえおアイウエオ漢字";
	info!("  Original text: {}", test_text);

	// Encode UTF-8 string to Shift-JIS bytes
	// This converts Unicode characters to their Shift-JIS byte representation
	let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(test_text);

	if had_errors {
		info!("  ⚠ Encoding had errors");
	} else {
		info!("  ✓ Encoded to {} bytes", encoded.len());
	}

	// Convert Shift-JIS bytes to character codes
	// In Shift-JIS: ASCII uses 1 byte (0x00-0x7F), Japanese characters use 2 bytes
	info!("\n  Character codes:");
	let mut i = 0;
	while i < encoded.len() {
		let code = if encoded[i] < 0x80 {
			// Single-byte character (ASCII range: 0x00-0x7F)
			let code = encoded[i] as u16;
			i += 1;
			code
		} else if i + 1 < encoded.len() {
			// Double-byte character (Japanese character)
			// Combine two bytes in big-endian order to form the character code
			let code = u16::from_be_bytes([encoded[i], encoded[i + 1]]);
			i += 2;
			code
		} else {
			// Incomplete sequence (shouldn't happen with valid encoding)
			i += 1;
			continue;
		};

		info!("    0x{:04X}", code);

		// Try to lookup the glyph in system font using the Shift-JIS character code
		// Note: Font files may use different indexing schemes, so not all codes may be present
		if let Some(glyph) = sys_font.lookup(code) {
			info!("      ✓ Found in system font");
			// Optionally display the glyph as ASCII art
			// Convert glyph to bitmap for ASCII art visualization
			let bitmap: GlyphBitmap = (&glyph).into();
			let art = bitmap.to_ascii_art();
			// Print first 4 lines only (to avoid flooding the log)
			println!("{art}");
		} else {
			info!("      ✗ Not found in system font");
		}
	}

	// Test ASCII character lookup
	info!("\nTesting ASCII character lookup...");
	let ascii_chars = "ABC123";
	for ch in ascii_chars.chars() {
		let code = ch as u16;
		if let Some(glyph) = sys_font.lookup(code) {
			info!("  '{}' (0x{:04X}): ✓ Found", ch, code);
			let bitmap: GlyphBitmap = (&glyph).into();
			let art = bitmap.to_ascii_art();
			println!("{art}");
		} else {
			info!("  '{}' (0x{:04X}): ✗ Not found", ch, code);
		}
	}

	// Compare fonts
	info!("\nFont comparison:");
	info!("  System font: {} glyphs, size {}", sys_font.num_of_glyphs(), sys_font.font_size());
	info!("  Rubi font:   {} glyphs, size {}", rubi_font.num_of_glyphs(), rubi_font.font_size());
}

/// Dumps all glyphs from a font to a square PNG image.
///
/// # Layout
/// - Background: White (RGB: 255, 255, 255)
/// - Glyph pixels: Black (RGB: 0, 0, 0)
/// - Separators: 1px green lines (RGB: 0, 255, 0)
/// - Grid: Square layout with glyphs arranged in rows and columns
///
/// # Arguments
/// * `font` - Font file to export
/// * `output_filename` - Output filename (will be saved to bin/ directory)
fn dump_font_to_image(font: &FntFile, output_filename: &str) {
	info!("\nDumping font to image: {}", output_filename);

	// Collect all glyphs from the font
	let glyphs: Vec<Glyph> = font.iter().collect();
	let num_glyphs = glyphs.len();

	if num_glyphs == 0 {
		info!("  ⚠ No glyphs to dump");
		return;
	}

	info!("  Total glyphs: {}", num_glyphs);

	// Calculate grid dimensions to make a square layout
	let grid_size = (num_glyphs as f64).sqrt().ceil() as usize;
	info!("  Grid layout: {}x{} cells", grid_size, grid_size);

	// Get glyph size and separator width
	let glyph_size = font.font_size() as u32;
	let separator_width = 1u32;

	// Calculate image dimensions: (glyph_size + separator) * grid_size + separator
	let img_width = grid_size as u32 * (glyph_size + separator_width) + separator_width;
	let img_height = grid_size as u32 * (glyph_size + separator_width) + separator_width;

	info!("  Image dimensions: {}x{} pixels", img_width, img_height);

	// Create image with white background (255, 255, 255)
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, white);

	// Draw green separators (1px grid lines)
	let green = Rgb([0, 255, 0]);

	// Vertical separators
	for col in 0..=grid_size {
		let x = col as u32 * (glyph_size + separator_width);
		for y in 0..img_height {
			img.put_pixel(x, y, green);
		}
	}

	// Horizontal separators
	for row in 0..=grid_size {
		let y = row as u32 * (glyph_size + separator_width);
		for x in 0..img_width {
			img.put_pixel(x, y, green);
		}
	}

	// Draw each glyph in black
	let black = Rgb([0, 0, 0]);

	for (idx, glyph) in glyphs.iter().enumerate() {
		let grid_row = idx / grid_size;
		let grid_col = idx % grid_size;

		// Calculate starting position for this glyph (offset by separator)
		let start_x = grid_col as u32 * (glyph_size + separator_width) + separator_width;
		let start_y = grid_row as u32 * (glyph_size + separator_width) + separator_width;

		// Convert glyph to bitmap and draw using line iterator
		let bitmap: GlyphBitmap = glyph.into();

		for (y_offset, line) in bitmap.line_iterator().enumerate() {
			for (x_offset, &pixel) in line.iter().enumerate() {
				if pixel {
					img.put_pixel(start_x + x_offset as u32, start_y + y_offset as u32, black);
				}
			}
		}
	}

	// Save the image to bin/ directory
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let output_path = bin_root.join(output_filename);

	match img.save(&output_path) {
		Ok(_) => info!("  ✓ Image saved: {}", output_path.display()),
		Err(e) => info!("  ✗ Failed to save image: {}", e),
	}
}
