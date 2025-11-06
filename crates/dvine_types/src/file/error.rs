//! Error types for file format parsing and manipulation.
//!
//! This module provides a unified error handling system using [`DvFileError`]
//! for all file formats supported by dvine-rs.
//!
//! # Examples
//!
//! ```no_run
//! use dvine_types::file::{DvFileError, FileType, kg::File as KgFile};
//!
//! fn load_kg_file(path: &str) -> Result<KgFile, DvFileError> {
//!     KgFile::open(path)
//! }
//!
//! fn handle_error(err: DvFileError) {
//!     match err.file_type() {
//!         Some(FileType::Kg) => println!("KG file error: {}", err),
//!         Some(FileType::Pft) => println!("PFT file error: {}", err),
//!         _ => println!("File error: {}", err),
//!     }
//! }
//! ```

use thiserror::Error;

/// Unified error type for all file format operations
#[derive(Debug, Error)]
pub enum DvFileError {
	/// Not enough data to parse
	#[error("{file_type} error: Insufficient data (expected {expected} bytes, got {actual} bytes)")]
	InsufficientData {
		/// File type that encountered the error
		file_type: FileType,
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Invalid magic number
	#[error(
		"{file_type} error: Invalid magic number (expected {expected:02X?}, got {actual:02X?})"
	)]
	InvalidMagic {
		/// File type that encountered the error
		file_type: FileType,
		/// Expected magic bytes
		expected: Vec<u8>,
		/// Actual magic bytes
		actual: Vec<u8>,
	},

	/// Unsupported compression type (KG files)
	#[error("{file_type} error: Unsupported compression type {compression_type}")]
	UnsupportedCompressionType {
		/// File type that encountered the error
		file_type: FileType,
		/// Compression type value
		compression_type: u8,
	},

	/// Entry count mismatch (PFT files)
	#[error(
		"{file_type} error: Entry count mismatch (header specifies {expected}, found {actual})"
	)]
	EntryCountMismatch {
		/// File type that encountered the error
		file_type: FileType,
		/// Expected number of entries
		expected: u32,
		/// Actual number of entries
		actual: usize,
	},

	/// Block index out of range (DSK files)
	#[error("{file_type} error: Block index {index} out of range (total blocks: {total})")]
	BlockOutOfRange {
		/// File type that encountered the error
		file_type: FileType,
		/// Block index that was requested
		index: u32,
		/// Total number of blocks available
		total: usize,
	},

	/// Invalid file extraction (DSK files)
	#[error(
		"{file_type} error: Invalid extraction (requires {required} bytes, only {available} available)"
	)]
	InvalidExtraction {
		/// File type that encountered the error
		file_type: FileType,
		/// Number of bytes required
		required: usize,
		/// Number of bytes available
		available: usize,
	},

	/// File too large (DSK files)
	#[error(
		"{file_type} error: File too large (size {size} bytes needs {blocks_needed} blocks, only {blocks_available} available)"
	)]
	FileTooLarge {
		/// File type that encountered the error
		file_type: FileType,
		/// Size of the file in bytes
		size: usize,
		/// Number of blocks needed
		blocks_needed: usize,
		/// Number of blocks available
		blocks_available: usize,
	},

	/// Invalid opening mode (Startup.ini files)
	#[error("{file_type} error: Invalid opening mode value {value}")]
	InvalidOpeningMode {
		/// File type that encountered the error
		file_type: FileType,
		/// Invalid value
		value: u8,
	},

	/// Invalid VGA mode (Startup.ini files)
	#[error("{file_type} error: Invalid VGA mode value {value}")]
	InvalidVgaMode {
		/// File type that encountered the error
		file_type: FileType,
		/// Invalid value
		value: u8,
	},

	/// Invalid render mode (Startup.ini files)
	#[error("{file_type} error: Invalid render mode value {value}")]
	InvalidRenderMode {
		/// File type that encountered the error
		file_type: FileType,
		/// Invalid value
		value: u32,
	},

	/// Invalid font size (FNT files)
	#[error("{file_type} error: Invalid font size value {value}")]
	InvalidFontSize {
		/// File type that encountered the error
		file_type: FileType,
		/// Invalid value
		value: u32,
	},

	/// Character code out of range (FNT files)
	#[error("{file_type} error: Character code {code:04X} out of range (max {max_code:04X})")]
	CodeOutOfRange {
		/// File type that encountered the error
		file_type: FileType,
		/// Character code that was requested
		code: u16,
		/// Maximum valid character code
		max_code: u16,
	},

	/// Glyph already exists (FNT files)
	#[error("{file_type} error: Glyph for character code {code:04X} already exists")]
	GlyphAlreadyExists {
		/// File type that encountered the error
		file_type: FileType,
		/// Character code that already exists
		code: u16,
	},

	/// Checksum mismatch (ITEM files)
	#[error("{file_type} error: Checksum mismatch (expected {expected:08X}, got {actual:08X})")]
	ChecksumMismatch {
		/// File type that encountered the error
		file_type: FileType,
		/// Expected checksum value
		expected: u32,
		/// Actual calculated checksum
		actual: u32,
	},

	/// Invalid record count (ITEM files)
	#[error(
		"{file_type} error: Invalid record count ({total_bytes} bytes is not a multiple of {record_size})"
	)]
	InvalidRecordCount {
		/// File type that encountered the error
		file_type: FileType,
		/// Total bytes in the data section
		total_bytes: usize,
		/// Size of each record
		record_size: usize,
	},

	/// Buffer underflow during decompression (KG files)
	#[error("{file_type} error: Buffer underflow during decompression: {message}")]
	UnderflowError {
		/// File type that encountered the error
		file_type: FileType,
		/// Error message
		message: String,
	},

	/// Decompression error (KG files)
	#[error("{file_type} error: Decompression failed: {message}")]
	DecompressionError {
		/// File type that encountered the error
		file_type: FileType,
		/// Error message
		message: String,
	},

	/// Entry not found
	#[error("{file_type} error: Entry not found: {message}")]
	EntryNotFound {
		/// File type that encountered the error
		file_type: FileType,
		/// Error message
		message: String,
	},

	/// hound library error
	#[error(transparent)]
	HoundError(#[from] hound::Error),

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),

	/// Slice conversion error
	#[error(transparent)]
	TryFromSliceError(#[from] std::array::TryFromSliceError),
}

impl DvFileError {
	/// Returns the file type associated with this error
	pub fn file_type(&self) -> Option<FileType> {
		match self {
			Self::InsufficientData {
				file_type,
				..
			}
			| Self::InvalidMagic {
				file_type,
				..
			}
			| Self::UnsupportedCompressionType {
				file_type,
				..
			}
			| Self::EntryCountMismatch {
				file_type,
				..
			}
			| Self::BlockOutOfRange {
				file_type,
				..
			}
			| Self::InvalidExtraction {
				file_type,
				..
			}
			| Self::FileTooLarge {
				file_type,
				..
			}
			| Self::InvalidOpeningMode {
				file_type,
				..
			}
			| Self::InvalidVgaMode {
				file_type,
				..
			}
			| Self::InvalidRenderMode {
				file_type,
				..
			}
			| Self::InvalidFontSize {
				file_type,
				..
			}
			| Self::CodeOutOfRange {
				file_type,
				..
			}
			| Self::GlyphAlreadyExists {
				file_type,
				..
			}
			| Self::ChecksumMismatch {
				file_type,
				..
			}
			| Self::InvalidRecordCount {
				file_type,
				..
			}
			| Self::UnderflowError {
				file_type,
				..
			}
			| Self::DecompressionError {
				file_type,
				..
			}
			| Self::EntryNotFound {
				file_type,
				..
			} => Some(*file_type),
			_ => None,
		}
	}

	/// Returns true if this is an I/O error
	pub fn is_io_error(&self) -> bool {
		matches!(self, Self::IOError(_))
	}

	/// Returns true if this is an insufficient data error
	pub fn is_insufficient_data(&self) -> bool {
		matches!(self, Self::InsufficientData { .. })
	}

	/// Returns true if this is an invalid magic error
	pub fn is_invalid_magic(&self) -> bool {
		matches!(self, Self::InvalidMagic { .. })
	}

	/// Returns true if this is a decompression-related error
	pub fn is_decompression_error(&self) -> bool {
		matches!(
			self,
			Self::UnsupportedCompressionType { .. }
				| Self::UnderflowError { .. }
				| Self::DecompressionError { .. }
		)
	}

	/// Create an insufficient data error
	pub fn insufficient_data(file_type: FileType, expected: usize, actual: usize) -> Self {
		Self::InsufficientData {
			file_type,
			expected,
			actual,
		}
	}

	/// Create an invalid magic error
	pub fn invalid_magic(file_type: FileType, expected: &[u8], actual: &[u8]) -> Self {
		Self::InvalidMagic {
			file_type,
			expected: expected.to_vec(),
			actual: actual.to_vec(),
		}
	}
}

/// File type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
	/// PFT archive file
	Pft,
	/// DSK data file
	Dsk,
	/// EFC sound effect file
	Efc,
	/// Startup configuration file
	StartupIni,
	/// Font file
	Fnt,
	/// Item data file
	Item,
	/// KG image file
	Kg,
	/// MFD mouse cursor animation file
	Mfd,
	/// SPR sprite animation file
	Spr,
}

impl FileType {
	/// Returns the typical file extension for this file type
	pub fn extension(&self) -> &'static str {
		match self {
			FileType::Pft => "PFT",
			FileType::Dsk => "DSK",
			FileType::Efc => "EFC",
			FileType::StartupIni => "ini",
			FileType::Fnt => "FNT",
			FileType::Item => "dat",
			FileType::Kg => "",
			FileType::Mfd => "MFD",
			FileType::Spr => "SPR",
		}
	}

	/// Returns a human-readable description of this file type
	pub fn description(&self) -> &'static str {
		match self {
			FileType::Pft => "Archive index file",
			FileType::Dsk => "Archive data file",
			FileType::StartupIni => "Startup configuration",
			FileType::Fnt => "Font file",
			FileType::Item => "Item database",
			FileType::Kg => "Compressed image file",
			FileType::Efc => "Sound effect file",
			FileType::Mfd => "Mouse cursor animation file",
			FileType::Spr => "Sprite animation file",
		}
	}
}

impl std::fmt::Display for FileType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FileType::Pft => write!(f, "PFT"),
			FileType::Dsk => write!(f, "DSK"),
			FileType::Efc => write!(f, "EFC"),
			FileType::StartupIni => write!(f, "Startup.ini"),
			FileType::Fnt => write!(f, "FNT"),
			FileType::Item => write!(f, "ITEM"),
			FileType::Kg => write!(f, "KG"),
			FileType::Mfd => write!(f, "MFD"),
			FileType::Spr => write!(f, "SPR"),
		}
	}
}
