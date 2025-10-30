//! Font support test

use dvine_internal::prelude::*;
use image::{ImageBuffer, Rgb, RgbImage};
use log::info;

#[allow(unused)]
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

	sys_font.iter().for_each(|g| {
		let b: GlyphBitmap = g.into();
		let art = b.to_ascii_art();
		println!("{art}");
	});

	// test_jis_encoding(&sys_font, &rubi_font);

	info!("\n✓ Font test complete");
}

#[allow(unused)]
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

/// Renders glyphs to a bitmap image file.
///
/// Creates a PNG image with glyphs arranged in a grid pattern.
/// - Background: White (255, 255, 255)
/// - Glyph pixels: Black (0, 0, 0)
/// - Spacing: Magenta (255, 0, 255)
///
/// # Arguments
///
/// * `font` - Font file to render glyphs from
/// * `text` - Text to render (will be encoded to Shift-JIS)
/// * `output_path` - Path to save the output PNG file
/// * `spacing` - Number of pixels between glyphs (default: 2)
#[allow(unused)]
pub(super) fn render_glyphs_to_bitmap(
	font: &FntFile,
	text: &str,
	output_path: &std::path::Path,
	spacing: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	info!("Rendering glyphs to bitmap...");
	info!("  Text: {}", text);
	info!("  Output: {}", output_path.display());

	// Encode text to Shift-JIS
	let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
	if had_errors {
		return Err("Failed to encode text to Shift-JIS".into());
	}

	// Extract character codes
	let mut codes = Vec::new();
	let mut i = 0;
	while i < encoded.len() {
		let code = if encoded[i] < 0x80 {
			let code = encoded[i] as u16;
			i += 1;
			code
		} else if i + 1 < encoded.len() {
			let code = u16::from_be_bytes([encoded[i], encoded[i + 1]]);
			i += 2;
			code
		} else {
			i += 1;
			continue;
		};
		codes.push(code);
	}

	info!("  Character codes: {} total", codes.len());

	// Get glyphs
	let mut glyphs = Vec::new();
	for code in &codes {
		if let Some(glyph) = font.lookup(*code) {
			glyphs.push(glyph);
		} else {
			info!("    ✗ Glyph not found for code 0x{:04X}", code);
		}
	}

	if glyphs.is_empty() {
		return Err("No glyphs found for the given text".into());
	}

	info!("  Found {} glyphs", glyphs.len());

	// Get glyph size
	let glyph_size = font.font_size() as u32;
	info!("  Glyph size: {}x{}", glyph_size, glyph_size);

	// Calculate image dimensions
	let glyphs_per_row = 10; // Maximum glyphs per row
	let num_rows = (glyphs.len() as u32).div_ceil(glyphs_per_row);

	let width = glyphs_per_row * glyph_size + (glyphs_per_row + 1) * spacing;
	let height = num_rows * glyph_size + (num_rows + 1) * spacing;

	info!("  Image size: {}x{} pixels", width, height);
	info!("  Layout: {} glyphs per row, {} rows", glyphs_per_row, num_rows);

	// Create image with white background
	let white = Rgb([255u8, 255u8, 255u8]);
	let black = Rgb([0u8, 0u8, 0u8]);
	let magenta = Rgb([255u8, 0u8, 255u8]); // Spacing color

	let mut img: RgbImage = ImageBuffer::from_pixel(width, height, white);

	// Fill spacing areas with magenta
	for y in 0..height {
		for x in 0..width {
			// Check if pixel is in spacing area
			let col = x / (glyph_size + spacing);
			let row = y / (glyph_size + spacing);
			let x_in_cell = x % (glyph_size + spacing);
			let y_in_cell = y % (glyph_size + spacing);

			// Spacing is at the beginning of each cell
			if x_in_cell < spacing || y_in_cell < spacing {
				img.put_pixel(x, y, magenta);
			}
		}
	}

	// Draw glyphs
	for (idx, glyph) in glyphs.iter().enumerate() {
		let row = (idx as u32) / glyphs_per_row;
		let col = (idx as u32) % glyphs_per_row;

		// Calculate top-left position (after spacing)
		let base_x = col * (glyph_size + spacing) + spacing;
		let base_y = row * (glyph_size + spacing) + spacing;

		// Convert glyph to bitmap
		let bitmap: GlyphBitmap = glyph.into();
		let n = glyph_size as usize;

		// Draw each pixel
		for y in 0..n {
			for x in 0..n {
				if bitmap.pixels()[y * n + x] {
					let px = base_x + x as u32;
					let py = base_y + y as u32;
					if px < width && py < height {
						img.put_pixel(px, py, black);
					}
				}
			}
		}
	}

	// Save image
	img.save(output_path)?;
	info!("  ✓ Image saved successfully");

	Ok(())
}

/// Test function to demonstrate rendering glyphs to bitmap
#[allow(unused)]
pub(super) fn test_render_to_bitmap() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	let sys_font_path = bin_root.join("SYSTEM.FNT");
	let rubi_font_path = bin_root.join("RUBI.FNT");

	info!("Loading fonts for bitmap rendering...");

	let sys_font = match FntFile::open(&sys_font_path) {
		Ok(font) => {
			info!("✓ Loaded system font: {}", sys_font_path.display());
			info!("  Font size: {}", font.font_size());
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
			font
		}
		Err(e) => {
			info!("✗ Failed to load rubi font: {}", e);
			return;
		}
	};

	// Test 1: ASCII uppercase alphabet
	let alphabet_output = bin_root.join("font_render_alphabet.png");
	let alphabet_text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

	match render_glyphs_to_bitmap(&sys_font, alphabet_text, &alphabet_output, 2) {
		Ok(_) => info!("✓ Alphabet render complete: {}", alphabet_output.display()),
		Err(e) => info!("✗ Alphabet render failed: {}", e),
	}

	// Test 2: ASCII numbers and lowercase
	let numbers_output = bin_root.join("font_render_numbers.png");
	let numbers_text = "0123456789abcdefghijklmnopqrstuvwxyz";

	match render_glyphs_to_bitmap(&sys_font, numbers_text, &numbers_output, 2) {
		Ok(_) => info!("✓ Numbers render complete: {}", numbers_output.display()),
		Err(e) => info!("✗ Numbers render failed: {}", e),
	}

	// Test 3: Special characters and punctuation
	let special_output = bin_root.join("font_render_special.png");
	let special_text = "!@#$%^&*()_+-=[]{}|;:',.<>?/";

	match render_glyphs_to_bitmap(&sys_font, special_text, &special_output, 2) {
		Ok(_) => info!("✓ Special chars render complete: {}", special_output.display()),
		Err(e) => info!("✗ Special chars render failed: {}", e),
	}

	// Test 4: Hello World with system font
	let hello_output = bin_root.join("font_render_hello.png");
	let hello_text = "Hello World!";

	match render_glyphs_to_bitmap(&sys_font, hello_text, &hello_output, 2) {
		Ok(_) => info!("✓ Hello World render complete: {}", hello_output.display()),
		Err(e) => info!("✗ Hello World render failed: {}", e),
	}

	// Test 5: Same text with rubi font (smaller)
	let hello_rubi_output = bin_root.join("font_render_hello_rubi.png");

	match render_glyphs_to_bitmap(&rubi_font, hello_text, &hello_rubi_output, 2) {
		Ok(_) => info!("✓ Hello World (rubi) render complete: {}", hello_rubi_output.display()),
		Err(e) => info!("✗ Hello World (rubi) render failed: {}", e),
	}

	// Test 6: Different spacing demonstration
	let spacing_output = bin_root.join("font_render_spacing.png");
	let spacing_text = "Spacing Test";

	match render_glyphs_to_bitmap(&sys_font, spacing_text, &spacing_output, 4) {
		Ok(_) => info!("✓ Spacing test (4px) render complete: {}", spacing_output.display()),
		Err(e) => info!("✗ Spacing test render failed: {}", e),
	}

	// Test 7: Japanese text (may not have all glyphs)
	let japanese_output = bin_root.join("font_render_japanese.png");
	let japanese_text = "あいうえおアイウエオ漢字";

	match render_glyphs_to_bitmap(&sys_font, japanese_text, &japanese_output, 2) {
		Ok(_) => info!("✓ Japanese render complete: {}", japanese_output.display()),
		Err(e) => info!("✗ Japanese render failed: {}", e),
	}

	// Test 8: Full ASCII printable characters demonstration
	let full_ascii_output = bin_root.join("font_render_full_ascii.png");
	let full_ascii_text = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";

	match render_glyphs_to_bitmap(&sys_font, full_ascii_text, &full_ascii_output, 2) {
		Ok(_) => info!("✓ Full ASCII render complete: {}", full_ascii_output.display()),
		Err(e) => info!("✗ Full ASCII render failed: {}", e),
	}

	info!("\n✅ Bitmap rendering test complete!");
	info!("Generated images:");
	info!("  • font_render_alphabet.png - Uppercase A-Z");
	info!("  • font_render_numbers.png - Numbers 0-9 and lowercase a-z");
	info!("  • font_render_special.png - Special characters");
	info!("  • font_render_hello.png - Hello World (16x16)");
	info!("  • font_render_hello_rubi.png - Hello World (8x8)");
	info!("  • font_render_spacing.png - Spacing demonstration (4px)");
	info!("  • font_render_japanese.png - Japanese characters (if available)");
	info!("  • font_render_full_ascii.png - All printable ASCII characters");
}
