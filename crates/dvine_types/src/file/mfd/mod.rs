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
//! // Get a specific frame (zero-copy reference)
//! if let Some(frame) = mfd.frame(0) {
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
//! for (index, frame) in mfd.frames().iter().enumerate() {
//!     println!("Frame #{}: {}", index, frame);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Building an MFD file
//!
//! ```no_run
//! use dvine_types::file::mfd::{FileBuilder, Frame};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut builder = FileBuilder::new();
//!
//! // Add frames
//! let frame1 = Frame::blank(24, 24, 0, 0);
//! builder.add_frame(frame1)?;
//!
//! let frame2 = Frame::blank(32, 32, -16, -16);
//! builder.add_frame(frame2)?;
//!
//! // Build and save
//! let mfd = builder.build()?;
//! mfd.save("output.mfd")?;
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
//! if let Some(frame) = mfd.frame(0) {
//!     let pgm_data = frame.to_pgm();
//!     fs::write("frame_00.pgm", pgm_data)?;
//! }
//!
//! // Export as RGBA
//! if let Some(frame) = mfd.frame(1) {
//!     let rgba = frame.to_rgba();
//!     // Use rgba data with image libraries
//! }
//! # Ok(())
//! # }
//! ```

use crate::file::{DvFileError, FileType};

pub mod frame;

pub use frame::{DEFAULT_RGBA_PALETTE, Frame, FrameRowIterator};

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

	/// Maximum bitmap data size (from `BITMAP_DATA_START` t`GLYPH_TABLE_OFFSET`)
	pub const MAX_BITMAP_SIZE: usize = GLYPH_TABLE_OFFSET - BITMAP_DATA_START;
}

/// MFD file structure, representing a complete mouse cursor animation file.
///
/// This structure fully parses the MFD file on load and stores frames in memory.
/// It does not retain the raw file data, making it more memory efficient for
/// applications that need to modify frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// All frames in the file (fully parsed)
	frames: Vec<Frame>,
}

impl File {
	/// Creates a new empty MFD file.
	pub fn new() -> Self {
		Self {
			frames: Vec::new(),
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
	#[inline]
	pub fn frame_count(&self) -> usize {
		self.frames.len()
	}

	/// Returns a reference to a specific frame (zero-copy access).
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// A reference to the frame, or None if the index is out of range.
	#[inline]
	pub fn frame(&self, index: usize) -> Option<&Frame> {
		self.frames.get(index)
	}

	/// Returns a mutable reference to a specific frame.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// A mutable reference to the frame, or None if the index is out of range.
	#[inline]
	pub fn frame_mut(&mut self, index: usize) -> Option<&mut Frame> {
		self.frames.get_mut(index)
	}

	/// Returns a slice of all frames (zero-copy access).
	#[inline]
	pub fn frames(&self) -> &[Frame] {
		&self.frames
	}

	/// Returns a mutable slice of all frames.
	#[inline]
	pub fn frames_mut(&mut self) -> &mut [Frame] {
		&mut self.frames
	}

	/// Returns an iterator over all frames.
	pub fn iter(&self) -> std::slice::Iter<'_, Frame> {
		self.frames.iter()
	}

	/// Returns a mutable iterator over all frames.
	pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Frame> {
		self.frames.iter_mut()
	}

	/// Adds a new frame to the file.
	///
	/// # Arguments
	///
	/// * `frame` - The frame to add
	///
	/// # Errors
	///
	/// Returns an error if adding the frame would exceed the maximum bitmap size.
	pub fn add_frame(&mut self, frame: Frame) -> Result<(), DvFileError> {
		// Calculate total bitmap size with new frame
		let current_size: usize = self.frames.iter().map(frame::Frame::pixel_count).sum();
		let new_size = current_size + frame.pixel_count();

		if new_size > constants::MAX_BITMAP_SIZE {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: constants::MAX_BITMAP_SIZE,
			});
		}

		self.frames.push(frame);
		Ok(())
	}

	/// Removes a frame from the file.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	///
	/// # Returns
	///
	/// The removed frame, or None if the index is out of range.
	pub fn remove_frame(&mut self, index: usize) -> Option<Frame> {
		if index < self.frames.len() {
			Some(self.frames.remove(index))
		} else {
			None
		}
	}

	/// Replaces a frame in the file.
	///
	/// # Arguments
	///
	/// * `index` - Frame index (0-based)
	/// * `frame` - The new frame
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The index is out of range
	/// - Replacing the frame would exceed the maximum bitmap size
	pub fn replace_frame(&mut self, index: usize, frame: Frame) -> Result<Frame, DvFileError> {
		if index >= self.frames.len() {
			return Err(DvFileError::BlockOutOfRange {
				file_type: FileType::Mfd,
				index: index as u32,
				total: self.frames.len(),
			});
		}

		// Calculate total bitmap size with replaced frame
		let current_size: usize = self
			.frames
			.iter()
			.enumerate()
			.filter(|(i, _)| *i != index)
			.map(|(_, f)| f.pixel_count())
			.sum();
		let new_size = current_size + frame.pixel_count();

		if new_size > constants::MAX_BITMAP_SIZE {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: constants::MAX_BITMAP_SIZE,
			});
		}

		Ok(std::mem::replace(&mut self.frames[index], frame))
	}

	/// Saves the MFD file to disk.
	///
	/// # Arguments
	///
	/// * `path` - Output file path
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be written.
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), DvFileError> {
		let data = self.to_bytes()?;
		std::fs::write(path, data)?;
		Ok(())
	}

	/// Serializes the MFD file to bytes.
	///
	/// # Errors
	///
	/// Returns an error if the frames exceed the maximum bitmap size.
	pub fn to_bytes(&self) -> Result<Vec<u8>, DvFileError> {
		// Calculate total bitmap size
		let total_bitmap_size: usize = self.frames.iter().map(frame::Frame::pixel_count).sum();

		if total_bitmap_size > constants::MAX_BITMAP_SIZE {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: total_bitmap_size,
				blocks_needed: total_bitmap_size,
				blocks_available: constants::MAX_BITMAP_SIZE,
			});
		}

		// Calculate file size
		let file_size =
			constants::GLYPH_TABLE_OFFSET + self.frames.len() * constants::GLYPH_ENTRY_SIZE;
		let mut data = vec![0u8; file_size];

		// Write header
		// Frame count at offset 0x08
		let frame_count = self.frames.len() as u32;
		data[constants::FRAME_COUNT_OFFSET..constants::FRAME_COUNT_OFFSET + 4]
			.copy_from_slice(&frame_count.to_le_bytes());

		// Write bitmap data and build glyph table
		let mut bitmap_offset = 0u32;
		for (i, frame) in self.frames.iter().enumerate() {
			// Write bitmap data
			let bitmap_start = constants::BITMAP_DATA_START + bitmap_offset as usize;
			let bitmap_end = bitmap_start + frame.pixel_count();
			data[bitmap_start..bitmap_end].copy_from_slice(frame.pixels());

			// Write glyph table entry
			let glyph_offset = constants::GLYPH_TABLE_OFFSET + i * constants::GLYPH_ENTRY_SIZE;
			data[glyph_offset..glyph_offset + 2].copy_from_slice(&frame.width().to_le_bytes());
			data[glyph_offset + 2..glyph_offset + 4].copy_from_slice(&frame.height().to_le_bytes());
			data[glyph_offset + 4..glyph_offset + 6]
				.copy_from_slice(&frame.x_offset().to_le_bytes());
			data[glyph_offset + 6..glyph_offset + 8]
				.copy_from_slice(&frame.y_offset().to_le_bytes());
			data[glyph_offset + 8..glyph_offset + 12].copy_from_slice(&bitmap_offset.to_le_bytes());

			bitmap_offset += frame.pixel_count() as u32;
		}

		Ok(data)
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
		// Validate minimum file size (header + at least glyph table start)
		if data.len() < constants::GLYPH_TABLE_OFFSET {
			return Err(DvFileError::insufficient_data(
				FileType::Mfd,
				constants::GLYPH_TABLE_OFFSET,
				data.len(),
			));
		}

		// Read frame count from offset 0x08
		if data.len() < constants::FRAME_COUNT_OFFSET + 4 {
			return Err(DvFileError::insufficient_data(
				FileType::Mfd,
				constants::FRAME_COUNT_OFFSET + 4,
				data.len(),
			));
		}

		let frame_count = u32::from_le_bytes([
			data[constants::FRAME_COUNT_OFFSET],
			data[constants::FRAME_COUNT_OFFSET + 1],
			data[constants::FRAME_COUNT_OFFSET + 2],
			data[constants::FRAME_COUNT_OFFSET + 3],
		]);

		// Parse glyph table and extract frames
		let mut frames = Vec::with_capacity(frame_count as usize);
		for i in 0..frame_count as usize {
			let glyph_offset = constants::GLYPH_TABLE_OFFSET + i * constants::GLYPH_ENTRY_SIZE;

			// Validate we have enough data for this entry
			if data.len() < glyph_offset + constants::GLYPH_ENTRY_SIZE {
				return Err(DvFileError::insufficient_data(
					FileType::Mfd,
					glyph_offset + constants::GLYPH_ENTRY_SIZE,
					data.len(),
				));
			}

			// Parse entry fields
			let width = u16::from_le_bytes([data[glyph_offset], data[glyph_offset + 1]]);
			let height = u16::from_le_bytes([data[glyph_offset + 2], data[glyph_offset + 3]]);
			let x_offset = i16::from_le_bytes([data[glyph_offset + 4], data[glyph_offset + 5]]);
			let y_offset = i16::from_le_bytes([data[glyph_offset + 6], data[glyph_offset + 7]]);
			let bitmap_offset = u32::from_le_bytes([
				data[glyph_offset + 8],
				data[glyph_offset + 9],
				data[glyph_offset + 10],
				data[glyph_offset + 11],
			]);

			// Calculate absolute bitmap offset and extract pixels
			let bitmap_start = constants::BITMAP_DATA_START + bitmap_offset as usize;
			let pixel_count = width as usize * height as usize;
			let bitmap_end = bitmap_start + pixel_count;

			// Validate bitmap range
			if bitmap_end > data.len() {
				return Err(DvFileError::InvalidExtraction {
					file_type: FileType::Mfd,
					required: bitmap_end,
					available: data.len(),
				});
			}

			// Extract pixel data and create frame
			let pixels = data[bitmap_start..bitmap_end].to_vec();
			frames.push(Frame::new(width, height, x_offset, y_offset, pixels));
		}

		Ok(Self {
			frames,
		})
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
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		Self::from_bytes(&data)
	}
}

impl Default for File {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "MFD File: {} frames", self.frames.len())
	}
}

impl<'a> IntoIterator for &'a File {
	type Item = &'a Frame;
	type IntoIter = std::slice::Iter<'a, Frame>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a> IntoIterator for &'a mut File {
	type Item = &'a mut Frame;
	type IntoIter = std::slice::IterMut<'a, Frame>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

/// Builder for creating MFD files.
///
/// This builder provides a convenient way to construct MFD files programmatically
/// with proper validation and error handling.
///
/// # Example
///
/// ```no_run
/// use dvine_types::file::mfd::{FileBuilder, Frame};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut builder = FileBuilder::new();
///
/// // Add multiple frames
/// for i in 0..10 {
///     let frame = Frame::blank(24, 24, 0, 0);
///     builder.add_frame(frame)?;
/// }
///
/// // Build and save
/// let mfd = builder.build()?;
/// mfd.save("output.mfd")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct FileBuilder {
	frames: Vec<Frame>,
}

impl FileBuilder {
	/// Creates a new empty MFD file builder.
	pub fn new() -> Self {
		Self {
			frames: Vec::new(),
		}
	}

	/// Adds a frame to the builder.
	///
	/// # Arguments
	///
	/// * `frame` - The frame to add
	///
	/// # Errors
	///
	/// Returns an error if adding the frame would exceed the maximum bitmap size.
	pub fn add_frame(&mut self, frame: Frame) -> Result<&mut Self, DvFileError> {
		// Calculate total bitmap size with new frame
		let current_size: usize = self.frames.iter().map(frame::Frame::pixel_count).sum();
		let new_size = current_size + frame.pixel_count();

		if new_size > constants::MAX_BITMAP_SIZE {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: constants::MAX_BITMAP_SIZE,
			});
		}

		self.frames.push(frame);
		Ok(self)
	}

	/// Adds multiple frames to the builder.
	///
	/// # Arguments
	///
	/// * `frames` - Iterator of frames to add
	///
	/// # Errors
	///
	/// Returns an error if adding the frames would exceed the maximum bitmap size.
	pub fn add_frames<I>(&mut self, frames: I) -> Result<&mut Self, DvFileError>
	where
		I: IntoIterator<Item = Frame>,
	{
		for frame in frames {
			self.add_frame(frame)?;
		}
		Ok(self)
	}

	/// Returns the number of frames currently in the builder.
	#[inline]
	pub fn frame_count(&self) -> usize {
		self.frames.len()
	}

	/// Returns the total bitmap size of all frames.
	#[inline]
	pub fn bitmap_size(&self) -> usize {
		self.frames.iter().map(frame::Frame::pixel_count).sum()
	}

	/// Clears all frames from the builder.
	pub fn clear(&mut self) {
		self.frames.clear();
	}

	/// Builds the MFD file.
	///
	/// This consumes the builder and returns the constructed file.
	///
	/// # Errors
	///
	/// Returns an error if the total bitmap size exceeds the maximum.
	pub fn build(self) -> Result<File, DvFileError> {
		let total_size: usize = self.frames.iter().map(frame::Frame::pixel_count).sum();

		if total_size > constants::MAX_BITMAP_SIZE {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: total_size,
				blocks_needed: total_size,
				blocks_available: constants::MAX_BITMAP_SIZE,
			});
		}

		Ok(File {
			frames: self.frames,
		})
	}

	/// Builds and saves the MFD file directly.
	///
	/// This is a convenience method that builds the file and saves it in one step.
	///
	/// # Arguments
	///
	/// * `path` - Output file path
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The total bitmap size exceeds the maximum
	/// - The file cannot be written
	pub fn save(self, path: impl AsRef<std::path::Path>) -> Result<File, DvFileError> {
		let file = self.build()?;
		file.save(&path)?;
		Ok(file)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_frame_creation() {
		let frame = Frame::blank(8, 8, 0, 0);
		assert_eq!(frame.width(), 8);
		assert_eq!(frame.height(), 8);
		assert_eq!(frame.pixel_count(), 64);
	}

	#[test]
	fn test_builder_basic() {
		let mut builder = FileBuilder::new();
		let frame1 = Frame::blank(24, 24, 0, 0);
		let frame2 = Frame::blank(32, 32, -16, -16);

		builder.add_frame(frame1).unwrap();
		builder.add_frame(frame2).unwrap();

		assert_eq!(builder.frame_count(), 2);
		assert_eq!(builder.bitmap_size(), 24 * 24 + 32 * 32);

		let file = builder.build().unwrap();
		assert_eq!(file.frame_count(), 2);
	}

	#[test]
	fn test_builder_size_limit() {
		let mut builder = FileBuilder::new();

		// Try to add frames that exceed the limit
		// MAX_BITMAP_SIZE = 13,260 bytes
		let frame = Frame::blank(100, 100, 0, 0); // 10,000 pixels

		// First one should succeed
		builder.add_frame(frame.clone()).unwrap();

		// Adding another should fail (10,000 + 10,000 > 13,260)
		let result = builder.add_frame(frame);
		assert!(result.is_err());
	}

	#[test]
	fn test_file_roundtrip() {
		let mut builder = FileBuilder::new();
		let frame1 = Frame::blank(24, 24, 0, 0);
		let frame2 = Frame::blank(32, 32, -8, -8);

		builder.add_frame(frame1).unwrap();
		builder.add_frame(frame2).unwrap();

		let file = builder.build().unwrap();
		let bytes = file.to_bytes().unwrap();
		let loaded = File::from_bytes(&bytes).unwrap();

		assert_eq!(file.frame_count(), loaded.frame_count());
		assert_eq!(file.frames(), loaded.frames());
	}
}
