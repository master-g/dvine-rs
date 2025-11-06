//! `.SPR` file format support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating SPR (Sprite) files
//! used in the `D+VINE[LUV]` visual novel engine. SPR files contain animated sprite
//! frames with both color data and transparency masks.
//!
//! # File Structure
//!
//! The SPR file format consists of:
//! - **Header (0x00-0x0F):** File metadata including frame count and reserved bytes
//! - **Frame Descriptors:** Metadata entries (24 bytes each) describing each frame
//! - **Data Area:** Raw pixel data containing both color sprites and transparency masks
//!
//! # Frame Descriptor Format
//!
//! Each frame descriptor (24 bytes) contains:
//! - Color offset (4 bytes, little-endian, relative to data area start)
//! - Mask offset (4 bytes, little-endian, relative to data area start)
//! - Width (4 bytes, little-endian)
//! - Height (4 bytes, little-endian)
//! - Hotspot X (4 bytes, little-endian)
//! - Hotspot Y (4 bytes, little-endian)
//!
//! # Pixel Format
//!
//! - **Sprite pixels**: Indexed color values (176-255 range maps to palette indices 0-79)
//! - **Mask pixels**: Binary transparency values (0x00 = transparent, 0xFF = opaque)
//!
//! # Usage Examples
//!
//! ## Loading an SPR file
//!
//! ```no_run
//! use dvine_types::file::spr::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let spr = File::open("AG.SPR")?;
//!
//! println!("Total frames: {}", spr.frame_count());
//!
//! // Get a specific frame
//! if let Some(frame) = spr.get_frame(0) {
//!     println!("Frame 0: {}x{}", frame.width(), frame.height());
//!     println!("Hotspot: ({}, {})", frame.hotspot_x(), frame.hotspot_y());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Iterating over frames
//!
//! ```no_run
//! use dvine_types::file::spr::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let spr = File::open("KATIA.SPR")?;
//!
//! for (index, frame) in spr.iter().enumerate() {
//!     println!("Frame #{}: {}", index, frame);
//!     println!("  Sprite data: {} bytes", frame.sprite_pixels().len());
//!     println!("  Mask data: {} bytes", frame.mask_pixels().len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Exporting frames
//!
//! ```no_run
//! use dvine_types::file::spr::File;
//! use std::fs;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let spr = File::open("AG.SPR")?;
//!
//! // Export frame sprite and mask as PGM
//! if let Some(frame) = spr.get_frame(0) {
//!     let sprite_pgm = frame.sprite_to_pgm();
//!     let mask_pgm = frame.mask_to_pgm();
//!
//!     fs::write("frame_00_sprite.pgm", sprite_pgm)?;
//!     fs::write("frame_00_mask.pgm", mask_pgm)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating a new SPR file
//!
//! ```no_run
//! use dvine_types::file::spr::{File, FrameEntry, Frame};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut spr = File::new();
//!
//! // Create a simple 10x10 frame
//! let entry = FrameEntry::new(0, 100, 10, 10, 5, 5);
//! let sprite_pixels = vec![176; 100]; // Palette index 0
//! let mask_pixels = vec![0xFF; 100];  // Fully opaque
//! let frame = Frame::new(entry, sprite_pixels, mask_pixels);
//!
//! spr.add_frame(frame)?;
//! spr.save("output.spr")?;
//! # Ok(())
//! # }
//! ```

use std::io::Cursor;

use crate::file::{DvFileError, FileType};

pub mod frame;
pub mod palette;

pub use frame::{ColorRowIterator, Frame, FrameEntry, FrameRowIterator};
pub use palette::{Color, Palette};

/// SPR file constants.
pub mod constants {
	/// Size of the file header (16 bytes: `frame_count` + reserved)
	pub const HEADER_SIZE: usize = 16;

	/// Size of each frame descriptor entry (24 bytes)
	pub const FRAME_DESCRIPTOR_SIZE: usize = 24;

	/// Offset of frame count in the header
	pub const FRAME_COUNT_OFFSET: usize = 0;

	/// Size of reserved bytes in header
	pub const RESERVED_SIZE: usize = 12;
}

/// SPR file structure, representing a complete sprite animation file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// Complete file data
	raw: Vec<u8>,

	/// Number of frames in the file
	frame_count: u32,

	/// Frame entries (descriptors)
	entries: Vec<FrameEntry>,
}

impl File {
	/// Creates a new empty SPR file.
	pub fn new() -> Self {
		// Initialize with empty header
		let raw = vec![0u8; constants::HEADER_SIZE];
		// Frame count is 0 (already initialized)

		Self {
			raw,
			frame_count: 0,
			entries: Vec::new(),
		}
	}

	/// Opens an SPR file from the specified path.
	///
	/// # Arguments
	///
	/// * `path` - Path to the SPR file.
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The file is too small to contain required headers
	/// - The frame descriptors are invalid
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

	/// Returns a mutable reference to the frame entries.
	///
	/// **Warning:** Modifying entries directly may lead to inconsistencies
	/// with the underlying pixel data. Prefer using `update_frame()` instead.
	pub fn entries_mut(&mut self) -> &mut [FrameEntry] {
		&mut self.entries
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

	/// Gets a complete frame (entry + sprite + mask data) by index.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// The complete frame with both sprite and mask pixel data, or None if the
	/// index is out of range or the data is invalid.
	pub fn get_frame(&self, index: usize) -> Option<Frame> {
		let entry = self.entries.get(index)?;

		// Calculate data area start
		let data_start = self.data_area_start();

		// Calculate absolute offsets
		let sprite_start = data_start + entry.color_offset as usize;
		let mask_start = data_start + entry.mask_offset as usize;

		let pixel_count = entry.pixel_count();
		let sprite_end = sprite_start + pixel_count;
		let mask_end = mask_start + pixel_count;

		// Validate ranges
		if sprite_end > self.raw.len() || mask_end > self.raw.len() {
			return None;
		}

		// Extract pixel data
		let sprite_pixels = self.raw[sprite_start..sprite_end].to_vec();
		let mask_pixels = self.raw[mask_start..mask_end].to_vec();

		Some(Frame::new(*entry, sprite_pixels, mask_pixels))
	}

	/// Returns an iterator over all frames in the file.
	pub fn iter(&self) -> FrameIterator<'_> {
		FrameIterator {
			file: self,
			current_index: 0,
		}
	}

	/// Calculates the start offset of the data area.
	///
	/// Data area starts after header and all frame descriptors.
	#[inline]
	fn data_area_start(&self) -> usize {
		constants::HEADER_SIZE + (self.frame_count as usize * constants::FRAME_DESCRIPTOR_SIZE)
	}

	/// Updates a frame's pixel data in the file.
	///
	/// This method updates both sprite and mask data for a specific frame.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	/// * `sprite_pixels` - New sprite pixel data
	/// * `mask_pixels` - New mask pixel data
	///
	/// # Returns
	///
	/// `true` if the frame was updated successfully, `false` if the index is
	/// out of range or the pixel data lengths don't match.
	pub fn update_frame(&mut self, index: usize, sprite_pixels: &[u8], mask_pixels: &[u8]) -> bool {
		let Some(entry) = self.entries.get(index) else {
			return false;
		};

		let expected_size = entry.pixel_count();
		if sprite_pixels.len() != expected_size || mask_pixels.len() != expected_size {
			return false;
		}

		let data_start = self.data_area_start();

		// Update sprite data
		let sprite_start = data_start + entry.color_offset as usize;
		let sprite_end = sprite_start + expected_size;
		if sprite_end > self.raw.len() {
			return false;
		}
		self.raw[sprite_start..sprite_end].copy_from_slice(sprite_pixels);

		// Update mask data
		let mask_start = data_start + entry.mask_offset as usize;
		let mask_end = mask_start + expected_size;
		if mask_end > self.raw.len() {
			return false;
		}
		self.raw[mask_start..mask_end].copy_from_slice(mask_pixels);

		true
	}

	/// Updates a complete frame (entry + sprite + mask) in the file.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	/// * `frame` - Complete frame with entry and pixel data
	///
	/// # Returns
	///
	/// `true` if the frame was updated successfully, `false` otherwise.
	pub fn update_complete_frame(&mut self, index: usize, frame: &Frame) -> bool {
		// Update pixel data first
		if !self.update_frame(index, frame.sprite_pixels(), frame.mask_pixels()) {
			return false;
		}

		// Update entry metadata in the descriptor table
		if let Some(entry) = self.entries.get_mut(index) {
			*entry = *frame.entry();

			// Update the raw bytes for the frame descriptor
			let offset = constants::HEADER_SIZE + index * constants::FRAME_DESCRIPTOR_SIZE;
			if offset + constants::FRAME_DESCRIPTOR_SIZE <= self.raw.len() {
				self.raw[offset..offset + 4].copy_from_slice(&entry.color_offset.to_le_bytes());
				self.raw[offset + 4..offset + 8].copy_from_slice(&entry.mask_offset.to_le_bytes());
				self.raw[offset + 8..offset + 12].copy_from_slice(&entry.width.to_le_bytes());
				self.raw[offset + 12..offset + 16].copy_from_slice(&entry.height.to_le_bytes());
				self.raw[offset + 16..offset + 20].copy_from_slice(&entry.hotspot_x.to_le_bytes());
				self.raw[offset + 20..offset + 24].copy_from_slice(&entry.hotspot_y.to_le_bytes());
				return true;
			}
		}

		false
	}

	/// Adds a new frame to the SPR file.
	///
	/// This method appends a frame to the end of the file, updating all necessary
	/// structures including the header, descriptor table, and data area.
	///
	/// # Arguments
	///
	/// * `frame` - The frame to add
	///
	/// # Errors
	///
	/// Returns an error if the frame data is invalid.
	pub fn add_frame(&mut self, frame: Frame) -> Result<(), DvFileError> {
		// Calculate current data area size
		let current_data_start = self.data_area_start();
		let current_data_size = self.raw.len().saturating_sub(current_data_start);

		// Calculate offsets for new frame data (relative to data area start)
		let sprite_offset = current_data_size as u32;
		let mask_offset = sprite_offset + frame.sprite_pixels().len() as u32;

		// Create new entry with calculated offsets
		let new_entry = FrameEntry::new(
			sprite_offset,
			mask_offset,
			frame.width(),
			frame.height(),
			frame.hotspot_x(),
			frame.hotspot_y(),
		);

		// Build new raw data
		let new_frame_count = self.frame_count + 1;
		let new_descriptors_size = new_frame_count as usize * constants::FRAME_DESCRIPTOR_SIZE;
		let new_header_and_descriptors_size = constants::HEADER_SIZE + new_descriptors_size;

		let mut new_raw = Vec::with_capacity(
			new_header_and_descriptors_size + current_data_size + frame.entry().pixel_count() * 2,
		);

		// Write header
		new_raw.extend_from_slice(&new_frame_count.to_le_bytes());
		new_raw.extend_from_slice(&[0u8; constants::RESERVED_SIZE]);

		// Write all frame descriptors (existing + new)
		for entry in &self.entries {
			new_raw.extend_from_slice(&entry.color_offset.to_le_bytes());
			new_raw.extend_from_slice(&entry.mask_offset.to_le_bytes());
			new_raw.extend_from_slice(&entry.width.to_le_bytes());
			new_raw.extend_from_slice(&entry.height.to_le_bytes());
			new_raw.extend_from_slice(&entry.hotspot_x.to_le_bytes());
			new_raw.extend_from_slice(&entry.hotspot_y.to_le_bytes());
		}

		// Write new frame descriptor
		new_raw.extend_from_slice(&new_entry.color_offset.to_le_bytes());
		new_raw.extend_from_slice(&new_entry.mask_offset.to_le_bytes());
		new_raw.extend_from_slice(&new_entry.width.to_le_bytes());
		new_raw.extend_from_slice(&new_entry.height.to_le_bytes());
		new_raw.extend_from_slice(&new_entry.hotspot_x.to_le_bytes());
		new_raw.extend_from_slice(&new_entry.hotspot_y.to_le_bytes());

		// Write existing data area
		if current_data_start < self.raw.len() {
			new_raw.extend_from_slice(&self.raw[current_data_start..]);
		}

		// Write new frame data
		new_raw.extend_from_slice(frame.sprite_pixels());
		new_raw.extend_from_slice(frame.mask_pixels());

		// Update state
		self.raw = new_raw;
		self.frame_count = new_frame_count;
		self.entries.push(new_entry);

		Ok(())
	}

	/// Saves the SPR file to disk.
	///
	/// # Arguments
	///
	/// * `path` - Output file path
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be written.
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), DvFileError> {
		std::fs::write(path, &self.raw)?;
		Ok(())
	}

	/// Serializes the SPR file to bytes.
	pub fn to_bytes(&self) -> Vec<u8> {
		self.raw.clone()
	}

	/// Loads an SPR file from a byte slice.
	///
	/// # Arguments
	///
	/// * `data` - Raw file data
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file is too small to contain the header
	/// - Frame descriptors are invalid
	/// - Data offsets are out of bounds
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let mut cursor = Cursor::new(data);
		Self::from_reader(&mut cursor)
	}

	/// Loads an SPR file from any reader.
	///
	/// # Arguments
	///
	/// * `reader` - Data reader
	///
	/// # Errors
	///
	/// Returns an error if the file structure is invalid.
	pub fn from_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, DvFileError> {
		// Read entire file
		let mut raw = Vec::new();
		reader.read_to_end(&mut raw)?;

		// Validate minimum file size
		if raw.len() < constants::HEADER_SIZE {
			return Err(DvFileError::insufficient_data(
				FileType::Spr,
				constants::HEADER_SIZE,
				raw.len(),
			));
		}

		// Read frame count from header
		let frame_count = u32::from_le_bytes([
			raw[constants::FRAME_COUNT_OFFSET],
			raw[constants::FRAME_COUNT_OFFSET + 1],
			raw[constants::FRAME_COUNT_OFFSET + 2],
			raw[constants::FRAME_COUNT_OFFSET + 3],
		]);

		// Calculate expected size for header + descriptors
		let descriptors_size = frame_count as usize * constants::FRAME_DESCRIPTOR_SIZE;
		let header_and_descriptors_size = constants::HEADER_SIZE + descriptors_size;

		if raw.len() < header_and_descriptors_size {
			return Err(DvFileError::insufficient_data(
				FileType::Spr,
				header_and_descriptors_size,
				raw.len(),
			));
		}

		// Parse frame descriptors
		let mut entries = Vec::with_capacity(frame_count as usize);
		for i in 0..frame_count as usize {
			let offset = constants::HEADER_SIZE + i * constants::FRAME_DESCRIPTOR_SIZE;

			let color_offset = u32::from_le_bytes([
				raw[offset],
				raw[offset + 1],
				raw[offset + 2],
				raw[offset + 3],
			]);

			let mask_offset = u32::from_le_bytes([
				raw[offset + 4],
				raw[offset + 5],
				raw[offset + 6],
				raw[offset + 7],
			]);

			let width = u32::from_le_bytes([
				raw[offset + 8],
				raw[offset + 9],
				raw[offset + 10],
				raw[offset + 11],
			]);

			let height = u32::from_le_bytes([
				raw[offset + 12],
				raw[offset + 13],
				raw[offset + 14],
				raw[offset + 15],
			]);

			let hotspot_x = u32::from_le_bytes([
				raw[offset + 16],
				raw[offset + 17],
				raw[offset + 18],
				raw[offset + 19],
			]);

			let hotspot_y = u32::from_le_bytes([
				raw[offset + 20],
				raw[offset + 21],
				raw[offset + 22],
				raw[offset + 23],
			]);

			entries.push(FrameEntry::new(
				color_offset,
				mask_offset,
				width,
				height,
				hotspot_x,
				hotspot_y,
			));
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
		write!(f, "SPR File: {} frames, {} bytes", self.frame_count, self.raw.len())
	}
}

/// Iterator over frames in an SPR file.
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

	#[test]
	fn test_new_file() {
		let file = File::new();
		assert_eq!(file.frame_count(), 0);
		assert_eq!(file.entries().len(), 0);
	}

	#[test]
	fn test_add_frame() {
		let mut file = File::new();

		let entry = FrameEntry::new(0, 0, 2, 2, 1, 1);
		let sprite = vec![176, 177, 178, 179];
		let mask = vec![0xFF; 4];
		let frame = Frame::new(entry, sprite, mask);

		assert!(file.add_frame(frame).is_ok());
		assert_eq!(file.frame_count(), 1);

		let retrieved = file.get_frame(0).unwrap();
		assert_eq!(retrieved.width(), 2);
		assert_eq!(retrieved.height(), 2);
	}

	#[test]
	fn test_roundtrip() {
		let mut file = File::new();

		// Add a frame
		let entry = FrameEntry::new(0, 0, 3, 3, 1, 1);
		let sprite = vec![176; 9];
		let mask = vec![0xFF; 9];
		let frame = Frame::new(entry, sprite.clone(), mask.clone());
		file.add_frame(frame).unwrap();

		// Serialize and deserialize
		let bytes = file.to_bytes();
		let loaded = File::from_bytes(&bytes).unwrap();

		assert_eq!(loaded.frame_count(), 1);
		let loaded_frame = loaded.get_frame(0).unwrap();
		assert_eq!(loaded_frame.sprite_pixels(), &sprite);
		assert_eq!(loaded_frame.mask_pixels(), &mask);
	}
}
