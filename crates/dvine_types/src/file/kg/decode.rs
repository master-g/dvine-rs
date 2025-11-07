//! KG Image Format Decompression
//!
//! ## Overview
//!
//! KG is a proprietary image compression format used in `DirectDraw`-based games from around 2000.
//! This module implements the decompression algorithm discovered through reverse engineering
//! of the original game executable (`dvine.exe`).
//!
//! ## File Format
//!
//! ### Header Structure (32 bytes)
//!
//! | Offset | Size | Field              | Description                          |
//! |--------|------|--------------------|--------------------------------------|
//! | 0x00   | 2    | `magic`            | "KG" (0x4B, 0x47)                   |
//! | 0x02   | 1    | `version`          | Version number (usually 0x02)        |
//! | 0x03   | 1    | `compression_type` | Compression type (1/2/3)             |
//! | 0x04   | 2    | `width`            | Image width in pixels                |
//! | 0x06   | 2    | `height`           | Image height in pixels               |
//! | 0x08   | 4    | `reserved_1`       | Reserved bytes                       |
//! | 0x0C   | 4    | `palette_offset`   | Offset to palette data (usually 0x30)|
//! | 0x10   | 4    | `data_offset`      | Offset to compressed data            |
//! | 0x14   | 4    | `file_size`        | Total file size in bytes             |
//! | 0x18   | 8    | `reserved_2`       | Reserved bytes                       |
//!
//! ### Palette Format
//!
//! - **Location**: At `palette_offset` (typically 0x30, after optional padding)
//! - **Size**: 1024 bytes (256 colors × 4 bytes)
//! - **Format**: BGRA, each color is 4 bytes:
//!   - Byte 0: Blue
//!   - Byte 1: Green
//!   - Byte 2: Red
//!   - Byte 3: Alpha (usually 0)
//! - **Note**: Index 0 is typically transparent (0x00, 0xFF, 0xFF, 0x00) for chroma-keying
//!
//! ### Compressed Data
//!
//! - **Location**: At `data_offset`
//! - **Format**: Bitstream-encoded opcodes and data
//! - **Encoding**: Variable-length prefix codes with big-endian bit ordering
//!
//! ## Compression Algorithm (Type 1)
//!
//! Type 1 compression uses a combination of dictionary lookup with LRU caching and
//! various copy operations for efficient compression of palette-indexed images.
//!
//! ### Bytes Per Pixel (BPP)
//!
//! The algorithm supports different BPP values based on compression type:
//! - Type 1: BPP = 1 (8-bit indexed color)
//! - Type 2: BPP = 3 (24-bit RGB, planar format)
//! - Type 3: BPP = 3 (24-bit RGB, interleaved)
//!
//! ### State Variables
//!
//! - `output_buffer`: Decompressed pixel data
//! - `write_position`: Current write offset (in bytes)
//! - `bit_buffer`: 8-bit buffer for bitstream reading
//! - `bits_remaining`: Number of valid bits in buffer
//! - `lru_cache`: 256×8 LRU cache for color indices
//! - `current_color`: Reference color for LRU cache updates
//!
//! ### Opcodes
//!
//! | Opcode | Bits    | Operation                               |
//! |--------|---------|----------------------------------------|
//! | 0      | "0"     | Dictionary lookup (direct or LRU)      |
//! | 2      | "10"    | Copy from previous pixel               |
//! | 12     | "110"   | Copy from one line up                  |
//! | 13     | "1100"  | Copy diagonal up-right                 |
//! | 14     | "1101"  | Copy diagonal up-left                  |
//! | 15     | "1111"  | Copy from two pixels back              |
//!
//! ### Decompression Steps
//!
//! 1. **Initialization**:
//!    - Initialize LRU cache: each entry to {0, 1, 2, 3, 4, 5, 6, 7}
//!    - Write first 2 bytes directly from bitstream
//!
//! 2. **Main Loop** (until `write_position >= total_size`):
//!    - Read opcode from bitstream
//!    - Execute corresponding operation
//!    - Update write position
//!
//! 3. **Dictionary Lookup (Opcode 0)**:
//!    - Read 1-bit flag: if 1, read 8-bit color index directly
//!    - If 0, read 3-bit cache index from LRU cache of previous pixel's color
//!    - Write pixel, update LRU cache with current color as key
//!
//! 4. **Copy Operations (Opcodes 2, 12-15)**:
//!    - Read variable-length count
//!    - Copy pixels from computed source offset
//!
//! ### Variable-Length Integer Encoding
//!
//! Progressive bit reading until non-zero value:
//! 1. Try 2 bits → if non-zero, return value (1-3)
//! 2. Try 4 bits → if non-zero, return value + 3 (4-18)
//! 3. Try 8 bits → if non-zero, return value
//! 4. Try 16 bits → if non-zero, return value
//! 5. Read 32 bits (16-bit high + 16-bit low)
//!

use crate::file::{DvFileError, FileType};

use super::{Compression, Header, opcodes};

/// State structure for the decompressor
#[derive(Debug)]
struct DecompressorState {
	output_buffer: Vec<u8>,
	compressed_data: Vec<u8>,
	read_offset: usize,
	write_position: usize,
	total_size: usize,
	line_bytes: usize,
	bytes_per_pixel: usize,
	current_color: u8,
	bit_buffer: u8,
	bits_remaining: u32,
	lru_cache: [[u8; 8]; 256],
	palette: [[u8; 4]; 256],
}

impl DecompressorState {
	fn new(width: usize, height: usize, bytes_per_pixel: usize, compressed_data: Vec<u8>) -> Self {
		let total_size = width * height * bytes_per_pixel;
		let line_bytes = width * bytes_per_pixel;

		// Initialize LRU cache - each entry must be initialized to {0, 1, 2, 3, 4, 5, 6, 7}
		// This is critical! Initializing to all zeros causes 37% error rate in small files.
		let lru_cache = [[0u8, 1, 2, 3, 4, 5, 6, 7]; 256];

		Self {
			output_buffer: vec![0; total_size],
			compressed_data,
			read_offset: 0,
			write_position: 0,
			total_size,
			line_bytes,
			bytes_per_pixel,
			current_color: 0,
			bit_buffer: 0,
			bits_remaining: 0,
			lru_cache,
			palette: [[0; 4]; 256],
		}
	}

	/// Reads specified number of bits from the compressed bitstream
	///
	/// Uses big-endian bit ordering (high bits first) and maintains an 8-bit buffer
	/// for efficient reading of partial bytes.
	fn read_bits(&mut self, num_bits: u32) -> u32 {
		let mut edx = u32::from(self.bit_buffer);
		let mut ebx = self.bits_remaining;
		let mut eax = num_bits;

		if ebx < num_bits {
			loop {
				let shift_bits = ebx;
				edx <<= shift_bits;
				eax -= ebx;

				let new_byte = self.compressed_data[self.read_offset];
				self.read_offset += 1;
				edx = (edx & 0xFFFF_FF00) | u32::from(new_byte);
				ebx = 8;

				let ecx_loop = eax;
				if ebx >= ecx_loop {
					edx <<= ecx_loop;
					break;
				}
			}
			ebx -= eax;
		} else {
			edx <<= num_bits;
			ebx -= num_bits;
		}

		self.bit_buffer = edx as u8;
		edx >>= 8;
		self.bits_remaining = ebx;

		edx
	}

	/// Reads a variable-length integer from the bitstream
	///
	/// Progressive encoding: tries 2, 4, 8, 16, then 32 bits until non-zero value found.
	fn read_variable_length(&mut self) -> u32 {
		let value = self.read_bits(2);
		if value != 0 {
			return value;
		}

		let value = self.read_bits(4);
		if value != 0 {
			return value + 3;
		}

		let value = self.read_bits(8);
		if value != 0 {
			return value;
		}

		let value = self.read_bits(16);
		if value != 0 {
			return value;
		}

		let high = self.read_bits(16);
		let low = self.read_bits(16);
		(high << 16) | low
	}

	/// Reads an opcode from the bitstream using variable-length prefix encoding
	///
	/// Encoding: 0="0", 2="10", {12,13,14,15}="11"+`2bit_index`
	fn read_opcode(&mut self) -> u8 {
		const OPCODE_TABLE: [u8; 4] = [12, 13, 14, 15];

		let bit1 = self.read_bits(1);
		if bit1 == 0 {
			return 0;
		}

		let bit2 = self.read_bits(1);
		if bit2 == 0 {
			return 2;
		}

		let index = self.read_bits(2) as usize;
		OPCODE_TABLE[index]
	}

	/// Updates the LRU cache by moving a color to the front
	///
	/// CRITICAL: Uses `self.current_color` as index, NOT the `reference_color` parameter!
	/// This matches the assembly code which uses a global variable.
	///
	/// Note: Manual loop is 2× faster than `copy_within` for small arrays (benchmark verified)
	#[inline(always)]
	fn update_lru_cache(&mut self, _reference_color: u8, new_color: u8) {
		let cache_entry = &mut self.lru_cache[self.current_color as usize];

		// Find position of new_color in cache, if it exists
		let mut position = 8;
		for i in 0..8 {
			if cache_entry[i] == new_color {
				position = i;
				break;
			}
		}

		// If already at front, nothing to do
		if position == 0 {
			return;
		}

		// If not found, treat as position 7 (will be shifted out)
		if position == 8 {
			position = 7;
		}

		// Manual shift is faster than copy_within for small arrays (8 elements)
		// Benchmark: manual loop ~1.06 µs vs copy_within ~2.08 µs per 1000 operations
		for i in (1..=position).rev() {
			cache_entry[i] = cache_entry[i - 1];
		}
		cache_entry[0] = new_color;
	}

	/// Reads a color index either directly (8 bits) or from LRU cache (3 bits)
	///
	/// 1-bit flag determines mode: 1=direct read, 0=LRU cache lookup
	#[inline(always)]
	fn read_color_index(&mut self) -> u8 {
		let flag = self.read_bits(1);

		if flag == 1 {
			self.read_bits(8) as u8
		} else {
			// Direct calculation without boundary checks (matches assembly)
			let ref_pos = self.write_position - self.bytes_per_pixel;
			let reference_color = self.output_buffer[ref_pos];
			let cache_index = self.read_bits(3) as usize;
			self.lru_cache[reference_color as usize][cache_index]
		}
	}

	/// Writes a single pixel value to the output buffer at current position
	fn write_pixel(&mut self, value: u8) {
		self.output_buffer[self.write_position] = value;
	}

	/// Copies pixel data from source to destination offset
	///
	/// For BPP=1: Simple byte copy
	/// For BPP>1: Per-pixel copy with stride (planar format)
	fn copy_data(&mut self, dst_offset: usize, src_offset: usize, length: usize) {
		if self.bytes_per_pixel == 1 {
			for i in 0..length {
				self.output_buffer[dst_offset + i] = self.output_buffer[src_offset + i];
			}
			self.write_position += length;
		} else {
			let total_bytes = self.bytes_per_pixel * length;
			self.write_position += total_bytes;

			let mut ecx = dst_offset;
			let mut esi = src_offset;
			let mut edi = length;

			while edi > 0 {
				let dl = self.output_buffer[esi];
				self.output_buffer[ecx] = dl;
				ecx += self.bytes_per_pixel;
				esi += self.bytes_per_pixel;
				edi -= 1;
			}
		}
	}

	/// Opcode 0: Dictionary lookup with LRU cache
	///
	/// Reads color index (direct or from cache), writes pixel, and updates LRU
	#[inline(always)]
	fn opcode_0_dictionary_lookup(&mut self) {
		// Read color index from bitstream or LRU cache
		let color_index = self.read_color_index();

		// Write the pixel value
		self.write_pixel(color_index);

		// Get reference color (direct calculation without boundary checks)
		let ref_pos = self.write_position - self.bytes_per_pixel;
		let reference_color = self.output_buffer[ref_pos];

		// Set current_color - this is used by update_lru_cache
		self.current_color = reference_color;

		// Update LRU cache
		self.update_lru_cache(reference_color, color_index);

		// Advance write position
		self.write_position += self.bytes_per_pixel;
	}

	/// Opcode 2: Copy from previous pixel (left or up depending on position)
	///
	/// Reads variable-length count and copies from `write_position - bytes_per_pixel`
	fn opcode_2_copy_previous_pixel(&mut self) {
		let length = self.read_variable_length() as usize;
		// Direct calculation without boundary checks (matches assembly)
		let src = self.write_position - self.bytes_per_pixel;
		self.copy_data(self.write_position, src, length);
	}

	/// Opcode 12: Copy from one line up (same horizontal position)
	fn opcode_12_copy_up_1_line(&mut self) {
		let length = self.read_variable_length() as usize;
		// Direct calculation without boundary checks (matches assembly)
		let src = self.write_position - self.line_bytes;
		self.copy_data(self.write_position, src, length);
	}

	/// Opcode 13: Copy from diagonal up-right (one line up, one pixel right)
	fn opcode_13_copy_up_right(&mut self) {
		let length = self.read_variable_length() as usize;
		// Direct calculation without boundary checks (matches assembly)
		let src = self.write_position - self.line_bytes + self.bytes_per_pixel;
		self.copy_data(self.write_position, src, length);
	}

	/// Opcode 14: Copy from diagonal up-left (one line up, one pixel left)
	fn opcode_14_copy_up_left(&mut self) {
		let length = self.read_variable_length() as usize;
		// Direct calculation without boundary checks (matches assembly)
		let src = self.write_position - self.line_bytes - self.bytes_per_pixel;
		self.copy_data(self.write_position, src, length);
	}

	/// Opcode 15: Copy from two pixels back (double BPP offset)
	fn opcode_15_copy_up_double(&mut self) {
		let length = self.read_variable_length() as usize;
		// Direct calculation without boundary checks (matches assembly)
		let src = self.write_position - self.bytes_per_pixel * 2;
		self.copy_data(self.write_position, src, length);
	}

	/// Main decompression routine for Type 1 compression
	///
	/// 1. Initialize by writing first 2 bytes directly
	/// 2. Loop reading opcodes and executing operations until complete
	fn decompress_type1(&mut self) {
		// Initialize: Read first 2 bytes directly from bitstream
		for _ in 0..2 {
			let byte_val = self.read_bits(8) as u8;
			self.write_pixel(byte_val);
			self.write_position += self.bytes_per_pixel;
		}

		// Main decompression loop
		while self.write_position < self.total_size {
			let opcode = self.read_opcode();

			match opcode {
				opcodes::OP_DICT_LOOKUP => self.opcode_0_dictionary_lookup(),
				opcodes::OP_COPY_PREV_PIXEL => self.opcode_2_copy_previous_pixel(),
				opcodes::OP_COPY_PREV_LINE => self.opcode_12_copy_up_1_line(),
				opcodes::OP_COPY_DIAGONAL_1 => self.opcode_13_copy_up_right(),
				opcodes::OP_COPY_DIAGONAL_2 => self.opcode_14_copy_up_left(),
				opcodes::OP_COPY_DOUBLE_BPP => self.opcode_15_copy_up_double(),
				_ => {}
			}
		}
	}
}

fn load_palette(data: &[u8], header: &Header) -> Option<[[u8; 4]; 256]> {
	if header.compression_type() != Compression::BPP3 {
		return None;
	}
	if header.palette_offset == 0 {
		return None;
	}

	let mut palette = [[0u8; 4]; 256];
	let palette_offset = header.palette_offset as usize;

	for (i, color) in palette.iter_mut().enumerate() {
		let offset = palette_offset + i * 4;
		if offset + 3 < data.len() {
			let b = data[offset];
			let g = data[offset + 1];
			let r = data[offset + 2];
			color[0] = r;
			color[1] = g;
			color[2] = b;
			color[3] = 0;
		}
	}

	Some(palette)
}

fn apply_palette(indexed_data: &[u8], palette: Option<&[[u8; 4]; 256]>) -> Vec<u8> {
	let Some(pal) = palette else {
		return indexed_data.to_vec();
	};

	let mut rgb_data = Vec::with_capacity(indexed_data.len() * 3);
	for &index in indexed_data {
		let color = &pal[index as usize];
		rgb_data.push(color[0]);
		rgb_data.push(color[1]);
		rgb_data.push(color[2]);
	}

	rgb_data
}

/// Decompress KG format data from a byte slice
/// Returns (Header, RGB data)
pub fn decompress(data: &[u8]) -> Result<super::File, DvFileError> {
	let header = Header::from_bytes(data)?;

	let compression_type = header.compression_type();

	let padding =
		header.padding_size().map(|size| data[Header::SIZE..Header::SIZE + size].to_vec());

	let palette = load_palette(data, &header);
	let has_palette = palette.is_some();

	let output_bpp = if compression_type == Compression::BPP3 && has_palette {
		1
	} else if compression_type == Compression::BPP3 {
		3
	} else {
		1
	};

	let width = header.width as usize;
	let height = header.height as usize;
	let data_offset = header.data_offset as usize;

	let compressed_data = data[data_offset..].to_vec();

	let mut state = DecompressorState::new(width, height, output_bpp, compressed_data);
	if let Some(pal) = palette {
		state.palette = pal;
	} else {
		return Err(DvFileError::DecompressionError {
			file_type: FileType::Kg,
			message: "Missing palette".to_string(),
		});
	}

	if compression_type == Compression::BPP3 {
		state.decompress_type1();
	} else {
		return Err(DvFileError::UnsupportedCompressionType {
			file_type: FileType::Kg,
			compression_type: compression_type as u8,
		});
	}

	let final_data = if has_palette {
		apply_palette(&state.output_buffer, Some(&state.palette))
	} else {
		state.output_buffer
	};

	Ok(super::File {
		header,
		padding,
		palette,
		pixels: final_data,
	})
}
