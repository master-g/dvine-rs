//! KG Image Format Compression
//!
//! ## Overview
//!
//! This module implements the compression algorithm for the KG image format,
//! which is the inverse of the decompression algorithm in `decode.rs`.
//!
//! ## Compression Strategy
//!
//! The encoder uses the same opcodes as the decoder:
//! - Opcode 0: Dictionary lookup with LRU cache
//! - Opcode 2: Copy from previous pixel
//! - Opcode 12: Copy from one line up
//! - Opcode 13: Copy diagonal up-right
//! - Opcode 14: Copy diagonal up-left
//! - Opcode 15: Copy from two pixels back
//!
//! ## Algorithm
//!
//! 1. Convert RGB data to indexed color using palette quantization
//! 2. Write first 2 bytes directly
//! 3. For each subsequent pixel:
//!    - Try to find matching run using various copy patterns
//!    - If no good match, use dictionary lookup
//!    - Choose the most efficient encoding
//! 4. Output compressed bitstream
//!

use crate::file::{DvFileError, FileType};

use super::{Compression, Header, opcodes};

/// Bit writer for encoding the compressed bitstream
#[derive(Debug)]
struct BitWriter {
	data: Vec<u8>,
	bit_buffer: u8,
	bits_in_buffer: u32,
}

impl BitWriter {
	fn new() -> Self {
		Self {
			data: Vec::new(),
			bit_buffer: 0,
			bits_in_buffer: 0,
		}
	}

	/// Writes specified number of bits to the output stream
	///
	/// Uses big-endian bit ordering (high bits first) to match the decoder
	fn write_bits(&mut self, value: u32, num_bits: u32) {
		if num_bits == 0 {
			return;
		}

		// Write bits one at a time to avoid overflow issues
		for i in (0..num_bits).rev() {
			let bit = if i < 32 {
				(value >> i) & 1
			} else {
				0
			};

			self.bit_buffer = (self.bit_buffer << 1) | (bit as u8);
			self.bits_in_buffer += 1;

			// Flush buffer if full
			if self.bits_in_buffer == 8 {
				self.data.push(self.bit_buffer);
				self.bit_buffer = 0;
				self.bits_in_buffer = 0;
			}
		}
	}

	/// Writes a variable-length integer using progressive encoding
	///
	/// Encoding strategy (reverse of decoder):
	/// - 1-3: Write as 2 bits
	/// - 4-18: Write 2 zero bits, then (value-3) as 4 bits
	/// - 19-255: Write 6 zero bits, then value as 8 bits
	/// - 256-65535: Write 14 zero bits, then value as 16 bits
	/// - Larger: Write 30 zero bits, then value as 32 bits (16-bit high + 16-bit low)
	fn write_variable_length(&mut self, value: u32) {
		if (1..=3).contains(&value) {
			self.write_bits(value, 2);
		} else if (4..=18).contains(&value) {
			self.write_bits(0, 2);
			self.write_bits(value - 3, 4);
		} else if (19..=255).contains(&value) {
			self.write_bits(0, 2);
			self.write_bits(0, 4);
			self.write_bits(value, 8);
		} else if (256..=65535).contains(&value) {
			self.write_bits(0, 2);
			self.write_bits(0, 4);
			self.write_bits(0, 8);
			self.write_bits(value, 16);
		} else {
			self.write_bits(0, 2);
			self.write_bits(0, 4);
			self.write_bits(0, 8);
			self.write_bits(0, 16);
			let high = value >> 16;
			let low = value & 0xFFFF;
			self.write_bits(high, 16);
			self.write_bits(low, 16);
		}
	}

	/// Writes an opcode using variable-length prefix encoding
	///
	/// Encoding: 0="0", 2="10", {12,13,14,15}="11"+`2bit_index`
	fn write_opcode(&mut self, opcode: u8) {
		match opcode {
			0 => self.write_bits(0, 1),
			2 => self.write_bits(0b10, 2),
			12 => self.write_bits(0b1100, 4),
			13 => self.write_bits(0b1101, 4),
			14 => self.write_bits(0b1110, 4),
			15 => self.write_bits(0b1111, 4),
			_ => {}
		}
	}

	/// Flushes any remaining bits in the buffer
	fn flush(&mut self) {
		if self.bits_in_buffer > 0 {
			self.bit_buffer <<= 8 - self.bits_in_buffer;
			self.data.push(self.bit_buffer);
			self.bit_buffer = 0;
			self.bits_in_buffer = 0;
		}
	}

	/// Returns the compressed data
	fn into_data(mut self) -> Vec<u8> {
		self.flush();
		self.data
	}
}

/// State structure for the compressor
#[derive(Debug)]
struct CompressorState {
	indexed_data: Vec<u8>,
	writer: BitWriter,
	read_position: usize,
	#[allow(dead_code)]
	width: usize,
	#[allow(dead_code)]
	height: usize,
	bytes_per_pixel: usize,
	line_bytes: usize,
	lru_cache: [[u8; 8]; 256],
	current_color: u8,
}

impl CompressorState {
	fn new(indexed_data: Vec<u8>, width: usize, height: usize, bytes_per_pixel: usize) -> Self {
		let line_bytes = width * bytes_per_pixel;
		let lru_cache = [[0u8, 1, 2, 3, 4, 5, 6, 7]; 256];

		Self {
			indexed_data,
			writer: BitWriter::new(),
			read_position: 0,
			width,
			height,
			bytes_per_pixel,
			line_bytes,
			lru_cache,
			current_color: 0,
		}
	}

	/// Updates the LRU cache by moving a color to the front
	///
	/// Must match the decoder's behavior exactly
	#[inline(always)]
	fn update_lru_cache(&mut self, _reference_color: u8, new_color: u8) {
		let cache_entry = &mut self.lru_cache[self.current_color as usize];

		let mut position = cache_entry.iter().position(|&color| color == new_color).unwrap_or(8);

		if position == 0 {
			return;
		}

		if position == 8 {
			position = 7;
		}

		for i in (1..=position).rev() {
			cache_entry[i] = cache_entry[i - 1];
		}
		cache_entry[0] = new_color;
	}

	/// Writes a color index either directly or through LRU cache
	///
	/// Returns the number of bits written
	fn write_color_index(&mut self, color_index: u8) -> u32 {
		if self.read_position < self.bytes_per_pixel {
			// No previous pixel, write directly
			self.writer.write_bits(1, 1);
			self.writer.write_bits(color_index as u32, 8);
			return 9;
		}

		let ref_pos = self.read_position - self.bytes_per_pixel;
		let reference_color = self.indexed_data[ref_pos];

		// Check if color is in LRU cache
		let cache_entry = &self.lru_cache[reference_color as usize];
		if let Some(cache_index) = cache_entry.iter().position(|&c| c == color_index) {
			// Write through cache (1 + 3 = 4 bits)
			self.writer.write_bits(0, 1);
			self.writer.write_bits(cache_index as u32, 3);
			4
		} else {
			// Write directly (1 + 8 = 9 bits)
			self.writer.write_bits(1, 1);
			self.writer.write_bits(color_index as u32, 8);
			9
		}
	}

	/// Encodes a dictionary lookup operation
	fn encode_dictionary_lookup(&mut self) {
		let color_index = self.indexed_data[self.read_position];

		// Write opcode 0
		self.writer.write_opcode(opcodes::OP_DICT_LOOKUP);

		// Write color index
		self.write_color_index(color_index);

		// Update LRU cache (must match decoder)
		if self.read_position >= self.bytes_per_pixel {
			let ref_pos = self.read_position - self.bytes_per_pixel;
			let reference_color = self.indexed_data[ref_pos];
			self.current_color = reference_color;
			self.update_lru_cache(reference_color, color_index);
		}

		self.read_position += self.bytes_per_pixel;
	}

	/// Checks if we can match a run from the given source position
	fn check_run_length(&self, src_pos: usize) -> usize {
		let mut length = 0;
		let mut read_pos = self.read_position;
		let mut src = src_pos;
		let total_size = self.indexed_data.len();

		while read_pos < total_size && src < read_pos {
			if self.indexed_data[read_pos] != self.indexed_data[src] {
				break;
			}
			length += 1;
			read_pos += self.bytes_per_pixel;
			src += self.bytes_per_pixel;
		}

		length
	}

	/// Tries to find the best copy operation for current position
	///
	/// Returns (opcode, length) or None if no good match found
	#[allow(clippy::unnecessary_map_or)]
	fn find_best_copy_operation(&self) -> Option<(u8, usize)> {
		let min_length = 2; // Minimum length to be worth encoding
		let mut best_op: Option<(u8, usize)> = None;

		// Try opcode 2: copy from previous pixel
		if self.read_position >= self.bytes_per_pixel {
			let src = self.read_position - self.bytes_per_pixel;
			let length = self.check_run_length(src);
			if length >= min_length {
				best_op = Some((opcodes::OP_COPY_PREV_PIXEL, length));
			}
		}

		// Try opcode 12: copy from one line up
		if self.read_position >= self.line_bytes {
			let src = self.read_position - self.line_bytes;
			let length = self.check_run_length(src);
			if length >= min_length && best_op.map_or(true, |(_, len)| length > len) {
				best_op = Some((opcodes::OP_COPY_PREV_LINE, length));
			}
		}

		// Try opcode 13: copy diagonal up-right
		if self.read_position >= self.line_bytes && self.line_bytes > self.bytes_per_pixel {
			let src = self.read_position - self.line_bytes + self.bytes_per_pixel;
			if src < self.read_position {
				let length = self.check_run_length(src);
				if length >= min_length && best_op.map_or(true, |(_, len)| length > len) {
					best_op = Some((opcodes::OP_COPY_DIAGONAL_1, length));
				}
			}
		}

		// Try opcode 14: copy diagonal up-left
		if self.read_position >= self.line_bytes + self.bytes_per_pixel {
			let src = self.read_position - self.line_bytes - self.bytes_per_pixel;
			let length = self.check_run_length(src);
			if length >= min_length && best_op.map_or(true, |(_, len)| length > len) {
				best_op = Some((opcodes::OP_COPY_DIAGONAL_2, length));
			}
		}

		// Try opcode 15: copy from two pixels back
		if self.read_position >= self.bytes_per_pixel * 2 {
			let src = self.read_position - self.bytes_per_pixel * 2;
			let length = self.check_run_length(src);
			if length >= min_length && best_op.map_or(true, |(_, len)| length > len) {
				best_op = Some((opcodes::OP_COPY_DOUBLE_BPP, length));
			}
		}

		best_op
	}

	/// Encodes a copy operation
	fn encode_copy_operation(&mut self, opcode: u8, length: usize) {
		self.writer.write_opcode(opcode);
		self.writer.write_variable_length(length as u32);
		self.read_position += length * self.bytes_per_pixel;
	}

	/// Main compression routine for Type 1 compression
	fn compress_type1(&mut self) {
		let total_size = self.indexed_data.len();

		// Write first 2 bytes directly
		for _ in 0..2 {
			if self.read_position >= total_size {
				break;
			}
			let byte_val = self.indexed_data[self.read_position];
			self.writer.write_bits(byte_val as u32, 8);
			self.read_position += self.bytes_per_pixel;
		}

		// Main compression loop
		while self.read_position < total_size {
			// Try to find a good copy operation
			if let Some((opcode, length)) = self.find_best_copy_operation() {
				self.encode_copy_operation(opcode, length);
			} else {
				// No good match, use dictionary lookup
				self.encode_dictionary_lookup();
			}
		}
	}
}

/// Builds a palette from RGB image data
///
/// This is a simple palette generation that finds unique colors.
/// For better results, consider using a proper color quantization algorithm.
fn build_palette(rgb_data: &[u8]) -> Result<([[u8; 4]; 256], Vec<u8>), DvFileError> {
	let pixel_count = rgb_data.len() / 3;
	let mut unique_colors = Vec::new();
	let mut indexed_data = vec![0u8; pixel_count];

	for i in 0..pixel_count {
		let r = rgb_data[i * 3];
		let g = rgb_data[i * 3 + 1];
		let b = rgb_data[i * 3 + 2];
		let color = [r, g, b, 0];

		// Find or add color to palette
		let index = if let Some(idx) = unique_colors.iter().position(|&c| c == color) {
			idx
		} else {
			if unique_colors.len() >= 256 {
				return Err(DvFileError::CompressionError {
					file_type: FileType::Kg,
					message: format!(
						"Image has more than 256 unique colors (found at pixel {})",
						i
					),
				});
			}
			unique_colors.push(color);
			unique_colors.len() - 1
		};

		indexed_data[i] = index as u8;
	}

	// Create full palette array
	let mut palette = [[0u8; 4]; 256];
	for (i, &color) in unique_colors.iter().enumerate() {
		palette[i] = color;
	}

	Ok((palette, indexed_data))
}

/// Converts palette from RGB to BGR format for KG file
fn palette_to_bgr(palette: &[[u8; 4]; 256]) -> Vec<u8> {
	let mut bgr_data = Vec::with_capacity(1024);
	for color in palette.iter() {
		bgr_data.push(color[2]); // B
		bgr_data.push(color[1]); // G
		bgr_data.push(color[0]); // R
		bgr_data.push(0); // A
	}
	bgr_data
}

/// Compresses RGB image data into KG format
///
/// # Arguments
///
/// * `rgb_data` - Raw RGB pixel data (width * height * 3 bytes)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
///
/// A complete KG file as a byte vector
pub fn compress(rgb_data: &[u8], width: u16, height: u16) -> Result<Vec<u8>, DvFileError> {
	// Validate input
	let expected_size = (width as usize) * (height as usize) * 3;
	if rgb_data.len() != expected_size {
		return Err(DvFileError::CompressionError {
			file_type: FileType::Kg,
			message: format!(
				"Invalid RGB data size: expected {} bytes ({}x{} * 3), got {} bytes",
				expected_size,
				width,
				height,
				rgb_data.len()
			),
		});
	}

	// Build palette and convert to indexed color
	let (palette, indexed_data) = build_palette(rgb_data)?;

	// Compress the indexed data
	let mut state = CompressorState::new(
		indexed_data,
		width as usize,
		height as usize,
		1, // bytes_per_pixel for indexed color
	);
	state.compress_type1();
	let compressed_data = state.writer.into_data();

	// Build file structure
	let padding = Header::default().create_default_padding();
	let palette_offset = (Header::SIZE + padding.len()) as u32;
	let palette_bytes = palette_to_bgr(&palette);
	let data_offset = palette_offset + palette_bytes.len() as u32;
	let file_size = data_offset + compressed_data.len() as u32;

	// Create header
	let header = Header {
		width,
		height,
		palette_offset,
		data_offset,
		file_size,
		compression_type: Compression::BPP3,
		version: 0x02,
		..Header::default()
	};

	// Assemble final file
	let mut output = Vec::with_capacity(file_size as usize);
	output.extend_from_slice(&header.to_bytes());
	output.extend_from_slice(&padding);
	output.extend_from_slice(&palette_bytes);
	output.extend_from_slice(&compressed_data);

	Ok(output)
}

/// Compresses a `File` structure back into KG format bytes
///
/// This is useful when you've loaded a KG file and want to save it again,
/// possibly after modifications.
pub fn compress_file(file: &super::File) -> Result<Vec<u8>, DvFileError> {
	let width = file.header().width();
	let height = file.header().height();
	compress(file.pixels(), width, height)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::file::kg::decode;

	#[test]
	fn test_bit_writer() {
		let mut writer = BitWriter::new();

		// Write some bits
		writer.write_bits(0b101, 3);
		writer.write_bits(0b1100, 4);
		writer.write_bits(0b1, 1);

		let data = writer.into_data();
		// 101_1100_1 = 0b10111001 = 0xB9
		assert_eq!(data[0], 0b10111001);
	}

	#[test]
	fn test_variable_length_encoding() {
		let mut writer = BitWriter::new();

		// Test small value (1-3): direct 2-bit encoding
		writer.write_variable_length(2);
		let data = writer.into_data();
		assert!(!data.is_empty());

		// Test medium value (4-18)
		let mut writer = BitWriter::new();
		writer.write_variable_length(10);
		let data = writer.into_data();
		assert!(!data.is_empty());
	}

	#[test]
	fn test_simple_compression() {
		// Create a simple 2x2 image with 4 colors
		let rgb_data = vec![
			255, 0, 0, // Red
			0, 255, 0, // Green
			0, 0, 255, // Blue
			255, 255, 0, // Yellow
		];

		let result = compress(&rgb_data, 2, 2);
		assert!(result.is_ok());

		let compressed = result.unwrap();
		assert!(compressed.len() > Header::SIZE);

		// Verify header
		let header = Header::from_bytes(&compressed).unwrap();
		assert_eq!(header.width(), 2);
		assert_eq!(header.height(), 2);
		assert_eq!(header.compression_type(), Compression::BPP3);
	}

	#[test]
	fn test_encode_decode_roundtrip() {
		// Create a test image with a pattern
		let width = 8;
		let height = 8;
		let mut rgb_data = Vec::with_capacity(width * height * 3);

		// Create a simple gradient pattern
		for y in 0..height {
			for x in 0..width {
				let r = ((x * 255) / width) as u8;
				let g = ((y * 255) / height) as u8;
				let b = 128u8;
				rgb_data.push(r);
				rgb_data.push(g);
				rgb_data.push(b);
			}
		}

		// Compress
		let compressed = compress(&rgb_data, width as u16, height as u16).unwrap();

		// Decompress
		let decompressed = decode::decompress(&compressed).unwrap();

		// Verify dimensions
		assert_eq!(decompressed.header().width(), width as u16);
		assert_eq!(decompressed.header().height(), height as u16);

		// Verify pixel data matches (should be identical after roundtrip)
		assert_eq!(decompressed.pixels(), &rgb_data);
	}

	#[test]
	fn test_encode_solid_color() {
		// Test with a solid color image (should compress well)
		let width = 16;
		let height = 16;
		let mut rgb_data = Vec::new();
		for _ in 0..(width * height) {
			rgb_data.extend_from_slice(&[255, 0, 0]); // All red pixels
		}

		let compressed = compress(&rgb_data, width as u16, height as u16).unwrap();
		let decompressed = decode::decompress(&compressed).unwrap();

		assert_eq!(decompressed.pixels(), &rgb_data);
	}

	#[test]
	fn test_encode_repeating_pattern() {
		// Test with a repeating pattern
		let width = 4;
		let height = 4;
		let mut rgb_data = Vec::new();

		// Create a checkerboard pattern with 2 colors
		for y in 0..height {
			for x in 0..width {
				if (x + y) % 2 == 0 {
					rgb_data.extend_from_slice(&[255, 255, 255]); // White
				} else {
					rgb_data.extend_from_slice(&[0, 0, 0]); // Black
				}
			}
		}

		let compressed = compress(&rgb_data, width as u16, height as u16).unwrap();
		let decompressed = decode::decompress(&compressed).unwrap();

		assert_eq!(decompressed.pixels(), &rgb_data);
	}

	#[test]
	fn test_encode_too_many_colors() {
		// Create an image with more than 256 colors (should fail)
		let width = 32;
		let height = 32;
		let mut rgb_data = Vec::new();

		for i in 0..(width * height) {
			let r = (i % 256) as u8;
			let g = ((i / 256) % 256) as u8;
			let b = ((i / 65536) % 256) as u8;
			rgb_data.extend_from_slice(&[r, g, b]);
		}

		let result = compress(&rgb_data, width as u16, height as u16);
		assert!(result.is_err());
		match result.unwrap_err() {
			DvFileError::CompressionError {
				..
			} => {}
			_ => panic!("Expected CompressionError"),
		}
	}

	#[test]
	fn test_encode_invalid_dimensions() {
		// Test with invalid data size
		let rgb_data = vec![255, 0, 0, 0, 255, 0]; // 2 pixels worth
		let result = compress(&rgb_data, 4, 4); // But claim 4x4

		assert!(result.is_err());
		match result.unwrap_err() {
			DvFileError::CompressionError {
				..
			} => {}
			_ => panic!("Expected CompressionError"),
		}
	}
}
