//! Error types for file format parsing and manipulation.

use thiserror::Error;

/// Errors that can occur when parsing or manipulating PFT files
#[derive(Debug, Error)]
pub enum PftError {
	/// Not enough data to parse
	#[error("Insufficient data: expected {expected} bytes, got {actual} bytes")]
	InsufficientData {
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Invalid magic number
	#[error("Invalid magic number: {0:02X?}")]
	InvalidMagic([u8; 4]),

	/// Entry count mismatch
	#[error("Entry count mismatch: header specifies {expected} entries, but found {actual}")]
	EntryCountMismatch {
		/// Expected number of entries
		expected: u32,
		/// Actual number of entries
		actual: usize,
	},

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

/// Errors that can occur when parsing or manipulating DSK files
#[derive(Debug, Error)]
pub enum DskError {
	/// Not enough data
	#[error("Insufficient data: expected at least {expected} bytes, got {actual} bytes")]
	InsufficientData {
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Block index out of range
	#[error("Block index {index} out of range (total blocks: {total})")]
	BlockOutOfRange {
		/// Block index that was requested
		index: u32,
		/// Total number of blocks available
		total: usize,
	},

	/// Invalid file extraction
	#[error(
		"Invalid file extraction: entry requires {required} bytes, but only {available} bytes available"
	)]
	InvalidExtraction {
		/// Number of bytes required
		required: usize,
		/// Number of bytes available
		available: usize,
	},

	/// File too large for available space
	#[error(
		"File too large: size {size} bytes requires {blocks_needed} blocks, but only {blocks_available} blocks available"
	)]
	FileTooLarge {
		/// Size of the file in bytes
		size: usize,
		/// Number of blocks needed
		blocks_needed: usize,
		/// Number of blocks available
		blocks_available: usize,
	},

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),

	/// Invalid PFT
	#[error(transparent)]
	PFTError(#[from] PftError),
}

/// Errors that can occur when parsing or manipulating Startup INI files
#[derive(Debug, Error)]
pub enum StartupIniError {
	/// Not enough data to parse
	#[error("Insufficient data: expected {expected} bytes, got {actual} bytes")]
	InsufficientData {
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Invalid opening mode value
	#[error("Invalid opening mode value: {0}")]
	InvalidOpeningMode(u8),

	/// Invalid VGA mode value
	#[error("Invalid VGA mode value: {0}")]
	InvalidVgaMode(u8),

	/// Invalid render mode value
	#[error("Invalid render mode value: {0}")]
	InvalidRenderMode(u32),

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

/// Errors that can occur when parsing or manipulating FNT files
#[derive(Debug, Error)]
pub enum FntError {
	/// Not enough data to parse
	#[error("Insufficient data: expected {expected} bytes, got {actual} bytes")]
	InsufficientData {
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Invalid font size value
	#[error("Invalid font size value: {0}")]
	InvalidFontSize(u32),

	/// Character code out of range
	#[error("Character code {code:04X} out of range (max {max_code:04X})")]
	CodeOutOfRange {
		/// Character code that was requested
		code: u16,
		/// Maximum valid character code
		max_code: u16,
	},

	/// Glyph already exists
	#[error("Glyph for character code {code:04X} already exists")]
	GlyphAlreadyExists {
		/// Character code that already exists
		code: u16,
	},

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}

/// Errors that can occur when parsing or manipulating ITEM files
#[derive(Debug, Error)]
pub enum ItemError {
	/// Not enough data to parse
	#[error("Insufficient data: expected {expected} bytes, got {actual} bytes")]
	InsufficientData {
		/// Expected number of bytes
		expected: usize,
		/// Actual number of bytes
		actual: usize,
	},

	/// Checksum mismatch
	#[error("Checksum mismatch: expected {expected:08X}, got {actual:08X}")]
	ChecksumMismatch {
		/// Expected checksum value
		expected: u32,
		/// Actual calculated checksum
		actual: u32,
	},

	/// Invalid record count
	#[error("Invalid record count: {total_bytes} bytes is not a multiple of {record_size}")]
	InvalidRecordCount {
		/// Total bytes in the data section
		total_bytes: usize,
		/// Size of each record
		record_size: usize,
	},

	/// IO error
	#[error(transparent)]
	IOError(#[from] std::io::Error),
}
