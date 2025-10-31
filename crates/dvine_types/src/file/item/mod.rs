//! `ITEM.dat` file type support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating item data files used in the `D+VINE[LUV]`
//! visual novel engine. The item files use a custom encrypted format with obfuscated checksum.
//!
//! # File Format
//!
//! The ITEM.dat file has the following structure:
//!
//! ```text
//! ┌─────────────────────────────────┐
//! │ Item Count (2 bytes, LE)        │  0x00-0x01
//! ├─────────────────────────────────┤
//! │ Encrypted Item Data             │  0x02 onwards
//! │ (item_count × 208 bytes)        │
//! ├─────────────────────────────────┤
//! │ Checksum Data (128 bytes)       │  End of file
//! └─────────────────────────────────┘
//! ```
//!
//! ## Checksum Algorithm
//!
//! The 32-bit checksum is scattered across a 128-byte buffer:
//! - Byte 0x00: bits [31:24]
//! - Byte 0x07: bits [23:16]
//! - Byte 0x29: bits [15:8]
//! - Byte 0x41: bits [7:0]
//!
//! The checksum is a simple sum of all decrypted data bytes.
//!
//! # Usage Examples
//!
//! ## Loading Item Data
//!
//! ```no_run
//! use dvine_types::file::item::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let items = File::open("ITEM.dat")?;
//!
//! println!("Loaded {} items", items.item_count());
//!
//! // Access a specific item
//! if let Some(item) = items.get_item(0) {
//!     println!("First byte of item data: {:02X}", item[0]);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Working with Decrypted Data
//!
//! ```no_run
//! use dvine_types::file::item::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let items = File::open("ITEM.dat")?;
//!
//! // Iterate over all items
//! for (index, item_data) in items.iter().enumerate() {
//!     println!("Item {}: {} bytes", index, item_data.len());
//! }
//! # Ok(())
//! # }
//! ```

use crate::file::error::ItemError;

pub mod entry;

/// Item file constants.
pub mod constants {
	/// Size of the item count field in bytes (2 bytes, little-endian)
	pub const ITEM_COUNT_SIZE: usize = 2;

	/// Size of each item record in bytes
	pub const ITEM_SIZE: usize = 208;

	/// Size of the checksum buffer at the end of file
	pub const CHECKSUM_BUFFER_SIZE: usize = 128;

	/// Checksum byte offsets within the 128-byte buffer
	pub const CHECKSUM_OFFSETS: [usize; 4] = [0x00, 0x07, 0x29, 0x41];
}

/// Decryption table used to decrypt item data.
const DECODE_TABLE: [u8; 0x100] = [
	0x2A, 0x50, 0x75, 0x7B, 0x45, 0x90, 0xEE, 0x9F, 0xF0, 0xED, 0x85, 0x2D, 0xF6, 0xDB, 0x64, 0x65,
	0xC6, 0xAD, 0x6E, 0x83, 0x26, 0x8E, 0xD8, 0xE3, 0x5C, 0xB4, 0x14, 0xD9, 0x31, 0xF5, 0xE9, 0x6D,
	0x84, 0xB3, 0xEC, 0xA9, 0xFE, 0x7A, 0x4C, 0x09, 0x1A, 0xCA, 0x3A, 0x51, 0x63, 0x68, 0xB1, 0xF7,
	0x02, 0x91, 0xDC, 0xC7, 0xD0, 0xA4, 0x0B, 0x2E, 0xF1, 0xAF, 0xAB, 0x77, 0x40, 0x3F, 0x01, 0x1E,
	0x54, 0xE7, 0x7D, 0x76, 0x21, 0xB8, 0x4B, 0x07, 0xBC, 0xA5, 0x6A, 0x8D, 0x69, 0x35, 0x22, 0xCD,
	0xA7, 0x5E, 0xA3, 0x49, 0x8C, 0x3E, 0x36, 0x57, 0x8F, 0xD3, 0x06, 0xFB, 0xE2, 0xC9, 0x6F, 0xFF,
	0x7C, 0xDD, 0x41, 0x25, 0xBB, 0xBE, 0x6C, 0x78, 0x66, 0x71, 0xE8, 0xD7, 0x39, 0xF4, 0x1B, 0xCB,
	0x6B, 0x7E, 0xD1, 0xA6, 0xCE, 0x23, 0x20, 0x56, 0x53, 0x59, 0x1F, 0x87, 0xF3, 0x3D, 0xE0, 0x2B,
	0xCC, 0x5A, 0x82, 0x97, 0xC5, 0xC4, 0xD5, 0xB5, 0xFA, 0xDE, 0x0F, 0x47, 0x28, 0x9E, 0x08, 0x5B,
	0xB7, 0x27, 0xF8, 0xC0, 0x37, 0xDF, 0x38, 0x2F, 0x32, 0x15, 0x72, 0xBD, 0x2C, 0x74, 0x9A, 0x0A,
	0x24, 0x86, 0xE1, 0xC3, 0x04, 0x3C, 0xBA, 0x10, 0x9B, 0x13, 0x79, 0xB2, 0xE6, 0x9C, 0xBF, 0xD4,
	0x93, 0xDA, 0xA1, 0x5F, 0x12, 0xCF, 0xB9, 0xE4, 0x17, 0x00, 0x16, 0x29, 0x98, 0x1C, 0xA2, 0xC1,
	0x7F, 0xEB, 0x4E, 0x52, 0xA8, 0xEF, 0x96, 0x48, 0x03, 0xD6, 0x4D, 0x30, 0x80, 0xB6, 0x1D, 0xF9,
	0x43, 0x5D, 0xD2, 0x67, 0x42, 0x95, 0x11, 0xE5, 0x19, 0x73, 0x99, 0x9D, 0x44, 0xFD, 0x0C, 0xC2,
	0xAE, 0x89, 0x3B, 0x05, 0x88, 0x81, 0x61, 0xFC, 0x4F, 0x92, 0x62, 0xAC, 0xB0, 0x18, 0x8A, 0x33,
	0x0D, 0xC8, 0xAA, 0x0E, 0x58, 0xEA, 0xA0, 0x70, 0x46, 0x4A, 0x94, 0xF2, 0x60, 0x55, 0x34, 0x8B,
];

/// Encryption table (inverse of `DECODE_TABLE`).
const ENCODE_TABLE: [u8; 0x100] = [
	0xB9, 0x3E, 0x30, 0xC8, 0xA4, 0xE3, 0x5A, 0x47, 0x8E, 0x27, 0x9F, 0x36, 0xDE, 0xF0, 0xF3, 0x8A,
	0xA7, 0xD6, 0xB4, 0xA9, 0x1A, 0x99, 0xBA, 0xB8, 0xED, 0xD8, 0x28, 0x6E, 0xBD, 0xCE, 0x3F, 0x7A,
	0x76, 0x44, 0x4E, 0x75, 0xA0, 0x63, 0x14, 0x91, 0x8C, 0xBB, 0x00, 0x7F, 0x9C, 0x0B, 0x37, 0x97,
	0xCB, 0x1C, 0x98, 0xEF, 0xFE, 0x4D, 0x56, 0x94, 0x96, 0x6C, 0x2A, 0xE2, 0xA5, 0x7D, 0x55, 0x3D,
	0x3C, 0x62, 0xD4, 0xD0, 0xDC, 0x04, 0xF8, 0x8B, 0xC7, 0x53, 0xF9, 0x46, 0x26, 0xCA, 0xC2, 0xE8,
	0x01, 0x2B, 0xC3, 0x78, 0x40, 0xFD, 0x77, 0x57, 0xF4, 0x79, 0x81, 0x8F, 0x18, 0xD1, 0x51, 0xB3,
	0xFC, 0xE6, 0xEA, 0x2C, 0x0E, 0x0F, 0x68, 0xD3, 0x2D, 0x4C, 0x4A, 0x70, 0x66, 0x1F, 0x12, 0x5E,
	0xF7, 0x69, 0x9A, 0xD9, 0x9D, 0x02, 0x43, 0x3B, 0x67, 0xAA, 0x25, 0x03, 0x60, 0x42, 0x71, 0xC0,
	0xCC, 0xE5, 0x82, 0x13, 0x20, 0x0A, 0xA1, 0x7B, 0xE4, 0xE1, 0xEE, 0xFF, 0x54, 0x4B, 0x15, 0x58,
	0x05, 0x31, 0xE9, 0xB0, 0xFA, 0xD5, 0xC6, 0x83, 0xBC, 0xDA, 0x9E, 0xA8, 0xAD, 0xDB, 0x8D, 0x07,
	0xF6, 0xB2, 0xBE, 0x52, 0x35, 0x49, 0x73, 0x50, 0xC4, 0x23, 0xF2, 0x3A, 0xEB, 0x11, 0xE0, 0x39,
	0xEC, 0x2E, 0xAB, 0x21, 0x19, 0x87, 0xCD, 0x90, 0x45, 0xB6, 0xA6, 0x64, 0x48, 0x9B, 0x65, 0xAE,
	0x93, 0xBF, 0xDF, 0xA3, 0x85, 0x84, 0x10, 0x33, 0xF1, 0x5D, 0x29, 0x6F, 0x80, 0x4F, 0x74, 0xB5,
	0x34, 0x72, 0xD2, 0x59, 0xAF, 0x86, 0xC9, 0x6B, 0x16, 0x1B, 0xB1, 0x0D, 0x32, 0x61, 0x89, 0x95,
	0x7E, 0xA2, 0x5C, 0x17, 0xB7, 0xD7, 0xAC, 0x41, 0x6A, 0x1E, 0xF5, 0xC1, 0x22, 0x09, 0x06, 0xC5,
	0x08, 0x38, 0xFB, 0x7C, 0x6D, 0x1D, 0x0C, 0x2F, 0x92, 0xCF, 0x88, 0x5B, 0xE7, 0xDD, 0x24, 0x5F,
];

/// Represents a single item record from ITEM.dat (208 bytes).
///
/// For now, we use a simple byte array to represent the item data.
/// The actual structure will be decoded as needed based on the documentation.
pub type ItemRaw = [u8; constants::ITEM_SIZE];

/// Item data file structure, representing a complete ITEM.dat file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// Number of items in the file
	item_count: u16,

	/// Decrypted item data (`item_count` × 208 bytes)
	items: Vec<ItemRaw>,
}

impl File {
	/// Creates a new empty Item File.
	pub fn new() -> Self {
		Self {
			item_count: 0,
			items: Vec::new(),
		}
	}

	/// Opens an item data file from the specified path.
	///
	/// This function:
	/// 1. Reads the file
	/// 2. Parses the item count
	/// 3. Decrypts the item data
	/// 4. Validates the checksum
	///
	/// # Arguments
	///
	/// * `path` - Path to the item data file
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The file is too small
	/// - The checksum validation fails
	/// - The data size is not a multiple of the item size
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, ItemError> {
		use std::io::Read;

		let mut file = std::fs::File::open(path)?;
		let mut data = Vec::new();
		file.read_to_end(&mut data)?;

		Self::from_bytes(&data)
	}

	/// Parses item data from a byte slice.
	///
	/// # File Structure
	///
	/// ```text
	/// [Item Count: 2 bytes] [Encrypted Data: N × 208 bytes] [Checksum Buffer: 128 bytes]
	/// ```
	///
	/// # Arguments
	///
	/// * `data` - Raw file data
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The data is too small
	/// - The checksum validation fails
	/// - The data size is not a multiple of the item size
	pub fn from_bytes(data: &[u8]) -> Result<Self, ItemError> {
		// Validate minimum size: item_count + checksum_buffer
		let min_size = constants::ITEM_COUNT_SIZE + constants::CHECKSUM_BUFFER_SIZE;
		if data.len() < min_size {
			return Err(ItemError::InsufficientData {
				expected: min_size,
				actual: data.len(),
			});
		}

		// Parse item count (2 bytes, little-endian)
		let item_count = u16::from_le_bytes([data[0], data[1]]);

		// Calculate expected data size
		let data_size = (item_count as usize) * constants::ITEM_SIZE;
		let total_expected =
			constants::ITEM_COUNT_SIZE + data_size + constants::CHECKSUM_BUFFER_SIZE;

		if data.len() < total_expected {
			return Err(ItemError::InsufficientData {
				expected: total_expected,
				actual: data.len(),
			});
		}

		// Extract encrypted data section
		let encrypted_data =
			&data[constants::ITEM_COUNT_SIZE..constants::ITEM_COUNT_SIZE + data_size];

		// Decrypt data and calculate checksum
		let mut decrypted = Vec::with_capacity(data_size);
		let mut checksum: u32 = 0;

		for &byte in encrypted_data {
			let decrypted_byte = DECODE_TABLE[byte as usize];
			decrypted.push(decrypted_byte);
			checksum = checksum.wrapping_add(decrypted_byte as u32);
		}

		// Extract and reconstruct checksum from 128-byte buffer
		let checksum_offset = constants::ITEM_COUNT_SIZE + data_size;
		let checksum_buffer =
			&data[checksum_offset..checksum_offset + constants::CHECKSUM_BUFFER_SIZE];

		let file_checksum = ((checksum_buffer[constants::CHECKSUM_OFFSETS[0]] as u32) << 24)
			| ((checksum_buffer[constants::CHECKSUM_OFFSETS[1]] as u32) << 16)
			| ((checksum_buffer[constants::CHECKSUM_OFFSETS[2]] as u32) << 8)
			| (checksum_buffer[constants::CHECKSUM_OFFSETS[3]] as u32);

		// Validate checksum
		if checksum != file_checksum {
			return Err(ItemError::ChecksumMismatch {
				expected: file_checksum,
				actual: checksum,
			});
		}

		// Convert decrypted bytes into Item array
		if decrypted.len() % constants::ITEM_SIZE != 0 {
			return Err(ItemError::InvalidRecordCount {
				total_bytes: decrypted.len(),
				record_size: constants::ITEM_SIZE,
			});
		}

		let mut items = Vec::with_capacity(item_count as usize);
		for chunk in decrypted.chunks_exact(constants::ITEM_SIZE) {
			let mut item = [0u8; constants::ITEM_SIZE];
			item.copy_from_slice(chunk);
			items.push(item);
		}

		Ok(Self {
			item_count,
			items,
		})
	}

	/// Returns the number of items in the file.
	pub fn item_count(&self) -> u16 {
		self.item_count
	}

	/// Returns a reference to the item at the specified index.
	///
	/// # Arguments
	///
	/// * `index` - Zero-based item index
	///
	/// # Returns
	///
	/// `Some(&Item)` if the index is valid, `None` otherwise.
	pub fn get_item(&self, index: usize) -> Option<&ItemRaw> {
		self.items.get(index)
	}

	/// Returns a mutable reference to the item at the specified index.
	///
	/// # Arguments
	///
	/// * `index` - Zero-based item index
	///
	/// # Returns
	///
	/// `Some(&mut Item)` if the index is valid, `None` otherwise.
	pub fn get_item_mut(&mut self, index: usize) -> Option<&mut ItemRaw> {
		self.items.get_mut(index)
	}

	/// Returns an iterator over all items.
	pub fn iter(&self) -> impl Iterator<Item = &ItemRaw> {
		self.items.iter()
	}

	/// Returns a mutable iterator over all items.
	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ItemRaw> {
		self.items.iter_mut()
	}

	/// Serializes the item file back to bytes.
	///
	/// This encrypts the data and generates a new checksum.
	pub fn to_bytes(&self) -> Vec<u8> {
		let data_size = (self.item_count as usize) * constants::ITEM_SIZE;
		let total_size = constants::ITEM_COUNT_SIZE + data_size + constants::CHECKSUM_BUFFER_SIZE;
		let mut buffer = Vec::with_capacity(total_size);

		// Write item count
		buffer.extend_from_slice(&self.item_count.to_le_bytes());

		// Encrypt data and calculate checksum
		let mut checksum: u32 = 0;
		for item in &self.items {
			for &byte in item {
				let encrypted_byte = ENCODE_TABLE[byte as usize];
				buffer.push(encrypted_byte);
				checksum = checksum.wrapping_add(byte as u32);
			}
		}

		// Create checksum buffer (128 bytes)
		let mut checksum_buffer = vec![0u8; constants::CHECKSUM_BUFFER_SIZE];
		checksum_buffer[constants::CHECKSUM_OFFSETS[0]] = ((checksum >> 24) & 0xFF) as u8;
		checksum_buffer[constants::CHECKSUM_OFFSETS[1]] = ((checksum >> 16) & 0xFF) as u8;
		checksum_buffer[constants::CHECKSUM_OFFSETS[2]] = ((checksum >> 8) & 0xFF) as u8;
		checksum_buffer[constants::CHECKSUM_OFFSETS[3]] = (checksum & 0xFF) as u8;

		buffer.extend_from_slice(&checksum_buffer);

		buffer
	}

	/// Adds a new item to the file.
	///
	/// # Arguments
	///
	/// * `item` - The item data to add
	pub fn add_item(&mut self, item: ItemRaw) {
		self.items.push(item);
		self.item_count += 1;
	}

	/// Removes the item at the specified index.
	///
	/// # Arguments
	///
	/// * `index` - Zero-based item index
	///
	/// # Returns
	///
	/// `Some(Item)` if the index was valid, `None` otherwise.
	pub fn remove_item(&mut self, index: usize) -> Option<ItemRaw> {
		if index < self.items.len() {
			self.item_count -= 1;
			Some(self.items.remove(index))
		} else {
			None
		}
	}
}

impl Default for File {
	fn default() -> Self {
		Self::new()
	}
}

impl TryFrom<&[u8]> for File {
	type Error = ItemError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for File {
	type Error = ItemError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl From<File> for Vec<u8> {
	fn from(file: File) -> Self {
		file.to_bytes()
	}
}

impl From<&File> for Vec<u8> {
	fn from(file: &File) -> Self {
		file.to_bytes()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Item File: {} items", self.item_count)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_decode_encode_table_bijection() {
		// Verify that ENCODE_TABLE is the inverse of DECODE_TABLE
		for i in 0..256 {
			let encrypted = i as u8;
			let decrypted = DECODE_TABLE[encrypted as usize];
			let re_encrypted = ENCODE_TABLE[decrypted as usize];
			assert_eq!(
				encrypted, re_encrypted,
				"Bijection failed at encrypted={:02X}, decrypted={:02X}",
				encrypted, decrypted
			);
		}
	}

	#[test]
	fn test_empty_file() {
		let file = File::new();
		assert_eq!(file.item_count(), 0);
		assert_eq!(file.get_item(0), None);
	}

	#[test]
	fn test_add_remove_item() {
		let mut file = File::new();

		let item = [0xAA; constants::ITEM_SIZE];
		file.add_item(item);

		assert_eq!(file.item_count(), 1);
		assert_eq!(file.get_item(0), Some(&item));

		let removed = file.remove_item(0);
		assert_eq!(removed, Some(item));
		assert_eq!(file.item_count(), 0);
	}

	#[test]
	fn test_serialization_roundtrip() {
		let mut file = File::new();

		// Add some test items
		let item1 = [0x01; constants::ITEM_SIZE];
		let item2 = [0x02; constants::ITEM_SIZE];
		file.add_item(item1);
		file.add_item(item2);

		// Serialize
		let bytes = file.to_bytes();

		// Deserialize
		let loaded = File::from_bytes(&bytes).expect("Failed to deserialize");

		// Verify
		assert_eq!(loaded.item_count(), file.item_count());
		assert_eq!(loaded.get_item(0), Some(&item1));
		assert_eq!(loaded.get_item(1), Some(&item2));
	}

	#[test]
	fn test_checksum_validation() {
		let mut file = File::new();
		let item = [0xFF; constants::ITEM_SIZE];
		file.add_item(item);

		let mut bytes = file.to_bytes();

		// Corrupt the checksum
		let checksum_offset = constants::ITEM_COUNT_SIZE + constants::ITEM_SIZE;
		bytes[checksum_offset + constants::CHECKSUM_OFFSETS[0]] ^= 0xFF;

		// Should fail checksum validation
		let result = File::from_bytes(&bytes);
		assert!(result.is_err());
		assert!(matches!(result, Err(ItemError::ChecksumMismatch { .. })));
	}

	#[test]
	fn test_insufficient_data() {
		let data = vec![0u8; 10]; // Too small
		let result = File::from_bytes(&data);
		assert!(result.is_err());
		assert!(matches!(result, Err(ItemError::InsufficientData { .. })));
	}

	#[test]
	fn test_iterator() {
		let mut file = File::new();
		file.add_item([0x01; constants::ITEM_SIZE]);
		file.add_item([0x02; constants::ITEM_SIZE]);
		file.add_item([0x03; constants::ITEM_SIZE]);

		let items: Vec<_> = file.iter().collect();
		assert_eq!(items.len(), 3);
		assert_eq!(items[0][0], 0x01);
		assert_eq!(items[1][0], 0x02);
		assert_eq!(items[2][0], 0x03);
	}
}
