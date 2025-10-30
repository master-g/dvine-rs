//! Font support test
//!
//! This module provides functionality to test and export font files.
//!
//! # Features
//!
//! - Load and inspect font files (SYSTEM.FNT, RUBI.FNT)
//! - Export all glyphs to PNG images with visual grid layout
//! - Test Shift-JIS character encoding and lookup
//! - Render single-line and multi-line text to images
//!
//! # Font Dump Format
//!
//! The `dump_font_to_image()` function exports fonts with:
//! - White background (RGB: 255, 255, 255)
//! - Black glyph pixels (RGB: 0, 0, 0)
//! - Green 1px separators between glyphs (RGB: 0, 255, 0)
//! - Square grid layout for optimal viewing
//!
//! # Text Rendering
//!
//! The module provides two text rendering functions:
//! - `render_text_to_image()` - Renders a single line of text
//! - `render_multiline_text_to_image()` - Renders multiple lines with proper spacing
//!
//! Both functions:
//! - Use white background and black text
//! - Auto-encode UTF-8 to Shift-JIS
//! - Handle variable-length character encoding
//! - Skip missing glyphs gracefully

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

	// Test text rendering
	render_text_to_image(&sys_font, "Hello World!", "text_render_hello.png");
	render_text_to_image(&sys_font, "あいうえお", "text_render_hiragana.png");
	render_text_to_image(&sys_font, "こんにちは世界", "text_render_mixed.png");
	render_text_to_image(
		&sys_font,
		"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
		"text_render_full_ascii.png",
	);

	// Multi-line text rendering
	render_multiline_text_to_image(
		&sys_font,
		&["Hello World!", "こんにちは", "日本語テスト", "12345"],
		"text_render_multiline.png",
	);

	// Demo: Create a sample text image
	demo_text_rendering(&sys_font);

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

	// Method 1: Using lookup_from_stream (recommended for byte streams)
	info!("\n  Method 1: Using lookup_from_stream");
	let glyphs = sys_font.lookup_from_stream(&encoded);
	info!("    Found {} glyphs", glyphs.len());
	for glyph in &glyphs {
		info!("    0x{:04X}", glyph.code());
		let bitmap: GlyphBitmap = glyph.into();
		let art = bitmap.to_ascii_art();
		println!("{art}");
	}

	// Method 2: Manual parsing (for demonstration)
	info!("\n  Method 2: Manual byte-by-byte parsing");
	let mut i = 0;
	while i < encoded.len() {
		let (glyph, consumed) = sys_font.lookup_from_bytes(&encoded[i..]);

		if let Some(g) = glyph {
			info!(
				"    0x{:04X} ({} byte{})",
				g.code(),
				consumed,
				if consumed > 1 {
					"s"
				} else {
					""
				}
			);
		} else {
			info!(
				"    Not found ({} byte{} consumed)",
				consumed,
				if consumed > 1 {
					"s"
				} else {
					""
				}
			);
		}

		i += consumed;
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

/// Renders a single line of text to a PNG image using the specified font.
///
/// # Layout
/// - Background: White (RGB: 255, 255, 255)
/// - Glyph pixels: Black (RGB: 0, 0, 0)
/// - Padding: 2 pixels on all sides
/// - Character spacing: 1 pixel between characters
///
/// # Arguments
/// * `font` - Font file to use for rendering
/// * `text` - UTF-8 text string to render (single line)
/// * `output_filename` - Output filename (will be saved to bin/ directory)
fn render_text_to_image(font: &FntFile, text: &str, output_filename: &str) {
	info!("\nRendering text to image: {}", output_filename);
	info!("  Text: \"{}\"", text);

	// Encode UTF-8 text to Shift-JIS bytes
	let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
	if had_errors {
		info!("  ⚠ Encoding had errors, some characters may be missing");
	}

	// Get all glyphs from the byte stream
	let glyphs = font.lookup_from_stream(&encoded);

	if glyphs.is_empty() {
		info!("  ⚠ No glyphs found for this text");
		return;
	}

	info!("  Found {} glyphs", glyphs.len());

	// Get glyph dimensions
	let glyph_size = font.font_size() as u32;
	let char_spacing = 1u32;
	let padding = 2u32;

	// Calculate image dimensions
	let img_width = padding * 2 + glyphs.len() as u32 * (glyph_size + char_spacing) - char_spacing;
	let img_height = padding * 2 + glyph_size;

	info!("  Image dimensions: {}x{} pixels", img_width, img_height);

	// Create image with white background
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, white);

	// Draw each glyph in black
	let black = Rgb([0, 0, 0]);
	let mut current_x = padding;

	for glyph in glyphs.iter() {
		// Convert glyph to bitmap
		let bitmap: GlyphBitmap = glyph.into();

		// Draw glyph using line iterator
		for (y_offset, line) in bitmap.line_iterator().enumerate() {
			for (x_offset, &pixel) in line.iter().enumerate() {
				if pixel {
					let px = current_x + x_offset as u32;
					let py = padding + y_offset as u32;
					img.put_pixel(px, py, black);
				}
			}
		}

		// Move to next character position
		current_x += glyph_size + char_spacing;
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

/// Renders multiple lines of text to a PNG image using the specified font.
///
/// # Layout
/// - Background: White (RGB: 255, 255, 255)
/// - Glyph pixels: Black (RGB: 0, 0, 0)
/// - Padding: 4 pixels on all sides
/// - Character spacing: 1 pixel between characters
/// - Line spacing: 2 pixels between lines
///
/// # Arguments
/// * `font` - Font file to use for rendering
/// * `lines` - Array of UTF-8 text strings, one per line
/// * `output_filename` - Output filename (will be saved to bin/ directory)
fn render_multiline_text_to_image(font: &FntFile, lines: &[&str], output_filename: &str) {
	info!("\nRendering multi-line text to image: {}", output_filename);
	info!("  Lines: {}", lines.len());

	// Encode all lines and collect glyphs
	let mut all_line_glyphs = Vec::new();
	let mut max_line_width = 0usize;

	for (idx, line) in lines.iter().enumerate() {
		info!("  Line {}: \"{}\"", idx + 1, line);

		// Encode UTF-8 text to Shift-JIS bytes
		let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(line);
		if had_errors {
			info!("    ⚠ Encoding had errors");
		}

		// Get all glyphs from the byte stream
		let glyphs = font.lookup_from_stream(&encoded);
		info!("    Found {} glyphs", glyphs.len());

		max_line_width = max_line_width.max(glyphs.len());
		all_line_glyphs.push(glyphs);
	}

	if all_line_glyphs.is_empty() || all_line_glyphs.iter().all(std::vec::Vec::is_empty) {
		info!("  ⚠ No glyphs found for any line");
		return;
	}

	// Get glyph dimensions
	let glyph_size = font.font_size() as u32;
	let char_spacing = 1u32;
	let line_spacing = 2u32;
	let padding = 4u32;

	// Calculate image dimensions
	let img_width = if max_line_width > 0 {
		padding * 2 + max_line_width as u32 * (glyph_size + char_spacing) - char_spacing
	} else {
		padding * 2 + glyph_size
	};
	let img_height =
		padding * 2 + all_line_glyphs.len() as u32 * (glyph_size + line_spacing) - line_spacing;

	info!("  Image dimensions: {}x{} pixels", img_width, img_height);

	// Create image with white background
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, white);

	// Draw each line
	let black = Rgb([0, 0, 0]);
	let mut current_y = padding;

	for line_glyphs in all_line_glyphs.iter() {
		let mut current_x = padding;

		for glyph in line_glyphs.iter() {
			// Convert glyph to bitmap
			let bitmap: GlyphBitmap = glyph.into();

			// Draw glyph using line iterator
			for (y_offset, line) in bitmap.line_iterator().enumerate() {
				for (x_offset, &pixel) in line.iter().enumerate() {
					if pixel {
						let px = current_x + x_offset as u32;
						let py = current_y + y_offset as u32;
						img.put_pixel(px, py, black);
					}
				}
			}

			// Move to next character position
			current_x += glyph_size + char_spacing;
		}

		// Move to next line
		current_y += glyph_size + line_spacing;
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

/// Demo function showing various text rendering capabilities.
///
/// Creates a sample image with Japanese and English text showcasing
/// the font rendering capabilities.
fn demo_text_rendering(font: &FntFile) {
	info!("\nCreating demo text rendering...");

	let demo_lines = vec![
		"D+VINE[LUV] Font Demo",
		"",
		"English: ABCDEFGHIJKLMNOPQRSTUVWXYZ",
		"Numbers: 0123456789",
		"",
		"Japanese Hiragana:",
		"あいうえお かきくけこ",
		"さしすせそ たちつてと",
		"",
		"Japanese Katakana:",
		"アイウエオ カキクケコ",
		"サシスセソ タチツテト",
	];

	render_multiline_text_to_image(font, &demo_lines, "text_demo_complete.png");
	info!("  ✓ Demo rendering complete");
}
