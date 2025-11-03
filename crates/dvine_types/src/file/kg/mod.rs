//! `.KG` file format support for `dvine-rs` project.

mod decode;
mod encode;

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
	/// Size of the header in bytes
	pub const SIZE: usize = constants::HEADER_SIZE;

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
	///
	/// This allows you to peek at the header without loading the entire file,
	/// which is useful for validation or determining file properties before
	/// deciding whether to decompress the full image.
	///
	/// # Example
	///
	/// ```no_run
	/// use dvine_types::file::kg::Header;
	/// use std::fs::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut file = File::open("image.kg")?;
	/// let header = Header::from_reader(&mut file)?;
	///
	/// // Check dimensions before loading full file
	/// if header.width() > 4096 || header.height() > 4096 {
	///     return Err("Image too large".into());
	/// }
	/// # Ok(())
	/// # }
	/// ```
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
		let decompress_data = decode::decompress(&data)?;

		Ok(Self {
			header: decompress_data.0,
			pixels: decompress_data.1,
		})
	}

	/// Creates a `.KG` file from any reader
	///
	/// Note: This reads the entire file into memory before decompression.
	/// The KG decompression algorithm requires random access to the data,
	/// so streaming decompression is not supported.
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, KgError> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		let decompress_data = decode::decompress(&data)?;

		Ok(Self {
			header: decompress_data.0,
			pixels: decompress_data.1,
		})
	}
}
