//! `.KG` file format support for `dvine-rs` project.

use std::io::Read;

use crate::file::KgError;

mod constants {
	/// Magic bytes for `.KG` files
	pub const MAGIC: [u8; 2] = [0x4B, 0x47]; // "KG"

	/// Header size for `.KG` files
	pub const HEADER_SIZE: usize = 48;
}

/// Compression types used in `.KG` files
/// TODO: move this to a more general location after implementing compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompressionType {
	/// No compression
	None = 0,

	/// RLE 1-pass compression
	RLE1Pass = 1,
}

/// Header structure for `.KG` files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Header {
	magic: [u8; 2],
	version: u8,
	compression_type: CompressionType,
	width: u16,
	height: u16,
	reserved_1: [u8; 4],
	palette_offset: u32,
	image_data_offset: u32,
	file_size: u32,
	reserved_2: [u8; 8],
}

impl Header {
	/// Creates a new `.KG` file header with the specified parameters.
	pub fn new(
		version: u8,
		compression_type: CompressionType,
		width: u16,
		height: u16,
		palette_offset: u32,
		image_data_offset: u32,
		file_size: u32,
	) -> Self {
		Self {
			magic: constants::MAGIC,
			version,
			compression_type,
			width,
			height,
			reserved_1: [0; 4],
			palette_offset,
			image_data_offset,
			file_size,
			reserved_2: [0; 8],
		}
	}

	/// Returns the version of the `.KG` file.
	pub fn version(&self) -> u8 {
		self.version
	}

	/// Returns the compression type used in the `.KG` file.
	pub fn compression_type(&self) -> CompressionType {
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
	pub fn image_data_offset(&self) -> u32 {
		self.image_data_offset
	}

	/// Returns the total file size in bytes.
	pub fn file_size(&self) -> u32 {
		self.file_size
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
			0 => CompressionType::None,
			1 => CompressionType::RLE1Pass,
			_ => {
				return Err(KgError::UnsupportedCompressionType(data[3]));
			}
		};
		let width = u16::from_le_bytes([data[4], data[5]]);
		let height = u16::from_le_bytes([data[6], data[7]]);
		let mut reserved_1 = [0u8; 4];
		reserved_1.copy_from_slice(&data[8..12]);
		let palette_offset = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
		let image_data_offset = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
		let file_size = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
		let mut reserved_2 = [0u8; 8];
		reserved_2.copy_from_slice(&data[24..32]);

		Ok(Header {
			magic,
			version,
			compression_type,
			width,
			height,
			reserved_1,
			palette_offset,
			image_data_offset,
			file_size,
			reserved_2,
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
		bytes[16..20].copy_from_slice(&self.image_data_offset.to_le_bytes());
		bytes[20..24].copy_from_slice(&self.file_size.to_le_bytes());
		bytes[24..32].copy_from_slice(&self.reserved_2);

		bytes
	}
}

impl Default for Header {
	fn default() -> Self {
		Self {
			magic: constants::MAGIC,
			version: 1,
			compression_type: CompressionType::None,
			width: 0,
			height: 0,
			reserved_1: [0; 4],
			palette_offset: 0,
			image_data_offset: 0,
			file_size: 0,
			reserved_2: [0; 8],
		}
	}
}
