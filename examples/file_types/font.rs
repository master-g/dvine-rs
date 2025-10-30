//! Font support test

use dvine_internal::prelude::*;
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
