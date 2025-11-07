//! `.MFD` file format support for `dvine-rs` project.
//!
//! This module provides support for loading and manipulating MFD (Mouse File Data) files
//! used in the `D+VINE[LUV]` visual novel engine. MFD files contain animated mouse cursor
//! frames with metadata including dimensions, hotspot offsets, and indexed pixel data.
//!
//! # File Structure Overview
//!
//! The MFD format is a proprietary binary format with a dynamic structure consisting of:
//! - **Fixed-size header** (16 bytes) containing key offsets and counts
//! - **Bitmap data region** starting at offset 0x10, containing raw indexed pixel data
//! - **Metadata region** whose location is specified in the header, containing:
//!   - Animation sequence start table (if animations present)
//!   - Glyph table (frame dimension and offset metadata)
//!   - Animation index table (frame index, duration pairs, and loop markers)
//!
//! ## Header Structure (16 bytes at offset 0x00)
//!
//! ```text
//! Offset  Size  Field                   Description
//! ------  ----  ----------------------  ------------------------------------------
//! 0x00    4     metadata_offset         Offset from BITMAP_DATA_START (0x10) to
//!                                       the metadata region
//! 0x04    4     animation_count         Number of animation sequences
//! 0x08    4     frame_count             Total number of frames in the file
//! 0x0C    4     anim_table_entry_count  Number of entries in animation index table
//! ```
//!
//! All fields are little-endian u32 values.
//!
//! ## Dynamic Offset Calculation
//!
//! The file uses **dynamic offsets** computed from header fields:
//!
//! ```text
//! metadata_region = BITMAP_DATA_START (0x10) + metadata_offset
//! animation_seq_table = metadata_region
//! glyph_table = animation_seq_table + (animation_count × 4)
//! anim_index_table = glyph_table + (frame_count × 12)
//! ```
//!
//! **Important:** Offsets are NOT hardcoded. Each file may have different bitmap sizes
//! and thus different metadata region locations.
//!
//! ## Bitmap Data Region (starts at 0x10)
//!
//! Contains raw indexed pixel data for all frames, stored sequentially or with potential
//! overlaps. Each frame's bitmap offset is stored in its glyph table entry as an offset
//! **relative to `BITMAP_DATA_START` (0x10)**.
//!
//! Pixel size for each frame = `width × height` bytes (1 byte per pixel).
//!
//! ## Metadata Region Structure
//!
//! ### 1. Animation Sequence Start Table
//!
//! ```text
//! Location: metadata_region
//! Size:     animation_count × 4 bytes
//! Format:   Array of u32 (little-endian)
//! ```
//!
//! Each entry is a start index into the animation index table. For example, if
//! `animation_sequences = [0, 11, 24]`, then:
//! - Animation 0 starts at `anim_index_table[0]`
//! - Animation 1 starts at `anim_index_table[11]`
//! - Animation 2 starts at `anim_index_table[24]`
//!
//! ### 2. Glyph Table (Frame Metadata)
//!
//! ```text
//! Location: glyph_table_offset (computed above)
//! Size:     frame_count × 12 bytes
//! Format:   Array of glyph entries
//!
//! Each glyph entry (12 bytes):
//!   +0x00 (2 bytes): width (u16)
//!   +0x02 (2 bytes): height (u16)
//!   +0x04 (2 bytes): x_offset (i16) - hotspot X
//!   +0x06 (2 bytes): y_offset (i16) - hotspot Y
//!   +0x08 (4 bytes): bitmap_offset (u32) - offset from BITMAP_DATA_START (0x10)
//! ```
//!
//! The `bitmap_offset` field points to where this frame's pixel data begins in the
//! bitmap data region. Absolute file offset = `0x10 + bitmap_offset`.
//!
//! ### 3. Animation Index Table
//!
//! ```text
//! Location: anim_index_table_offset (computed above)
//! Size:     anim_table_entry_count × 8 bytes
//! Format:   Array of animation entries
//!
//! Each animation entry (8 bytes):
//!   +0x00 (4 bytes): frame_index (u32)
//!   +0x04 (4 bytes): duration (u32) - in milliseconds or ticks
//!
//! Special value: frame_index = 0xFFFFFFFF indicates a loop marker
//! ```
//!
//! Animation sequences reference entries in this table using start indices from the
//! animation sequence start table. The sequence continues until either:
//! - A loop marker (0xFFFFFFFF) is encountered
//! - The next sequence's start index is reached
//! - The end of the table is reached
//!
//! ## Pixel Format
//!
//! Pixels are stored as indexed bytes with the following mapping:
//!
//! ```text
//! Index   Meaning       BMP Export Color
//! -----   -----------   ----------------
//! 0x00    Transparent   White (255, 255, 255)
//! 0x01    Outline       Gray  (128, 128, 128)
//! 0xFF    Fill          Black (0, 0, 0)
//! ```
//!
//! **Note:** The fill color is `0xFF`, not `0x02`. This was verified through analysis
//! of DXMSTEST.MFD and is critical for correct rendering.
//!
//! ## Example File: DXMSTEST.MFD
//!
//! ```text
//! Header values:
//!   metadata_offset = 0x33C0 (13,248 bytes from 0x10)
//!   animation_count = 3
//!   frame_count = 23
//!   anim_table_entry_count = 26
//!
//! Computed offsets:
//!   Bitmap data:     0x0010 - 0x33CF (13,248 bytes, 23 frames × 24×24)
//!   Metadata region: 0x33D0
//!   Anim seq table:  0x33D0 - 0x33DB (12 bytes, 3 sequences)
//!   Glyph table:     0x33DC - 0x34EF (276 bytes, 23 frames × 12)
//!   Anim idx table:  0x34F0 - 0x35BF (208 bytes, 26 entries × 8)
//!   File size:       13,760 bytes (0x35C0)
//!
//! Animation sequences:
//!   Sequence 0: entries [0..11)   (11 frames)
//!   Sequence 1: entries [11..24)  (13 frames)
//!   Sequence 2: entries [24..26)  (2 frames)
//! ```
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
use serde::{Deserialize, Serialize};

pub mod frame;

pub use frame::{DEFAULT_RGBA_PALETTE, Frame, FrameRowIterator};

/// MFD file format constants.
///
/// These constants define the structure sizes and offsets used by the original executable.
/// While the file format is largely dynamic (offsets computed from header fields), these
/// constants represent hardcoded structure sizes that remain fixed across all MFD files.
///
/// # Structure Size Constants
///
/// - `HEADER_SIZE`: 16 bytes - fixed header at file start
/// - `ANIM_SEQ_ENTRY_SIZE`: 4 bytes - each animation sequence start index
/// - `GLYPH_ENTRY_SIZE`: 12 bytes - each frame metadata entry
/// - `ANIM_ENTRY_SIZE`: 8 bytes - each animation index table entry
///
/// # Fixed Offsets
///
/// - `BITMAP_DATA_START`: 0x10 - bitmap data always starts after header
///
/// # Dynamic Offsets (Computed at Runtime)
///
/// All other offsets must be computed from header fields:
///
/// ```text
/// metadata_region = BITMAP_DATA_START + metadata_offset (from header +0x00)
/// glyph_table = metadata_region + (animation_count × ANIM_SEQ_ENTRY_SIZE)
/// anim_index_table = glyph_table + (frame_count × GLYPH_ENTRY_SIZE)
/// ```
///
/// **Never hardcode absolute offsets like 0x33D0!** These vary per file.
/// Only structure sizes and the bitmap start offset are hardcoded.
pub mod constants {
	/// Fixed offset where bitmap data starts (0x10) - hardcoded in original exe
	pub const BITMAP_DATA_START: usize = 0x10;

	/// Size of the file header (hardcoded in original exe)
	pub const HEADER_SIZE: usize = 16;

	/// Size of each animation sequence start table entry in bytes (hardcoded in original exe)
	pub const ANIM_SEQ_ENTRY_SIZE: usize = 4;

	/// Size of each glyph table entry in bytes (`width`, `height`, `x_offset`, `y_offset`, `bitmap_offset`)
	/// Hardcoded in original exe
	pub const GLYPH_ENTRY_SIZE: usize = 12;

	/// Size of each animation index table entry in bytes (`frame_index`, `duration`)
	/// Hardcoded in original exe
	pub const ANIM_ENTRY_SIZE: usize = 8;

	/// Header field offset: metadata region offset (+0x00) - dynamically read
	pub const METADATA_OFFSET_FIELD: usize = 0x00;
	/// Header field offset: animation sequence count (+0x04) - dynamically read
	pub const ANIMATION_COUNT_FIELD: usize = 0x04;
	/// Header field offset: frame count (+0x08) - dynamically read
	pub const FRAME_COUNT_OFFSET: usize = 0x08;
	/// Header field offset: animation table entry count (+0x0C) - dynamically read
	pub const ANIM_TABLE_ENTRY_COUNT_FIELD: usize = 0x0C;

	/// Loop marker value in animation index table
	pub const LOOP_MARKER: u32 = 0xFFFFFFFF;
}

/// Animation index table entry for MFD files.
///
/// Each entry defines a frame index and its display duration in the animation sequence.
/// A `frame_index` of `0xFFFFFFFF` (represented as `None`) indicates a loop marker,
/// signaling the animation should restart from the sequence's start index.
///
/// # Structure (8 bytes)
/// - `+0x00`: `frame_index` (u32) - Frame index or `0xFFFFFFFF` for loop marker
/// - `+0x04`: `duration` (u32) - Display duration in ticks (game-dependent time unit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationEntry {
	/// Frame index to display, or `None` for loop marker (`0xFFFFFFFF`)
	pub frame_index: Option<u32>,
	/// Display duration in ticks (e.g., 4, 6, 8 ticks per frame in DXMSTEST.MFD)
	pub duration: u32,
}

impl AnimationEntry {
	/// Creates a new animation entry with a frame index and duration
	pub fn new(frame_index: u32, duration: u32) -> Self {
		Self {
			frame_index: Some(frame_index),
			duration,
		}
	}

	/// Creates a loop marker entry (`frame_index` = `0xFFFFFFFF`)
	///
	/// # Arguments
	/// * `duration` - Additional delay before looping (typically 0 for immediate loop)
	pub fn loop_marker(duration: u32) -> Self {
		Self {
			frame_index: None,
			duration,
		}
	}

	/// Returns true if this is a loop marker entry
	pub fn is_loop_marker(&self) -> bool {
		self.frame_index.is_none()
	}
}

/// MFD file structure, representing a complete mouse cursor animation file.
///
/// This structure fully parses the MFD file on load and stores frames in memory.
/// It does not retain the raw file data, making it more memory efficient for
/// applications that need to modify frames.
///
/// # File Components
/// - **Header**: 16-byte header with `metadata_offset`, `animation_count`, `frame_count`, and `anim_table_entry_count`
/// - **Frames**: Vector of parsed frame data with pixel buffers
/// - **Animation sequences**: Optional animation sequence start indices (3 entries typically)
/// - **Animation index table**: Optional `frame_index` + `duration` pairs with loop markers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// All frames in the file (fully parsed)
	frames: Vec<Frame>,
	/// Animation sequence start indices (indexes into `animation_index_table`)
	/// Typically 3 sequences for normal, busy, and special cursor states
	animation_sequences: Option<Vec<u32>>,
	/// Animation index table entries (`frame_index` + `duration` pairs)
	/// Contains loop markers (`0xFFFFFFFF`) to indicate sequence boundaries
	animation_index_table: Option<Vec<AnimationEntry>>,
	/// File header (16 bytes, preserved for round-trip accuracy)
	header: [u8; 16],
}

impl File {
	/// Calculate metadata region offset dynamically from header
	fn metadata_region_offset(&self) -> usize {
		let metadata_offset = u32::from_le_bytes([
			self.header[constants::METADATA_OFFSET_FIELD],
			self.header[constants::METADATA_OFFSET_FIELD + 1],
			self.header[constants::METADATA_OFFSET_FIELD + 2],
			self.header[constants::METADATA_OFFSET_FIELD + 3],
		]);
		constants::BITMAP_DATA_START + metadata_offset as usize
	}

	/// Calculate glyph table offset dynamically from header
	fn glyph_table_offset(&self) -> usize {
		let animation_count = self.animation_count();
		self.metadata_region_offset() + (animation_count as usize * constants::ANIM_SEQ_ENTRY_SIZE)
	}

	/// Calculate animation index table offset dynamically from header
	#[allow(dead_code)]
	fn anim_index_table_offset(&self) -> usize {
		let frame_count = u32::from_le_bytes([
			self.header[constants::FRAME_COUNT_OFFSET],
			self.header[constants::FRAME_COUNT_OFFSET + 1],
			self.header[constants::FRAME_COUNT_OFFSET + 2],
			self.header[constants::FRAME_COUNT_OFFSET + 3],
		]);
		self.glyph_table_offset() + (frame_count as usize * constants::GLYPH_ENTRY_SIZE)
	}

	/// Calculate maximum bitmap size dynamically from header
	fn max_bitmap_size(&self) -> usize {
		self.metadata_region_offset() - constants::BITMAP_DATA_START
	}
}

impl File {
	/// Creates a new empty MFD file.
	pub fn new() -> Self {
		Self {
			frames: Vec::new(),
			animation_sequences: None,
			animation_index_table: None,
			header: [0u8; 16],
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

	/// Returns the animation sequence start indices if present.
	///
	/// Each value is an index into the `animation_index_table` indicating where
	/// the sequence begins. Typically 3 sequences (normal, busy, special).
	#[inline]
	pub fn animation_sequences(&self) -> Option<&[u32]> {
		self.animation_sequences.as_deref()
	}

	/// Sets the animation sequence start indices.
	pub fn set_animation_sequences(&mut self, sequences: Option<Vec<u32>>) {
		self.animation_sequences = sequences;
	}

	/// Returns the animation index table if present.
	///
	/// Contains `frame_index` + `duration` pairs with loop markers (`0xFFFFFFFF`).
	#[inline]
	pub fn animation_index_table(&self) -> Option<&[AnimationEntry]> {
		self.animation_index_table.as_deref()
	}

	/// Sets the animation index table.
	pub fn set_animation_index_table(&mut self, table: Option<Vec<AnimationEntry>>) {
		self.animation_index_table = table;
	}

	/// Returns the animation count from the header.
	///
	/// This is stored at offset +0x04 in the header.
	pub fn animation_count(&self) -> u32 {
		u32::from_le_bytes([
			self.header[constants::ANIMATION_COUNT_FIELD],
			self.header[constants::ANIMATION_COUNT_FIELD + 1],
			self.header[constants::ANIMATION_COUNT_FIELD + 2],
			self.header[constants::ANIMATION_COUNT_FIELD + 3],
		])
	}

	/// Returns the animation table entry count from the header.
	///
	/// This is stored at offset +0x0C in the header.
	pub fn anim_table_entry_count(&self) -> u32 {
		u32::from_le_bytes([
			self.header[constants::ANIM_TABLE_ENTRY_COUNT_FIELD],
			self.header[constants::ANIM_TABLE_ENTRY_COUNT_FIELD + 1],
			self.header[constants::ANIM_TABLE_ENTRY_COUNT_FIELD + 2],
			self.header[constants::ANIM_TABLE_ENTRY_COUNT_FIELD + 3],
		])
	}

	/// Legacy compatibility: Returns `animation_index_table` for backward compatibility.
	#[deprecated(since = "0.2.0", note = "Use animation_index_table() instead")]
	#[inline]
	pub fn animation_metadata(&self) -> Option<&[AnimationEntry]> {
		self.animation_index_table.as_deref()
	}

	/// Legacy compatibility: Sets `animation_index_table` for backward compatibility.
	#[deprecated(since = "0.2.0", note = "Use set_animation_index_table() instead")]
	pub fn set_animation_metadata(&mut self, metadata: Option<Vec<AnimationEntry>>) {
		self.animation_index_table = metadata;
	}

	/// Returns a reference to the file header.
	#[inline]
	pub fn header(&self) -> &[u8; 16] {
		&self.header
	}

	/// Sets the file header.
	pub fn set_header(&mut self, header: [u8; 16]) {
		self.header = header;
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
		let max_size = self.max_bitmap_size();

		if new_size > max_size {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: max_size,
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
		let max_size = self.max_bitmap_size();

		if new_size > max_size {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: max_size,
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
		let max_bitmap_size = self.max_bitmap_size();

		if total_bitmap_size > max_bitmap_size {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: total_bitmap_size,
				blocks_needed: total_bitmap_size,
				blocks_available: max_bitmap_size,
			});
		}

		// Calculate file size including all components (dynamically)
		let glyph_table_offset = self.glyph_table_offset();
		let glyph_table_size = self.frames.len() * constants::GLYPH_ENTRY_SIZE;
		let anim_table_size = self
			.animation_index_table
			.as_ref()
			.map(|t| t.len() * constants::ANIM_ENTRY_SIZE)
			.unwrap_or(0);

		let file_size = glyph_table_offset + glyph_table_size + anim_table_size;
		let mut data = vec![0u8; file_size];

		// Write header (preserve original header bytes)
		data[0..16].copy_from_slice(&self.header);

		// Update frame count at offset 0x08
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

			// Write glyph table entry (dynamically calculated offset)
			let glyph_offset = glyph_table_offset + i * constants::GLYPH_ENTRY_SIZE;
			data[glyph_offset..glyph_offset + 2].copy_from_slice(&frame.width().to_le_bytes());
			data[glyph_offset + 2..glyph_offset + 4].copy_from_slice(&frame.height().to_le_bytes());
			data[glyph_offset + 4..glyph_offset + 6]
				.copy_from_slice(&frame.x_offset().to_le_bytes());
			data[glyph_offset + 6..glyph_offset + 8]
				.copy_from_slice(&frame.y_offset().to_le_bytes());
			data[glyph_offset + 8..glyph_offset + 12].copy_from_slice(&bitmap_offset.to_le_bytes());

			bitmap_offset += frame.pixel_count() as u32;
		}

		// Write animation sequence start table (dynamically calculated offset)
		if let Some(ref sequences) = self.animation_sequences {
			let seq_offset = self.metadata_region_offset();
			for (i, &start_index) in sequences.iter().enumerate() {
				let offset = seq_offset + i * constants::ANIM_SEQ_ENTRY_SIZE;
				data[offset..offset + 4].copy_from_slice(&start_index.to_le_bytes());
			}
		}

		// Write animation index table after glyph table
		if let Some(ref anim_table) = self.animation_index_table {
			let anim_table_offset = glyph_table_offset + glyph_table_size;
			for (i, entry) in anim_table.iter().enumerate() {
				let offset = anim_table_offset + i * constants::ANIM_ENTRY_SIZE;
				let frame_idx = entry.frame_index.unwrap_or(constants::LOOP_MARKER);
				data[offset..offset + 4].copy_from_slice(&frame_idx.to_le_bytes());
				data[offset + 4..offset + 8].copy_from_slice(&entry.duration.to_le_bytes());
			}
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
		// Validate minimum file size (header)
		if data.len() < constants::HEADER_SIZE {
			return Err(DvFileError::insufficient_data(
				FileType::Mfd,
				constants::HEADER_SIZE,
				data.len(),
			));
		}

		// Preserve the full header (first 16 bytes)
		let mut header = [0u8; 16];
		header.copy_from_slice(&data[0..constants::HEADER_SIZE]);

		// Read header fields dynamically
		let metadata_offset = u32::from_le_bytes([
			data[constants::METADATA_OFFSET_FIELD],
			data[constants::METADATA_OFFSET_FIELD + 1],
			data[constants::METADATA_OFFSET_FIELD + 2],
			data[constants::METADATA_OFFSET_FIELD + 3],
		]);
		let animation_count = u32::from_le_bytes([
			data[constants::ANIMATION_COUNT_FIELD],
			data[constants::ANIMATION_COUNT_FIELD + 1],
			data[constants::ANIMATION_COUNT_FIELD + 2],
			data[constants::ANIMATION_COUNT_FIELD + 3],
		]);
		let frame_count = u32::from_le_bytes([
			data[constants::FRAME_COUNT_OFFSET],
			data[constants::FRAME_COUNT_OFFSET + 1],
			data[constants::FRAME_COUNT_OFFSET + 2],
			data[constants::FRAME_COUNT_OFFSET + 3],
		]);

		// Calculate offsets dynamically (as original exe does)
		let metadata_region_offset = constants::BITMAP_DATA_START + metadata_offset as usize;
		let glyph_table_offset =
			metadata_region_offset + (animation_count as usize * constants::ANIM_SEQ_ENTRY_SIZE);
		let anim_index_table_offset =
			glyph_table_offset + (frame_count as usize * constants::GLYPH_ENTRY_SIZE);

		// Validate file has enough data for glyph table
		let glyph_table_end =
			glyph_table_offset + (frame_count as usize * constants::GLYPH_ENTRY_SIZE);
		if data.len() < glyph_table_end {
			return Err(DvFileError::insufficient_data(FileType::Mfd, glyph_table_end, data.len()));
		}

		// Parse glyph table and extract frames
		let mut frames = Vec::with_capacity(frame_count as usize);
		for i in 0..frame_count as usize {
			let glyph_offset = glyph_table_offset + i * constants::GLYPH_ENTRY_SIZE;

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

		// Parse animation sequence start table (between metadata region and glyph table)
		let animation_sequences = if data.len() >= glyph_table_offset && animation_count > 0 {
			let seq_data = &data[metadata_region_offset..glyph_table_offset];
			let expected_size = animation_count as usize * constants::ANIM_SEQ_ENTRY_SIZE;

			if seq_data.len() >= expected_size {
				let mut sequences = Vec::new();
				for i in 0..animation_count as usize {
					let offset = i * constants::ANIM_SEQ_ENTRY_SIZE;
					let start_idx = u32::from_le_bytes([
						seq_data[offset],
						seq_data[offset + 1],
						seq_data[offset + 2],
						seq_data[offset + 3],
					]);
					sequences.push(start_idx);
				}
				Some(sequences)
			} else {
				None
			}
		} else {
			None
		};

		// Parse animation index table (after glyph table)
		let animation_index_table = if data.len() > anim_index_table_offset {
			let remaining_data = &data[anim_index_table_offset..];
			if remaining_data.len() >= constants::ANIM_ENTRY_SIZE {
				let mut anim_table = Vec::new();
				let mut offset = 0;

				while offset + constants::ANIM_ENTRY_SIZE <= remaining_data.len() {
					let frame_idx = u32::from_le_bytes([
						remaining_data[offset],
						remaining_data[offset + 1],
						remaining_data[offset + 2],
						remaining_data[offset + 3],
					]);
					let duration = u32::from_le_bytes([
						remaining_data[offset + 4],
						remaining_data[offset + 5],
						remaining_data[offset + 6],
						remaining_data[offset + 7],
					]);

					if frame_idx == constants::LOOP_MARKER {
						anim_table.push(AnimationEntry::loop_marker(duration));
					} else {
						anim_table.push(AnimationEntry::new(frame_idx, duration));
					}

					offset += constants::ANIM_ENTRY_SIZE;
				}

				if !anim_table.is_empty() {
					Some(anim_table)
				} else {
					None
				}
			} else {
				None
			}
		} else {
			None
		};

		Ok(Self {
			frames,
			animation_sequences,
			animation_index_table,
			header,
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
	animation_sequences: Option<Vec<u32>>,
	animation_index_table: Option<Vec<AnimationEntry>>,
	header: [u8; 16],
}

impl FileBuilder {
	/// Creates a new empty MFD file builder.
	pub fn new() -> Self {
		Self {
			frames: Vec::new(),
			animation_sequences: None,
			animation_index_table: None,
			header: [0u8; 16],
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

		// Note: We can't validate against max_bitmap_size here because we need the header
		// This will be validated when the file is saved via to_bytes()
		let reasonable_max = 1024 * 1024; // 1MB
		if new_size > reasonable_max {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: new_size,
				blocks_needed: new_size,
				blocks_available: reasonable_max,
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
		self.animation_sequences = None;
		self.animation_index_table = None;
	}

	/// Sets the animation sequence start indices for the file.
	///
	/// # Arguments
	/// * `sequences` - Vector of start indices (typically 3 entries for normal, busy, special)
	pub fn animation_sequences(&mut self, sequences: Vec<u32>) -> &mut Self {
		self.animation_sequences = Some(sequences);
		self
	}

	/// Sets the animation index table for the file.
	///
	/// # Arguments
	/// * `table` - Vector of `AnimationEntry` with frame indices and durations
	pub fn animation_index_table(&mut self, table: Vec<AnimationEntry>) -> &mut Self {
		self.animation_index_table = Some(table);
		self
	}

	/// Legacy compatibility: Sets `animation_index_table`.
	#[deprecated(since = "0.2.0", note = "Use animation_index_table() instead")]
	pub fn animation_metadata(&mut self, metadata: Vec<AnimationEntry>) -> &mut Self {
		self.animation_index_table = Some(metadata);
		self
	}

	/// Clears the animation data.
	pub fn clear_animation_data(&mut self) -> &mut Self {
		self.animation_sequences = None;
		self.animation_index_table = None;
		self
	}

	/// Sets the file header.
	pub fn header(&mut self, header: [u8; 16]) -> &mut Self {
		self.header = header;
		self
	}

	/// Gets a reference to the current header.
	pub fn get_header(&self) -> &[u8; 16] {
		&self.header
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

		// Prepare header with correct metadata_offset
		let mut header = self.header;

		// Calculate metadata_offset (offset from BITMAP_DATA_START to metadata region)
		let metadata_offset = total_size as u32;
		header[constants::METADATA_OFFSET_FIELD..constants::METADATA_OFFSET_FIELD + 4]
			.copy_from_slice(&metadata_offset.to_le_bytes());

		// Set animation_count
		let animation_count = self.animation_sequences.as_ref().map(Vec::len).unwrap_or(0) as u32;
		header[constants::ANIMATION_COUNT_FIELD..constants::ANIMATION_COUNT_FIELD + 4]
			.copy_from_slice(&animation_count.to_le_bytes());

		// Set frame_count
		let frame_count = self.frames.len() as u32;
		header[constants::FRAME_COUNT_OFFSET..constants::FRAME_COUNT_OFFSET + 4]
			.copy_from_slice(&frame_count.to_le_bytes());

		// Set anim_table_entry_count
		let anim_table_entry_count =
			self.animation_index_table.as_ref().map(Vec::len).unwrap_or(0) as u32;
		header
			[constants::ANIM_TABLE_ENTRY_COUNT_FIELD..constants::ANIM_TABLE_ENTRY_COUNT_FIELD + 4]
			.copy_from_slice(&anim_table_entry_count.to_le_bytes());

		let file = File {
			frames: self.frames,
			animation_sequences: self.animation_sequences,
			animation_index_table: self.animation_index_table,
			header,
		};

		// Now validate against the calculated max_bitmap_size
		let max_bitmap_size = file.max_bitmap_size();
		if total_size > max_bitmap_size {
			return Err(DvFileError::FileTooLarge {
				file_type: FileType::Mfd,
				size: total_size,
				blocks_needed: total_size,
				blocks_available: max_bitmap_size,
			});
		}

		Ok(file)
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

		// Try to add frames that exceed the reasonable limit (1MB in FileBuilder::add_frame)
		// Create a large frame that when doubled would exceed 1MB
		let frame = Frame::blank(800, 800, 0, 0); // 640,000 pixels

		// First one should succeed
		builder.add_frame(frame.clone()).unwrap();

		// Adding another should fail (640,000 + 640,000 = 1,280,000 > 1,048,576)
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
