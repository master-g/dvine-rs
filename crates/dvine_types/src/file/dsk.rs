//! `.dsk` file format support.
//!
//! DSK files are block-based container files that store multiple files.
//! Each block is 2048 bytes (0x0800). The DSK file works in conjunction with
//! a PFT file which contains the metadata (file names, indices, sizes).

use std::io::{self, Read};

use super::{BLOCK_SIZE, error::DskError, pft};

/// Type alias for a single block
#[allow(dead_code)]
pub type Block = [u8; BLOCK_SIZE];

/// DSK File
///
/// A block-based container file that stores multiple files in fixed-size blocks.
/// Use with a PFT file to access individual files by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File {
	/// Raw data stored as blocks
	data: Vec<u8>,
}

impl File {
	/// Creates a new empty DSK file
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
		}
	}

	/// Creates a DSK file with pre-allocated capacity for the given number of blocks
	pub fn with_capacity(blocks: usize) -> Self {
		Self {
			data: Vec::with_capacity(blocks * BLOCK_SIZE),
		}
	}

	/// Loads DSK file from a byte slice
	pub fn from_bytes(data: &[u8]) -> Result<Self, DskError> {
		Ok(Self {
			data: data.to_vec(),
		})
	}

	/// Loads DSK file from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		Ok(Self {
			data,
		})
	}

	/// Returns the raw data as a byte slice
	pub fn as_bytes(&self) -> &[u8] {
		&self.data
	}

	/// Serializes the DSK file to bytes
	pub fn to_bytes(&self) -> Vec<u8> {
		self.data.clone()
	}

	/// Returns the total size in bytes
	pub fn size(&self) -> usize {
		self.data.len()
	}

	/// Returns the number of complete blocks
	pub fn num_blocks(&self) -> usize {
		self.data.len() / BLOCK_SIZE
	}

	/// Gets a specific block by index
	pub fn get_block(&self, index: usize) -> Result<&[u8], DskError> {
		let start = index * BLOCK_SIZE;
		let end = start + BLOCK_SIZE;

		if end > self.data.len() {
			return Err(DskError::BlockOutOfRange {
				index: index as u32,
				total: self.num_blocks(),
			});
		}

		Ok(&self.data[start..end])
	}

	/// Gets a mutable reference to a specific block by index
	pub fn get_block_mut(&mut self, index: usize) -> Result<&mut [u8], DskError> {
		let total_blocks = self.num_blocks();
		let start = index * BLOCK_SIZE;
		let end = start + BLOCK_SIZE;

		if end > self.data.len() {
			return Err(DskError::BlockOutOfRange {
				index: index as u32,
				total: total_blocks,
			});
		}

		Ok(&mut self.data[start..end])
	}

	/// Extracts a file from the DSK using a PFT entry
	///
	/// Returns the actual file data (trimmed to `actual_size`)
	pub fn extract_file(&self, entry: &pft::Entry) -> Result<Vec<u8>, DskError> {
		let start_block = entry.index as usize;
		let blocks_needed = entry.blocks_needed() as usize;
		let actual_size = entry.actual_size as usize;

		// Calculate byte range
		let start_byte = start_block * BLOCK_SIZE;
		let end_byte = start_byte + (blocks_needed * BLOCK_SIZE);

		// Validate range
		if end_byte > self.data.len() {
			return Err(DskError::InvalidExtraction {
				required: end_byte,
				available: self.data.len(),
			});
		}

		// Extract and trim to actual size
		let mut file_data = self.data[start_byte..end_byte].to_vec();
		file_data.truncate(actual_size);

		Ok(file_data)
	}

	/// Extracts a file at a specific block index with a given size
	///
	/// This is useful when you don't have a PFT entry
	pub fn extract_at(&self, block_index: u32, size: u32) -> Result<Vec<u8>, DskError> {
		let start_block = block_index as usize;
		let size = size as usize;
		let blocks_needed = size.div_ceil(BLOCK_SIZE);

		let start_byte = start_block * BLOCK_SIZE;
		let end_byte = start_byte + (blocks_needed * BLOCK_SIZE);

		if end_byte > self.data.len() {
			return Err(DskError::InvalidExtraction {
				required: end_byte,
				available: self.data.len(),
			});
		}

		let mut file_data = self.data[start_byte..end_byte].to_vec();
		file_data.truncate(size);

		Ok(file_data)
	}

	/// Adds a file to the DSK, padding to block boundaries
	///
	/// Returns the block index where the file was added and the actual size
	pub fn add_file(&mut self, file_data: &[u8]) -> (u32, u32) {
		let start_block = self.num_blocks() as u32;
		let actual_size = file_data.len() as u32;

		// Add file data
		self.data.extend_from_slice(file_data);

		// Pad to block boundary
		let padding_needed = BLOCK_SIZE - (file_data.len() % BLOCK_SIZE);
		if padding_needed < BLOCK_SIZE {
			self.data.extend(std::iter::repeat_n(0, padding_needed));
		}

		(start_block, actual_size)
	}

	/// Adds a file at a specific block index
	///
	/// This will expand the DSK if necessary and overwrite existing data
	pub fn add_file_at(&mut self, block_index: u32, file_data: &[u8]) -> Result<(), DskError> {
		let start_byte = block_index as usize * BLOCK_SIZE;
		let blocks_needed = file_data.len().div_ceil(BLOCK_SIZE);
		let total_bytes_needed = start_byte + blocks_needed * BLOCK_SIZE;

		// Expand if necessary
		if self.data.len() < total_bytes_needed {
			self.data.resize(total_bytes_needed, 0);
		}

		// Write file data
		let end_byte = start_byte + file_data.len();
		self.data[start_byte..end_byte].copy_from_slice(file_data);

		// Ensure padding to block boundary
		let padding_start = end_byte;
		let padding_end = start_byte + (blocks_needed * BLOCK_SIZE);
		for byte in &mut self.data[padding_start..padding_end] {
			*byte = 0;
		}

		Ok(())
	}

	/// Clears all data from the DSK file
	pub fn clear(&mut self) {
		self.data.clear();
	}

	/// Reserves capacity for at least `additional` more blocks
	pub fn reserve(&mut self, additional_blocks: usize) {
		self.data.reserve(additional_blocks * BLOCK_SIZE);
	}

	/// Pads the DSK file to the next block boundary if not already aligned
	pub fn pad_to_block_boundary(&mut self) {
		let remainder = self.data.len() % BLOCK_SIZE;
		if remainder != 0 {
			let padding_needed = BLOCK_SIZE - remainder;
			self.data.extend(std::iter::repeat_n(0, padding_needed));
		}
	}

	/// Validates that the DSK file can accommodate all entries from a PFT file
	pub fn validate_with_pft(&self, pft: &pft::File) -> Result<(), DskError> {
		for entry in pft.entries().iter() {
			let start_block = entry.index as usize;
			let blocks_needed = entry.blocks_needed() as usize;
			let end_byte = (start_block + blocks_needed) * BLOCK_SIZE;

			if end_byte > self.data.len() {
				return Err(DskError::InvalidExtraction {
					required: end_byte,
					available: self.data.len(),
				});
			}
		}
		Ok(())
	}
}

impl Default for File {
	fn default() -> Self {
		Self::new()
	}
}

impl TryFrom<&[u8]> for File {
	type Error = DskError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for File {
	type Error = DskError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self {
			data: value,
		})
	}
}

impl TryFrom<&Vec<u8>> for File {
	type Error = DskError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<File> for Vec<u8> {
	fn from(file: File) -> Self {
		file.data
	}
}

impl From<&File> for Vec<u8> {
	fn from(file: &File) -> Self {
		file.to_bytes()
	}
}
