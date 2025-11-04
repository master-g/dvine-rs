//! `.KG` file format support for `dvine-rs` project.

mod decode;
mod encode;

use std::{fmt::Display, io::Read};

use crate::file::{KgError, kg::constants::MAGIC};

mod constants {
	/// Magic bytes for `.KG` files
	pub const MAGIC: [u8; 2] = [0x4B, 0x47]; // "KG"

	/// Header size for `.KG` files
	pub const HEADER_SIZE: usize = 32;
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
	magic: [u8; 2],                // 0x00 - 0x01
	version: u8,                   // 0x02
	compression_type: Compression, // 0x03
	width: u16,                    // 0x04 - 0x05
	height: u16,                   // 0x06 - 0x07
	reserved_1: [u8; 4],           // 0x08 - 0x0B
	palette_offset: u32,           // 0x0C - 0x0F
	data_offset: u32,              // 0x10 - 0x13
	file_size: u32,                // 0x14 - 0x17
	reserved_2: [u32; 2],          // 0x18 - 0x1F
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
			file_size: 0,
			reserved_2: [0; 2],
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

	/// Returns the total file size in bytes.
	pub fn file_size(&self) -> u32 {
		self.file_size
	}

	/// Returns the size of the padding in bytes, if any.
	pub fn padding_size(&self) -> Option<usize> {
		if self.palette_offset > 0 && self.palette_offset - constants::HEADER_SIZE as u32 > 0 {
			Some((self.palette_offset - constants::HEADER_SIZE as u32) as usize)
		} else {
			None
		}
	}

	/// Returns true if the image has padding
	pub fn has_padding(&self) -> bool {
		self.padding_size().is_some()
	}

	/// Creates default padding bytes for the image
	pub fn create_default_padding(&self) -> Vec<u8> {
		#[repr(C)]
		struct DefaultPadding {
			zero_0: u32,
			width: u32,
			height: u32,
			zero_1: u32,
		}
		let padding = DefaultPadding {
			zero_0: 0,
			width: self.width as u32,
			height: self.height as u32,
			zero_1: 0,
		};

		let mut bytes = Vec::with_capacity(std::mem::size_of::<DefaultPadding>());
		bytes.extend_from_slice(&padding.zero_0.to_le_bytes());
		bytes.extend_from_slice(&padding.width.to_le_bytes());
		bytes.extend_from_slice(&padding.height.to_le_bytes());
		bytes.extend_from_slice(&padding.zero_1.to_le_bytes());

		bytes
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

		let file_size = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
		let mut reserved_2 = [0u32; 2];
		reserved_2[0] = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
		reserved_2[1] = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

		Ok(Header {
			magic,
			version,
			compression_type,
			width,
			height,
			reserved_1,
			palette_offset,
			data_offset,
			file_size,
			reserved_2,
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
			- Image Data Offset: {} bytes\n\
			- File Size: {} bytes\n",
			self.magic,
			self.version,
			self.compression_type,
			self.width,
			self.height,
			self.palette_offset,
			self.data_offset,
			self.file_size
		)
	}
}

type Plalette = [[u8; 4]; 256];

/// Representation of a decoded `.KG` file
#[derive(Debug)]
pub struct File {
	/// Header of the `.KG` file
	header: Header,

	/// Padding for alignment
	padding: Option<Vec<u8>>,

	/// Palette data, if present
	palette: Option<Plalette>,

	/// Pixel data of the `.KG` file, in RGB format
	pixels: Vec<u8>,
}

impl File {
	/// Returns a reference to the header of the `.KG` file
	pub fn header(&self) -> &Header {
		&self.header
	}

	/// Returns a reference to the padding of the `.KG` file, if present
	pub fn padding(&self) -> Option<&Vec<u8>> {
		self.padding.as_ref()
	}

	/// Returns a reference to the palette of the `.KG` file, if present
	/// Returns `None` if the file does not contain a palette
	pub fn palette(&self) -> Option<&Plalette> {
		self.palette.as_ref()
	}

	/// Returns a reference to the pixel data of the `.KG` file
	pub fn pixels(&self) -> &[u8] {
		&self.pixels
	}

	/// Opens and parses a `.KG` file from the specified path
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, KgError> {
		let data = std::fs::read(path)?;
		decode::decompress(&data)
	}

	/// Creates a `.KG` file from any reader
	///
	/// Note: This reads the entire file into memory before decompression.
	/// The KG decompression algorithm requires random access to the data,
	/// so streaming decompression is not supported.
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, KgError> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		decode::decompress(&data)
	}
}
