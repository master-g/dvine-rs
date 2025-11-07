//! `.KG` file format support for `dvine-rs` project.
//!
//! This module provides both compression and decompression support for the KG image format,
//! a proprietary format used in DirectDraw-based games from around 2000.
//!
//! # Features
//!
//! - **Decompression**: Full support for Type 1 (BPP3) compression
//! - **Compression**: Efficient encoding with LRU cache and copy operations
//! - **Palette Support**: 256-color indexed images with BGRA palette
//!
//! # Examples
//!
//! ## Loading and Decompressing a KG File
//!
//! ```no_run
//! use dvine_types::file::kg::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load from file
//! let kg_file = File::open("image.kg")?;
//!
//! // Access image properties
//! println!("Width: {}, Height: {}",
//!     kg_file.header().width(),
//!     kg_file.header().height());
//!
//! // Get RGB pixel data
//! let pixels = kg_file.pixels();
//! # Ok(())
//! # }
//! ```
//!
//! ## Compressing RGB Data to KG Format
//!
//! ```no_run
//! use dvine_types::file::kg::compress;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create RGB image data (width * height * 3 bytes)
//! let width = 16;
//! let height = 16;
//! let mut rgb_data = vec![0u8; (width * height * 3) as usize];
//!
//! // Fill with some pattern (e.g., red gradient)
//! for y in 0..height {
//!     for x in 0..width {
//!         let idx = ((y * width + x) * 3) as usize;
//!         rgb_data[idx] = ((x * 255) / width) as u8;     // R
//!         rgb_data[idx + 1] = 0;                          // G
//!         rgb_data[idx + 2] = 0;                          // B
//!     }
//! }
//!
//! // Compress to KG format
//! let compressed = compress(&rgb_data, width, height)?;
//!
//! // Save to file
//! std::fs::write("output.kg", compressed)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Round-trip Conversion
//!
//! ```no_run
//! use dvine_types::file::kg::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load, modify, and save
//! let kg_file = File::open("input.kg")?;
//! kg_file.save("output.kg")?;
//! # Ok(())
//! # }
//! ```
//!
//! # Limitations
//!
//! - Maximum 256 unique colors (8-bit indexed color)
//! - Only Type 1 (BPP3) compression is currently supported
//! - Images with more than 256 colors will fail to compress

mod decode;
pub mod encode;

pub use encode::{compress, compress_file};

use std::{fmt::Display, io::Read};

use crate::file::{DvFileError, FileType, kg::constants::MAGIC};

mod constants {
	/// Magic bytes for `.KG` files
	pub const MAGIC: [u8; 2] = [0x4B, 0x47]; // "KG"

	/// Header size for `.KG` files
	pub const HEADER_SIZE: usize = 32;
}

// Opcode definitions for the KG decompression algorithm
mod opcodes {
	/// Dictionary lookup: Read color index from bitstream or LRU cache
	pub const OP_DICT_LOOKUP: u8 = 0;
	/// Copy from the previous pixel (left or up depending on position)
	pub const OP_COPY_PREV_PIXEL: u8 = 2;
	/// Copy from one line up (same horizontal position)
	pub const OP_COPY_PREV_LINE: u8 = 12;
	/// Copy from one line up and one pixel right (diagonal up-right)
	pub const OP_COPY_DIAGONAL_1: u8 = 13;
	/// Copy from one line up and one pixel left (diagonal up-left)
	pub const OP_COPY_DIAGONAL_2: u8 = 14;
	/// Copy from two pixels back (for double BPP)
	pub const OP_COPY_DOUBLE_BPP: u8 = 15;
}

/// Compression types used in `.KG` files
/// TODO: move this to a more general location after implementing compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Compression {
	/// No compression
	Unsupported = 0,

	/// Image is encoded using dictionary lookup with LRU cache and various copy operations
	/// Uses 1 or 3 bytes per pixel depending on whether a palette is present
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
	pub fn from_bytes(data: &[u8]) -> Result<Header, DvFileError> {
		if data.len() < constants::HEADER_SIZE {
			return Err(DvFileError::insufficient_data(
				FileType::Kg,
				constants::HEADER_SIZE,
				data.len(),
			));
		}

		let magic = [data[0], data[1]];
		if magic != constants::MAGIC {
			return Err(DvFileError::invalid_magic(FileType::Kg, &constants::MAGIC, &magic));
		}

		let version = data[2];
		let compression_type = match data[3] {
			1 => Compression::BPP3,
			_ => {
				return Err(DvFileError::UnsupportedCompressionType {
					file_type: FileType::Kg,
					compression_type: data[3],
				});
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
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
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
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let data = std::fs::read(path)?;
		decode::decompress(&data)
	}

	/// Creates a `.KG` file from any reader
	///
	/// Note: This reads the entire file into memory before decompression.
	/// The KG decompression algorithm requires random access to the data,
	/// so streaming decompression is not supported.
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		decode::decompress(&data)
	}

	/// Saves the `.KG` file to the specified path
	///
	/// This compresses the pixel data and writes the complete KG file.
	///
	/// # Example
	///
	/// ```no_run
	/// use dvine_types::file::kg::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let kg_file = File::open("input.kg")?;
	/// kg_file.save("output.kg")?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), DvFileError> {
		let compressed = compress_file(self)?;
		std::fs::write(path, compressed)?;
		Ok(())
	}

	/// Compresses the `.KG` file to bytes
	///
	/// This is useful when you need the compressed data without writing to a file.
	///
	/// # Example
	///
	/// ```no_run
	/// use dvine_types::file::kg::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let kg_file = File::open("input.kg")?;
	/// let compressed_data = kg_file.to_bytes()?;
	/// // Use compressed_data...
	/// # Ok(())
	/// # }
	/// ```
	pub fn to_bytes(&self) -> Result<Vec<u8>, DvFileError> {
		compress_file(self)
	}
}
