//! `.MFD` file format support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating MFD (Mouse File Data) files
//! used in the `D+VINE[LUV]` visual novel engine. MFD files contain animated mouse cursor
//! frames with metadata including dimensions, hotspot offsets, and indexed pixel data.
//!
//! # File Structure
//!
//! The MFD file format consists of:
//! - **Header (0x00-0x0F):** File metadata including frame count at offset 0x08
//! - **Bitmap Data (0x10-0x33DB):** Raw pixel data for all frames
//! - **Glyph Table (0x33DC+):** Frame metadata entries (12 bytes each)
//!
//! Each glyph table entry contains:
//! - Width (2 bytes, little-endian)
//! - Height (2 bytes, little-endian)
//! - X offset / hotspot (2 bytes, signed little-endian)
//! - Y offset / hotspot (2 bytes, signed little-endian)
//! - Bitmap offset (4 bytes, little-endian, relative to 0x10)
//!
//! # Pixel Format
//!
//! Pixels are stored as indexed bytes:
//! - `0`: Transparent
//! - `1`: Outline color
//! - Other values: Fill color
//!
//! # Usage Examples
//!
//! ## Loading an MFD file
//!
//! ```no_run
//! use dvine_types::file::mfd::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mfd = File::open("DXMSTEST.MFD")?;
//!
//! println!("Total frames: {}", mfd.frame_count());
//!
//! // Get a specific frame
//! if let Some(frame) = mfd.get_frame(0) {
//!     println!("Frame 0: {}x{}", frame.width(), frame.height());
//!     println!("Hotspot: ({}, {})", frame.x_offset(), frame.y_offset());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Iterating over frames
//!
//! ```no_run
//! use dvine_types::file::mfd::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mfd = File::open("DXMSTEST.MFD")?;
//!
//! for (index, frame) in mfd.iter().enumerate() {
//!     println!("Frame #{}: {}", index, frame);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Exporting frames
//!
//! ```no_run
//! use dvine_types::file::mfd::File;
//! use std::fs;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mfd = File::open("DXMSTEST.MFD")?;
//!
//! // Export a frame as PGM
//! if let Some(frame) = mfd.get_frame(0) {
//!     let pgm_data = frame.to_pgm();
//!     fs::write("frame_00.pgm", pgm_data)?;
//! }
//!
//! // Export as ASCII art
//! if let Some(frame) = mfd.get_frame(1) {
//!     println!("{}", frame.to_ascii_art_default());
//! }
//! # Ok(())
//! # }
//! ```

use std::io::Cursor;

use crate::file::{DvFileError, FileType};

pub mod frame;

pub use frame::{Frame, FrameEntry, FrameRowIterator};

/// MFD file constants.
pub mod constants {
	/// Fixed offset where bitmap data starts (0x10)
	pub const BITMAP_DATA_START: usize = 0x10;

	/// Fixed offset where the glyph table starts (0x33DC)
	pub const GLYPH_TABLE_OFFSET: usize = 0x33DC;

	/// Size of each glyph table entry in bytes
	pub const GLYPH_ENTRY_SIZE: usize = 12;

	/// Offset of frame count in the header
	pub const FRAME_COUNT_OFFSET: usize = 0x08;
}

/// MFD file structure, representing a complete mouse cursor animation file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// Complete file data
	raw: Vec<u8>,

	/// Number of frames in the file
	frame_count: u32,

	/// Frame entries (glyph table)
	entries: Vec<FrameEntry>,
}

impl File {
	/// Creates a new empty MFD file.
	pub fn new() -> Self {
		Self {
			raw: Vec::new(),
			frame_count: 0,
			entries: Vec::new(),
		}
	}

	/// Opens an MFD file from the specified path.
	///
	/// # Arguments
	///
	/// * `path` - Path to the MFD file.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The file is too small to contain required headers
	/// - The glyph table is invalid
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let data = std::fs::read(path)?;
		Self::from_bytes(&data)
	}

	/// Returns the number of frames in the file.
	pub fn frame_count(&self) -> u32 {
		self.frame_count
	}

	/// Returns a reference to the frame entries.
	pub fn entries(&self) -> &[FrameEntry] {
		&self.entries
	}

	/// Returns a specific frame entry by index.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// The frame entry, or None if the index is out of range.
	pub fn get_entry(&self, index: usize) -> Option<&FrameEntry> {
		self.entries.get(index)
	}

	/// Gets a complete frame (entry + pixel data) by index.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// The complete frame with pixel data, or None if the index is out of range
	/// or the bitmap data is invalid.
	///
	/// # Pixel Values
	///
	/// - `0`: Transparent pixel
	/// - `1`: Outline pixel
	/// - Other values (typically `2`): Fill pixel
	pub fn get_frame(&self, index: usize) -> Option<Frame> {
		let entry = self.entries.get(index)?;

		// Calculate absolute bitmap offset
		let bitmap_start = constants::BITMAP_DATA_START + entry.bitmap_offset as usize;
		let pixel_count = entry.pixel_count();
		let bitmap_end = bitmap_start + pixel_count;

		// Validate bitmap range
		if bitmap_end > self.raw.len() {
			return None;
		}

		// Extract pixel data
		let pixels = self.raw[bitmap_start..bitmap_end].to_vec();

		Some(Frame::new(*entry, pixels))
	}

	/// Returns an iterator over all frames in the file.
	pub fn iter(&self) -> FrameIterator<'_> {
		FrameIterator {
			file: self,
			current_index: 0,
		}
	}

	/// Serializes the MFD file to bytes.
	///
	/// Note: This preserves the original raw data structure.
	pub fn to_bytes(&self) -> Vec<u8> {
		self.raw.clone()
	}

	/// Loads an MFD file from a byte slice.
	///
	/// # Arguments
	///
	/// * `data` - Raw file data
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file is too small to contain the header
	/// - The glyph table offset is beyond the file size
	/// - Frame entries are invalid
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let mut cursor = Cursor::new(data);
		Self::from_reader(&mut cursor)
	}

	/// Loads an MFD file from any reader.
	///
	/// # Arguments
	///
	/// * `reader` - Data reader
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Not enough data can be read
	/// - The file structure is invalid
	pub fn from_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, DvFileError> {
		// Read entire file
		let mut raw = Vec::new();
		reader.read_to_end(&mut raw)?;

		// Validate minimum file size (header + at least glyph table start)
		if raw.len() < constants::GLYPH_TABLE_OFFSET {
			return Err(DvFileError::insufficient_data(
				FileType::Mfd,
				constants::GLYPH_TABLE_OFFSET,
				raw.len(),
			));
		}

		// Read frame count from offset 0x08
		if raw.len() < constants::FRAME_COUNT_OFFSET + 4 {
			return Err(DvFileError::insufficient_data(
				FileType::Mfd,
				constants::FRAME_COUNT_OFFSET + 4,
				raw.len(),
			));
		}

		let frame_count = u32::from_le_bytes([
			raw[constants::FRAME_COUNT_OFFSET],
			raw[constants::FRAME_COUNT_OFFSET + 1],
			raw[constants::FRAME_COUNT_OFFSET + 2],
			raw[constants::FRAME_COUNT_OFFSET + 3],
		]);

		// Parse glyph table entries
		let mut entries = Vec::with_capacity(frame_count as usize);
		for i in 0..frame_count as usize {
			let offset = constants::GLYPH_TABLE_OFFSET + i * constants::GLYPH_ENTRY_SIZE;

			// Validate we have enough data for this entry
			if raw.len() < offset + constants::GLYPH_ENTRY_SIZE {
				return Err(DvFileError::insufficient_data(
					FileType::Mfd,
					offset + constants::GLYPH_ENTRY_SIZE,
					raw.len(),
				));
			}

			// Parse entry fields
			let width = u16::from_le_bytes([raw[offset], raw[offset + 1]]);
			let height = u16::from_le_bytes([raw[offset + 2], raw[offset + 3]]);
			let x_offset = i16::from_le_bytes([raw[offset + 4], raw[offset + 5]]);
			let y_offset = i16::from_le_bytes([raw[offset + 6], raw[offset + 7]]);
			let bitmap_offset = u32::from_le_bytes([
				raw[offset + 8],
				raw[offset + 9],
				raw[offset + 10],
				raw[offset + 11],
			]);

			entries.push(FrameEntry::new(width, height, x_offset, y_offset, bitmap_offset));
		}

		Ok(Self {
			raw,
			frame_count,
			entries,
		})
	}
}

impl Default for File {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "MFD File: {} frames, {} bytes", self.frame_count, self.raw.len())
	}
}

/// Iterator over frames in an MFD file.
#[derive(Debug, Clone)]
pub struct FrameIterator<'a> {
	file: &'a File,
	current_index: usize,
}

impl<'a> Iterator for FrameIterator<'a> {
	type Item = Frame;

	fn next(&mut self) -> Option<Self::Item> {
		let frame = self.file.get_frame(self.current_index)?;
		self.current_index += 1;
		Some(frame)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.file.frame_count as usize - self.current_index;
		(remaining, Some(remaining))
	}
}

impl<'a> ExactSizeIterator for FrameIterator<'a> {
	fn len(&self) -> usize {
		self.file.frame_count as usize - self.current_index
	}
}

impl<'a> IntoIterator for &'a File {
	type Item = Frame;
	type IntoIter = FrameIterator<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Creates a minimal valid MFD file for testing
	fn create_test_mfd(frame_count: u32, entries: Vec<FrameEntry>) -> Vec<u8> {
		let mut data = vec![0u8; constants::GLYPH_TABLE_OFFSET];

		// Write frame count at offset 0x08
		data[constants::FRAME_COUNT_OFFSET..constants::FRAME_COUNT_OFFSET + 4]
			.copy_from_slice(&frame_count.to_le_bytes());

		// Write glyph table entries
		for entry in &entries {
			data.extend_from_slice(&entry.width.to_le_bytes());
			data.extend_from_slice(&entry.height.to_le_bytes());
			data.extend_from_slice(&entry.x_offset.to_le_bytes());
			data.extend_from_slice(&entry.y_offset.to_le_bytes());
			data.extend_from_slice(&entry.bitmap_offset.to_le_bytes());
		}

		// Add bitmap data for each entry
		for entry in &entries {
			let bitmap_offset = constants::BITMAP_DATA_START + entry.bitmap_offset as usize;
			let pixel_count = entry.pixel_count();

			// Ensure we have enough space
			if data.len() < bitmap_offset + pixel_count {
				data.resize(bitmap_offset + pixel_count, 0);
			}

			// Fill with test pattern
			for i in 0..pixel_count {
				data[bitmap_offset + i] = (i % 3) as u8;
			}
		}

		data
	}

	#[test]
	fn test_load_empty_file() {
		let data = vec![0u8; 16];
		let result = File::from_bytes(&data);
		assert!(result.is_err());
	}

	#[test]
	fn test_load_minimal_file() {
		let entries = vec![FrameEntry::new(8, 8, 0, 0, 0)];
		let data = create_test_mfd(1, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		assert_eq!(mfd.frame_count(), 1);
		assert_eq!(mfd.entries().len(), 1);
	}

	#[test]
	fn test_load_multiple_frames() {
		let entries = vec![
			FrameEntry::new(16, 16, -8, -8, 0),
			FrameEntry::new(32, 32, -16, -16, 256),
			FrameEntry::new(8, 8, -4, -4, 1280),
		];
		let data = create_test_mfd(3, entries.clone());

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		assert_eq!(mfd.frame_count(), 3);
		assert_eq!(mfd.entries().len(), 3);

		// Verify entries
		assert_eq!(mfd.get_entry(0).unwrap().width, 16);
		assert_eq!(mfd.get_entry(1).unwrap().width, 32);
		assert_eq!(mfd.get_entry(2).unwrap().width, 8);
	}

	#[test]
	fn test_get_frame() {
		let entries = vec![FrameEntry::new(4, 4, 0, 0, 0)];
		let data = create_test_mfd(1, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		let frame = mfd.get_frame(0).expect("Failed to get frame");

		assert_eq!(frame.width(), 4);
		assert_eq!(frame.height(), 4);
		assert_eq!(frame.pixels().len(), 16);
	}

	#[test]
	fn test_get_frame_out_of_bounds() {
		let entries = vec![FrameEntry::new(8, 8, 0, 0, 0)];
		let data = create_test_mfd(1, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		assert!(mfd.get_frame(1).is_none());
		assert!(mfd.get_frame(100).is_none());
	}

	#[test]
	fn test_iterator() {
		let entries = vec![
			FrameEntry::new(8, 8, 0, 0, 0),
			FrameEntry::new(16, 16, 0, 0, 64),
			FrameEntry::new(8, 8, 0, 0, 320),
		];
		let data = create_test_mfd(3, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		let frames: Vec<_> = mfd.iter().collect();

		assert_eq!(frames.len(), 3);
		assert_eq!(frames[0].width(), 8);
		assert_eq!(frames[1].width(), 16);
		assert_eq!(frames[2].width(), 8);
	}

	#[test]
	fn test_iterator_exact_size() {
		let entries = vec![FrameEntry::new(8, 8, 0, 0, 0), FrameEntry::new(8, 8, 0, 0, 64)];
		let data = create_test_mfd(2, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		let iter = mfd.iter();

		assert_eq!(iter.len(), 2);
		assert_eq!(iter.size_hint(), (2, Some(2)));
	}

	#[test]
	fn test_serialization_roundtrip() {
		let entries =
			vec![FrameEntry::new(16, 16, -8, -8, 0), FrameEntry::new(32, 32, -16, -16, 256)];
		let original_data = create_test_mfd(2, entries);

		let mfd = File::from_bytes(&original_data).expect("Failed to load MFD");
		let serialized = mfd.to_bytes();

		assert_eq!(serialized, original_data);
	}

	#[test]
	fn test_display() {
		let entries = vec![FrameEntry::new(8, 8, 0, 0, 0)];
		let data = create_test_mfd(1, entries);

		let mfd = File::from_bytes(&data).expect("Failed to load MFD");
		let display = format!("{}", mfd);

		assert!(display.contains("1 frames"));
	}
}
