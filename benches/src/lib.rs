//! Benchmark helper utilities for dvine-rs
//!
//! This module provides utilities for generating synthetic test data and common
//! benchmark helpers for the dvine-rs project.
//!
//! # Real Test Files
//!
//! The benchmark suite uses real KG files from the game located in `test_data/`:
//! - `BLACK` - Small file (1.1KB, 640x480) - Simple black image
//! - `VYADOY01` - Large file (126KB, 640x480) - Complex game scene
//!
//! These files are copied from `bin/kg_extract/` and represent real-world workloads.

/// Generates a simple test KG file with specified dimensions
///
/// This creates a minimal valid KG file with BPP3 compression for benchmarking purposes.
pub fn generate_test_kg_data(width: u16, height: u16) -> Vec<u8> {
	let mut data = Vec::new();

	// Magic bytes "KG"
	data.extend_from_slice(&[0x4B, 0x47]);

	// Version
	data.push(0x02);

	// Compression type (BPP3 = 1)
	data.push(0x01);

	// Width (little-endian)
	data.extend_from_slice(&width.to_le_bytes());

	// Height (little-endian)
	data.extend_from_slice(&height.to_le_bytes());

	// Reserved 1 (4 bytes)
	data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

	// Palette offset (after header + padding)
	let padding_size = 16u32;
	let palette_offset = 32u32 + padding_size;
	data.extend_from_slice(&palette_offset.to_le_bytes());

	// Data offset (after palette: 256 colors * 4 bytes)
	let data_offset = palette_offset + 1024;
	data.extend_from_slice(&data_offset.to_le_bytes());

	// File size (placeholder, will calculate later)
	let file_size_pos = data.len();
	data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

	// Reserved 2 (8 bytes)
	data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

	// Padding (16 bytes: zero, width, height, zero)
	data.extend_from_slice(&0u32.to_le_bytes());
	data.extend_from_slice(&(width as u32).to_le_bytes());
	data.extend_from_slice(&(height as u32).to_le_bytes());
	data.extend_from_slice(&0u32.to_le_bytes());

	// Palette (256 colors, BGRA format)
	for i in 0..=255u8 {
		let r = i;
		let g = i.wrapping_mul(2);
		let b = i.wrapping_mul(3);
		data.extend_from_slice(&[b, g, r, 0x00]);
	}

	// Compressed image data (simplified for benchmarking)
	// This creates a simple pattern that exercises the decompressor
	let compressed_data = generate_compressed_data(width as usize, height as usize);
	data.extend_from_slice(&compressed_data);

	// Update file size
	let file_size = data.len() as u32;
	data[file_size_pos..file_size_pos + 4].copy_from_slice(&file_size.to_le_bytes());

	data
}

/// Generates compressed data for a KG file
///
/// Creates realistic compressed data that exercises all major opcodes
fn generate_compressed_data(width: usize, height: usize) -> Vec<u8> {
	let mut compressed = Vec::new();
	let total_pixels = width * height;

	// We need to manually create bit-packed data
	// For simplicity, we'll create a pattern that uses different opcodes

	// First 2 pixels are written directly (8 bits each)
	compressed.push(0x00); // First pixel color index
	compressed.push(0xFF); // Second pixel color index

	// Now we need to encode the rest
	// Let's create a simple pattern with various opcodes
	let mut bits_buffer: Vec<bool> = Vec::new();

	let mut pixels_written = 2;

	while pixels_written < total_pixels {
		let _remaining = total_pixels - pixels_written;

		// Mix different operation types for realistic benchmarking
		match pixels_written % 5 {
			0 => {
				// Opcode 0: Dictionary lookup (bit pattern: 0)
				bits_buffer.push(false);
				// Flag: use cache (0)
				bits_buffer.push(false);
				// Cache index (3 bits): use index 0
				bits_buffer.push(false);
				bits_buffer.push(false);
				bits_buffer.push(false);
				pixels_written += 1;
			}
			1 => {
				// Opcode 2: Copy previous pixel (bit pattern: 10)
				bits_buffer.push(true);
				bits_buffer.push(false);
				// Length: use 2-bit value 1
				bits_buffer.push(false);
				bits_buffer.push(true);
				pixels_written += 1;
			}
			2 => {
				// Opcode 12: Copy up 1 line (bit pattern: 1100)
				bits_buffer.push(true);
				bits_buffer.push(true);
				bits_buffer.push(false);
				bits_buffer.push(false);
				// Length: use 2-bit value 2
				bits_buffer.push(true);
				bits_buffer.push(false);
				pixels_written += 2;
			}
			3 => {
				// Opcode 13: Copy diagonal (bit pattern: 1101)
				bits_buffer.push(true);
				bits_buffer.push(true);
				bits_buffer.push(false);
				bits_buffer.push(true);
				// Length: use 2-bit value 1
				bits_buffer.push(false);
				bits_buffer.push(true);
				pixels_written += 1;
			}
			_ => {
				// Opcode 0 with full color index
				bits_buffer.push(false);
				// Flag: read 8-bit color (1)
				bits_buffer.push(true);
				// Color index (8 bits)
				let color = (pixels_written % 256) as u8;
				for i in (0..8).rev() {
					bits_buffer.push((color >> i) & 1 == 1);
				}
				pixels_written += 1;
			}
		}

		// Prevent infinite loop
		if pixels_written > total_pixels {
			break;
		}
	}

	// Convert bit buffer to bytes
	for chunk in bits_buffer.chunks(8) {
		let mut byte = 0u8;
		for (i, &bit) in chunk.iter().enumerate() {
			if bit {
				byte |= 1 << (7 - i);
			}
		}
		compressed.push(byte);
	}

	// Pad with some extra bytes to ensure we have enough data
	compressed.extend_from_slice(&[0x00; 256]);

	compressed
}

/// Common benchmark sizes for synthetic test data
pub mod sizes {
	/// Tiny image: 64x64 (4,096 pixels)
	pub const TINY: (u16, u16) = (64, 64);
	/// Small image: 256x256 (65,536 pixels)
	pub const SMALL: (u16, u16) = (256, 256);
	/// Medium image: 512x512 (262,144 pixels)
	pub const MEDIUM: (u16, u16) = (512, 512);
	/// Large image: 1024x768 (786,432 pixels) - typical game asset
	pub const LARGE: (u16, u16) = (1024, 768);
	/// Extra large image: 1920x1080 (2,073,600 pixels) - HD resolution
	pub const XLARGE: (u16, u16) = (1920, 1080);
	/// Real game asset size: 640x480 (307,200 pixels) - matches BLACK and VYADOY01
	pub const REAL_GAME: (u16, u16) = (640, 480);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_test_kg_data() {
		let data = generate_test_kg_data(100, 100);

		// Check magic bytes
		assert_eq!(&data[0..2], &[0x4B, 0x47]);

		// Check version
		assert_eq!(data[2], 0x02);

		// Check compression type
		assert_eq!(data[3], 0x01);

		// Check minimum size (header + padding + palette)
		assert!(data.len() >= 32 + 16 + 1024);
	}

	#[test]
	fn test_sizes_constants() {
		assert_eq!(sizes::TINY, (64, 64));
		assert_eq!(sizes::SMALL, (256, 256));
		assert_eq!(sizes::MEDIUM, (512, 512));
		assert_eq!(sizes::LARGE, (1024, 768));
		assert_eq!(sizes::XLARGE, (1920, 1080));
	}
}
