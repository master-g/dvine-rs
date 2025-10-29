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
