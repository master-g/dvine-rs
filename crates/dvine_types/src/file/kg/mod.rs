//! `.KG` file format support for `dvine-rs` project.

use std::{fmt::Display, io::Read};

use crate::file::{KgError, kg::constants::MAGIC};

mod constants {
	/// Magic bytes for `.KG` files
	pub const MAGIC: [u8; 2] = [0x4B, 0x47]; // "KG"

	/// Header size for `.KG` files
	pub const HEADER_SIZE: usize = 20;
}

mod opcodes {
	// Opcode definitions
	pub const OP_DICT_LOOKUP: u8 = 0;
	pub const OP_COPY_PREV_PIXEL: u8 = 2;
	pub const OP_COPY_PREV_LINE: u8 = 12;
	pub const OP_COPY_DIAGONAL_1: u8 = 13;
	pub const OP_COPY_DIAGONAL_2: u8 = 14;
	pub const OP_COPY_DOUBLE_BPP: u8 = 15;
}

/// Compression types used in `.KG` files
/// TODO: move this to a more general location after implementing compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Compression {
	/// No compression
	Unsupported = 0,

	/// Image is encoding using RLE with a single pass per plane
	BPP3 = 1,
}

impl Display for Compression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Compression::Unsupported => write!(f, "Unsupported"),
			Compression::BPP3 => write!(f, "BPP3"),
		}
	}
}

/// Header structure for `.KG` files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Header {
	magic: [u8; 2],
	version: u8,
	compression_type: Compression,
	width: u16,
	height: u16,
	reserved_1: [u8; 4],
	palette_offset: u32,
	data_offset: u32,
}

impl Default for Header {
	fn default() -> Self {
		Self {
			magic: MAGIC,
			version: 0x02,
			compression_type: Compression::BPP3,
			width: 0,
			height: 0,
			reserved_1: [0; 4],
			palette_offset: 0,
			data_offset: 0,
		}
	}
}

impl Header {
	/// Creates a new `.KG` file header with the specified parameters.
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the version of the `.KG` file.
	pub fn version(&self) -> u8 {
		self.version
	}

	/// Returns the compression type used in the `.KG` file.
	pub fn compression_type(&self) -> Compression {
		self.compression_type
	}

	/// Returns the width of the image in pixels.
	pub fn width(&self) -> u16 {
		self.width
	}

	/// Returns the height of the image in pixels.
	pub fn height(&self) -> u16 {
		self.height
	}

	/// Returns the offset to the palette data.
	pub fn palette_offset(&self) -> u32 {
		self.palette_offset
	}

	/// Returns the offset to the image data.
	pub fn data_offset(&self) -> u32 {
		self.data_offset
	}

	/// Parses a `.KG` file header from the given byte slice.
	pub fn from_bytes(data: &[u8]) -> Result<Header, KgError> {
		if data.len() < constants::HEADER_SIZE {
			return Err(KgError::InsufficientData {
				expected: constants::HEADER_SIZE,
				actual: data.len(),
			});
		}

		let magic = [data[0], data[1]];
		if magic != constants::MAGIC {
			return Err(KgError::InvalidMagic {
				expected: constants::MAGIC,
				actual: magic,
			});
		}

		let version = data[2];
		let compression_type = match data[3] {
			1 => Compression::BPP3,
			_ => {
				return Err(KgError::UnsupportedCompressionType(data[3]));
			}
		};
		let width = u16::from_le_bytes([data[4], data[5]]);
		let height = u16::from_le_bytes([data[6], data[7]]);
		let mut reserved_1 = [0u8; 4];
		reserved_1.copy_from_slice(&data[8..12]);
		let palette_offset = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
		let data_offset = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);

		Ok(Header {
			magic,
			version,
			compression_type,
			width,
			height,
			reserved_1,
			palette_offset,
			data_offset,
		})
	}

	/// Loads a `.KG` file header from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, KgError> {
		let mut buffer = [0u8; constants::HEADER_SIZE];
		reader.read_exact(&mut buffer)?;
		Self::from_bytes(&buffer)
	}

	/// Converts the `Header` to bytes
	pub fn to_bytes(&self) -> [u8; constants::HEADER_SIZE] {
		let mut bytes = [0u8; constants::HEADER_SIZE];

		bytes[0..2].copy_from_slice(&self.magic);
		bytes[2] = self.version;
		bytes[3] = self.compression_type as u8;
		bytes[4..6].copy_from_slice(&self.width.to_le_bytes());
		bytes[6..8].copy_from_slice(&self.height.to_le_bytes());
		bytes[8..12].copy_from_slice(&self.reserved_1);
		bytes[12..16].copy_from_slice(&self.palette_offset.to_le_bytes());
		bytes[16..20].copy_from_slice(&self.data_offset.to_le_bytes());

		bytes
	}
}

impl Display for Header {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			".KG File Header:\n\
			- Magic: {:02X?}\n\
			- Version: {}\n\
			- Compression Type: {}\n\
			- Width: {} pixels\n\
			- Height: {} pixels\n\
			- Palette Offset: {} bytes\n\
			- Image Data Offset: {} bytes",
			self.magic,
			self.version,
			self.compression_type,
			self.width,
			self.height,
			self.palette_offset,
			self.data_offset,
		)
	}
}

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
	bit_buffer: u8,
	bits_remaining: u32,
	lru_cache: [[u8; 8]; 256],
	palette: [[u8; 4]; 256],
}

impl DecompressorState {
	fn new(width: usize, height: usize, bytes_per_pixel: usize, compressed_data: Vec<u8>) -> Self {
		let total_size = width * height * bytes_per_pixel;
		let line_bytes = width * bytes_per_pixel;

		Self {
			output_buffer: vec![0; total_size],
			compressed_data,
			read_offset: 0,
			write_position: 0,
			total_size,
			line_bytes,
			bytes_per_pixel,
			bit_buffer: 0,
			bits_remaining: 0,
			lru_cache: [[0; 8]; 256],
			palette: [[0; 4]; 256],
		}
	}

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

	fn update_lru_cache(&mut self, reference_color: u8, new_color: u8) {
		let cache_entry = &mut self.lru_cache[reference_color as usize];

		let mut position = 8;
		for (i, &color) in cache_entry.iter().enumerate() {
			if color == new_color {
				position = i;
				break;
			}
		}

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

	fn read_color_index(&mut self) -> u8 {
		let flag = self.read_bits(1);

		if flag == 1 {
			self.read_bits(8) as u8
		} else {
			let ref_pos = self.write_position.saturating_sub(self.bytes_per_pixel);
			let reference_color = self.output_buffer[ref_pos];
			let cache_index = self.read_bits(3) as usize;
			self.lru_cache[reference_color as usize][cache_index]
		}
	}

	fn write_pixel(&mut self, value: u8) {
		self.output_buffer[self.write_position] = value;
	}

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

	fn opcode_0_dictionary_lookup(&mut self) {
		let color_index = self.read_color_index();
		self.write_pixel(color_index);

		let ref_pos = self.write_position.saturating_sub(self.bytes_per_pixel);
		let reference_color = self.output_buffer[ref_pos];

		self.update_lru_cache(reference_color, color_index);
		self.write_position += self.bytes_per_pixel;
	}

	fn opcode_2_copy_previous_pixel(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.bytes_per_pixel);
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_12_copy_up_1_line(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.line_bytes);
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_13_copy_left_up(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = if self.write_position > self.line_bytes {
			self.write_position - self.line_bytes + self.bytes_per_pixel
		} else {
			self.bytes_per_pixel
		};
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_14_copy_up_2_lines(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = if self.write_position > (self.line_bytes + self.bytes_per_pixel) {
			self.write_position - self.line_bytes - self.bytes_per_pixel
		} else {
			0
		};
		self.copy_data(self.write_position, src, length);
	}

	fn opcode_15_copy_up_double(&mut self) {
		let length = self.read_variable_length() as usize;
		let src = self.write_position.saturating_sub(self.bytes_per_pixel * 2);
		self.copy_data(self.write_position, src, length);
	}

	fn decompress_type1(&mut self) -> bool {
		// Read first 2 bytes
		for _ in 0..2 {
			let byte_val = self.read_bits(8) as u8;
			self.write_pixel(byte_val);
			self.write_position += self.bytes_per_pixel;
		}

		// Main loop
		while self.write_position < self.total_size {
			let opcode = self.read_opcode();

			match opcode {
				opcodes::OP_DICT_LOOKUP => self.opcode_0_dictionary_lookup(),
				opcodes::OP_COPY_PREV_PIXEL => self.opcode_2_copy_previous_pixel(),
				opcodes::OP_COPY_PREV_LINE => self.opcode_12_copy_up_1_line(),
				opcodes::OP_COPY_DIAGONAL_1 => self.opcode_13_copy_left_up(),
				opcodes::OP_COPY_DIAGONAL_2 => self.opcode_14_copy_up_2_lines(),
				opcodes::OP_COPY_DOUBLE_BPP => self.opcode_15_copy_up_double(),
				_ => {}
			}
		}

		true
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

/// Result structure for decompressed KG data
#[derive(Debug)]
pub struct DecompressData {
	/// Raw decompressed data in RGB format
	pub raw: Vec<u8>,

	/// Image width
	pub width: u16,

	/// Image height
	pub height: u16,
}

/// Representation of a decoded `.KG` file
#[derive(Debug)]
pub struct File {
	/// Header of the `.KG` file
	header: Header,

	/// Pixel data of the `.KG` file, in RGB format
	pixels: Vec<u8>,
}

impl File {
	/// Returns a reference to the header of the `.KG` file
	pub fn header(&self) -> &Header {
		&self.header
	}

	/// Returns a reference to the pixel data of the `.KG` file
	pub fn pixels(&self) -> &[u8] {
		&self.pixels
	}

	/// Opens and parses a `.KG` file from the specified path
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, KgError> {
		let data = std::fs::read(path)?;
		let decompress_data = decompress(&data)?;

		Ok(Self {
			header: Header::from_bytes(&data)?,
			pixels: decompress_data.raw,
		})
	}
}

/// Decompress KG format data
/// Returns (RGB data, width, height)
pub fn decompress(data: &[u8]) -> Result<DecompressData, KgError> {
	let header = Header::from_bytes(data)?;

	let compression_type = header.compression_type();

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
	}

	let success = if compression_type == Compression::BPP3 {
		state.decompress_type1()
	} else {
		false
	};

	if !success {
		return Err(KgError::DecompressionError);
	}

	let final_data = if has_palette {
		apply_palette(&state.output_buffer, Some(&state.palette))
	} else {
		state.output_buffer
	};

	Ok(DecompressData {
		raw: final_data,
		width: header.width,
		height: header.height,
	})
}
