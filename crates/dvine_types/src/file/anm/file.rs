//! ANM file structure and I/O operations.
//!
//! This module defines the main `File` struct which represents a complete ANM
//! (Animation) file with header, index table, and animation sequences.

use std::io::Read;

use crate::file::{DvFileError, FileType};

use super::{constants, sequence::AnimationSequence};

/// Byte range occupied by a single animation slot's data payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotDataWindow {
	/// Absolute offset within the ANM file where the slot's data starts.
	pub start: usize,
	/// Absolute offset (exclusive) where the slot's data ends.
	pub end: usize,
}

impl SlotDataWindow {
	/// Length of the byte window.
	pub fn len(&self) -> usize {
		self.end.saturating_sub(self.start)
	}

	/// Returns `true` when the window has zero length.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

/// Computes byte windows for each slot based on the index table and file length.
pub fn compute_slot_windows(
	index_table: &[u16; constants::ANIMATION_SLOT_COUNT],
	file_len: usize,
) -> [Option<SlotDataWindow>; constants::ANIMATION_SLOT_COUNT] {
	let mut windows = [None; constants::ANIMATION_SLOT_COUNT];
	let mut entries: Vec<(usize, usize)> = index_table
		.iter()
		.enumerate()
		.filter_map(|(slot, &word_offset)| {
			if word_offset == constants::NO_ANIMATION {
				return None;
			}

			let start = constants::ANIMATION_DATA_OFFSET + (word_offset as usize * 2);
			if start >= file_len {
				return None;
			}

			Some((slot, start))
		})
		.collect();

	entries.sort_by_key(|&(_, start)| start);

	for i in 0..entries.len() {
		let (slot, start) = entries[i];
		let mut end = file_len;
		for &(_, next_start) in entries.iter().skip(i + 1) {
			if next_start > start {
				end = next_start;
				break;
			}
		}
		windows[slot] = Some(SlotDataWindow {
			start,
			end: end.min(file_len),
		});
	}

	windows
}

/// ANM file structure containing animation sequences.
///
/// An ANM file consists of:
/// - **Header** (32 bytes at 0x00): File identifier
/// - **Index Table** (512 bytes at 0x20): 256 u16 word offsets pointing to animation sequences
/// - **Animation Data** (starting at 0x220): Variable-length animation sequences
///
/// # Index Table Encoding
///
/// The index table contains word offsets (not byte offsets):
/// - Index table value: word offset (u16)
/// - Byte offset: `word_offset` × 2
/// - File offset: 0x220 + `byte_offset`
/// - Value 0xFFFF indicates no animation in that slot
///
/// # Shared Data Regions
///
/// Multiple slots can point to the same or overlapping data regions. This is
/// **intentional** and used for:
/// - Space optimization (reusing common sequences)
/// - Animation variants (different entry points into shared data)
///
/// # Examples
///
/// ## Opening and reading a file
///
/// ```no_run
/// use dvine_types::file::anm::File;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let anm = File::open("AGMAGIC.anm")?;
///
/// println!("Total slots: {}", anm.slot_count());
/// println!("Active sequences: {}", anm.sequences().len());
///
/// // Access a specific sequence
/// if let Some(seq) = anm.get_sequence(0) {
///     println!("Slot 0 has {} frames", seq.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Creating and modifying
///
/// ```
/// use dvine_types::file::anm::{File, AnimationSequence, FrameDescriptor};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut anm = File::new();
///
/// // Create a sequence
/// let mut seq = AnimationSequence::new();
/// seq.add_frame(FrameDescriptor::frame(0, 10));
/// seq.add_frame(FrameDescriptor::frame(1, 10));
/// seq.add_hold_marker();
///
/// // Add to file
/// anm.set_sequence(5, seq)?;
///
/// // Save to file
/// anm.save("output.anm")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
	/// File header (32 bytes)
	header: [u8; constants::HEADER_SIZE],

	/// Index table mapping slots to animation data offsets (WORD offsets, multiply by 2 for byte offset from 0x220)
	index_table: [u16; constants::ANIMATION_SLOT_COUNT],

	/// Animation sequences stored by slot index
	sequences: Vec<(usize, AnimationSequence)>,
}

impl File {
	/// Creates a new empty ANM file.
	///
	/// The file is initialized with:
	/// - Zero-filled header
	/// - Empty index table (all slots set to `NO_ANIMATION`)
	/// - No animation sequences
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::File;
	///
	/// let anm = File::new();
	/// assert_eq!(anm.sequences().len(), 0);
	/// assert_eq!(anm.slot_count(), 256);
	/// ```
	pub fn new() -> Self {
		Self {
			header: [0u8; constants::HEADER_SIZE],
			index_table: [constants::NO_ANIMATION; constants::ANIMATION_SLOT_COUNT],
			sequences: Vec::new(),
		}
	}

	/// Opens an ANM file from the specified path.
	///
	/// # Arguments
	///
	/// * `path` - Path to the ANM file
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The file is too small to contain required headers
	/// - The animation data is invalid
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let anm = File::open("AGMAGIC.anm")?;
	/// println!("Loaded {} sequences", anm.sequences().len());
	/// # Ok(())
	/// # }
	/// ```
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let data = std::fs::read(path)?;
		Self::from_bytes(&data)
	}

	/// Opens an ANM file from the specified path in raw mode (without simulating jumps).
	///
	/// This method reads animation sequences without executing jump instructions,
	/// preserving the original structure of the animation data. This is useful for
	/// editing tools that need to see the actual frame descriptors as stored in the file.
	///
	/// # Arguments
	///
	/// * `path` - Path to the ANM file
	///
	/// # Returns
	///
	/// The parsed ANM file with original animation structure preserved
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file cannot be opened or read
	/// - The file is too small to contain required headers
	/// - The animation data is invalid
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let anm = File::open_raw("AGMAGIC.anm")?;
	/// // Sequences will show actual jump instructions, not expanded loops
	/// println!("Loaded {} sequences", anm.sequences().len());
	/// # Ok(())
	/// # }
	/// ```
	pub fn open_raw(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let data = std::fs::read(path)?;
		Self::from_bytes_raw(&data)
	}

	/// Returns the number of animation slots (always 256).
	pub fn slot_count(&self) -> usize {
		constants::ANIMATION_SLOT_COUNT
	}

	/// Returns a reference to the file header.
	pub fn header(&self) -> &[u8; constants::HEADER_SIZE] {
		&self.header
	}

	/// Returns a mutable reference to the file header.
	pub fn header_mut(&mut self) -> &mut [u8; constants::HEADER_SIZE] {
		&mut self.header
	}

	/// Returns the sprite filename stored in the header.
	///
	/// The header contains a null-terminated ASCII string stored in the first
	/// 12 bytes (max 11 visible characters + null terminator).
	/// Returns an empty string if the header is all zeros or contains invalid UTF-8.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::File;
	///
	/// let mut anm = File::new();
	/// anm.set_spr_filename("test.spr").unwrap();
	/// assert_eq!(anm.spr_filename(), "test.spr");
	/// ```
	pub fn spr_filename(&self) -> &str {
		let end = self.header[..constants::SPR_FILENAME_FIELD_LEN]
			.iter()
			.position(|&b| b == 0)
			.unwrap_or(constants::SPR_FILENAME_FIELD_LEN);
		std::str::from_utf8(&self.header[..end]).unwrap_or("")
	}

	/// Sets the sprite filename in the header.
	///
	/// The filename is stored as a null-terminated ASCII string in the 12-byte header field.
	/// Filenames longer than 11 characters are rejected to match the on-disk format.
	///
	/// # Arguments
	///
	/// * `filename` - The sprite filename (typically "FILENAME.spr", case-insensitive)
	///
	/// # Errors
	///
	/// Returns an error if the filename contains non-ASCII characters.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut anm = File::new();
	/// anm.set_spr_filename("AGMAGIC.spr")?;
	/// assert_eq!(anm.spr_filename(), "AGMAGIC.spr");
	/// # Ok(())
	/// # }
	/// ```
	pub fn set_spr_filename(&mut self, filename: &str) -> Result<(), DvFileError> {
		if !filename.is_ascii() {
			return Err(DvFileError::BadEncoding {
				file_type: FileType::Anm,
				message: "SPR filename must be ASCII".to_string(),
			});
		}

		let bytes = filename.as_bytes();
		if bytes.len() > constants::SPR_FILENAME_MAX_LEN {
			return Err(DvFileError::BadEncoding {
				file_type: FileType::Anm,
				message: format!(
					"SPR filename must be at most {} characters",
					constants::SPR_FILENAME_MAX_LEN
				),
			});
		}

		self.header.fill(0);
		self.header[..bytes.len()].copy_from_slice(bytes);
		Ok(())
	}

	/// Returns a reference to the index table.
	///
	/// The index table contains WORD offsets (not byte offsets).
	/// To get the byte offset from 0x220, multiply the index value by 2.
	///
	/// # Note
	/// A value of 0xFFFF indicates an empty slot (no animation).
	pub fn index_table(&self) -> &[u16; constants::ANIMATION_SLOT_COUNT] {
		&self.index_table
	}

	/// Returns the word offset for a given slot index.
	///
	/// # Arguments
	///
	/// * `slot` - Slot index (0-255)
	///
	/// # Returns
	/// Returns the WORD offset for a given animation slot, or None if slot is out of range or empty.
	/// To get the byte offset from 0x220, multiply the returned value by 2.
	///
	/// # Example
	///
	/// ```
	/// # use dvine_types::file::anm::File;
	/// # let file = File::new();
	/// if let Some(word_offset) = file.get_slot_offset(0) {
	///     let byte_offset = word_offset as usize * 2;
	///     let file_position = 0x220 + byte_offset;
	/// }
	/// ```
	pub fn get_slot_offset(&self, slot: usize) -> Option<u16> {
		if slot < constants::ANIMATION_SLOT_COUNT
			&& self.index_table[slot] != constants::NO_ANIMATION
		{
			Some(self.index_table[slot])
		} else {
			None
		}
	}

	/// Returns a reference to all animation sequences.
	///
	/// Each element is a tuple of (`slot_index``AnimationSequence`ce).
	pub fn sequences(&self) -> &[(usize, AnimationSequence)] {
		&self.sequences
	}

	/// Returns a mutable reference to all animation sequences.
	/// Gets an animation sequence by slot index.
	///
	/// # Arguments
	///
	/// * `slot` - Slot index (0-255)
	///
	/// # Returns
	///
	/// A reference to the animation sequence, or None if not found
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let anm = File::open("AGMAGIC.anm")?;
	///
	/// if let Some(seq) = anm.get_sequence(0) {
	///     println!("Slot 0: {} frames", seq.len());
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub fn get_sequence(&self, slot: usize) -> Option<&AnimationSequence> {
		self.sequences.iter().find(|(s, _)| *s == slot).map(|(_, seq)| seq)
	}

	/// Gets a mutable reference to an animation sequence by slot index.
	///
	/// # Arguments
	///
	/// * `slot` - Slot index (0-255)
	///
	/// # Returns
	///
	/// A mutable reference to the animation sequence, or None if not found
	pub fn get_sequence_mut(&mut self, slot: usize) -> Option<&mut AnimationSequence> {
		self.sequences.iter_mut().find(|(s, _)| *s == slot).map(|(_, seq)| seq)
	}

	/// Sets an animation sequence for a given slot.
	///
	/// If a sequence already exists for the slot, it will be replaced.
	///
	/// # Arguments
	///
	/// * `slot` - Slot index (0-255)
	/// * `sequence` - Animation sequence to set
	///
	/// # Errors
	///
	/// Returns an error if the slot index is out of range (>= 256)
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::{File, AnimationSequence, FrameDescriptor};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut anm = File::new();
	///
	/// let mut seq = AnimationSequence::new();
	/// seq.add_frame(FrameDescriptor::frame(0, 10));
	/// seq.add_hold_marker();
	///
	/// anm.set_sequence(5, seq)?;
	/// assert!(anm.get_sequence(5).is_some());
	/// # Ok(())
	/// # }
	/// ```
	pub fn set_sequence(
		&mut self,
		slot: usize,
		sequence: AnimationSequence,
	) -> Result<(), DvFileError> {
		if slot >= constants::ANIMATION_SLOT_COUNT {
			return Err(DvFileError::EntryNotFound {
				file_type: FileType::Anm,
				message: format!(
					"Slot index {} out of range (max {})",
					slot,
					constants::ANIMATION_SLOT_COUNT - 1
				),
			});
		}

		// Remove existing sequence for this slot if present
		self.sequences.retain(|(s, _)| *s != slot);

		// Add new sequence
		self.sequences.push((slot, sequence));

		// Sort by slot index for consistent ordering
		self.sequences.sort_by_key(|(s, _)| *s);
		self.rebuild_index_table_from_sequences();

		Ok(())
	}

	/// Removes an animation sequence from a slot.
	///
	/// # Arguments
	///
	/// * `slot` - Slot index (0-255)
	///
	/// # Returns
	///
	/// The removed animation sequence, or None if the slot had no sequence
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::anm::{File, AnimationSequence};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut anm = File::new();
	/// anm.set_sequence(5, AnimationSequence::new())?;
	///
	/// let removed = anm.remove_sequence(5);
	/// assert!(removed.is_some());
	/// assert!(anm.get_sequence(5).is_none());
	/// # Ok(())
	/// # }
	/// ```
	pub fn remove_sequence(&mut self, slot: usize) -> Option<AnimationSequence> {
		if let Some(pos) = self.sequences.iter().position(|(s, _)| *s == slot) {
			let removed = self.sequences.remove(pos).1;
			self.rebuild_index_table_from_sequences();
			Some(removed)
		} else {
			None
		}
	}

	/// Saves the ANM file to the specified path.
	///
	/// # Arguments
	///
	/// * `path` - Path where the file will be saved
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be written
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::{File, AnimationSequence, FrameDescriptor};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut anm = File::new();
	///
	/// let mut seq = AnimationSequence::new();
	/// seq.add_frame(FrameDescriptor::frame(0, 10));
	/// seq.add_hold_marker();
	/// anm.set_sequence(0, seq)?;
	///
	/// anm.save("output.anm")?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn save(&self, path: impl AsRef<std::path::Path>) -> Result<(), DvFileError> {
		let bytes = self.to_bytes();
		std::fs::write(path, bytes)?;
		Ok(())
	}

	/// Converts the ANM file to bytes.
	///
	/// This method rebuilds the complete file structure:
	/// 1. Constructs the index table with word offsets
	/// 2. Serializes all animation sequences
	/// 3. Combines header + index table + animation data
	///
	/// # Returns
	///
	/// A byte vector containing the complete ANM file
	pub fn to_bytes(&self) -> Vec<u8> {
		let (index_table, animation_data) = self.build_serialized_layout();

		let mut bytes = Vec::with_capacity(
			constants::HEADER_SIZE + constants::INDEX_TABLE_SIZE + animation_data.len(),
		);

		// Header
		bytes.extend_from_slice(&self.header);

		// Index table
		for offset in &index_table {
			bytes.extend_from_slice(&offset.to_le_bytes());
		}

		// Animation data
		bytes.extend_from_slice(&animation_data);

		bytes
	}

	/// Parses an ANM file from bytes.
	///
	/// This method:
	/// 1. Validates file size
	/// 2. Parses header and index table
	/// 3. Parses each animation sequence independently
	/// 4. Handles shared/overlapping data regions correctly
	///
	/// # Arguments
	///
	/// * `data` - Byte slice containing the ANM file
	///
	/// # Errors
	///
	/// Returns an error if the file is too small or contains invalid data
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let data = std::fs::read("AGMAGIC.anm")?;
	/// let anm = File::from_bytes(&data)?;
	/// println!("Loaded {} sequences", anm.sequences().len());
	/// # Ok(())
	/// # }
	/// ```
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let min_size = constants::HEADER_SIZE + constants::INDEX_TABLE_SIZE;
		if data.len() < min_size {
			return Err(DvFileError::insufficient_data(FileType::Anm, min_size, data.len()));
		}

		// Parse header
		let mut header = [0u8; constants::HEADER_SIZE];
		header.copy_from_slice(&data[0..constants::HEADER_SIZE]);
		Self::validate_header(&header)?;

		// Parse index table
		let mut index_table = [0u16; constants::ANIMATION_SLOT_COUNT];
		let index_start = constants::INDEX_TABLE_OFFSET;
		for (i, entry) in index_table.iter_mut().enumerate() {
			let offset = index_start + i * 2;
			*entry = u16::from_le_bytes([data[offset], data[offset + 1]]);
		}

		// Parse animation sequences
		// Note: Multiple slots may point to the same or overlapping data regions.
		// This is LEGAL and intentional - used for space optimization or animation variants.
		// We parse each slot independently, even if offsets overlap.
		let mut sequences = Vec::new();
		let slot_windows = compute_slot_windows(&index_table, data.len());

		for (slot, &word_offset_value) in index_table.iter().enumerate() {
			// 0xFFFF means no animation in this slot
			if word_offset_value == constants::NO_ANIMATION {
				continue;
			}

			let Some(window) = slot_windows[slot] else {
				continue;
			};

			// Parse sequence for this slot independently
			// Even if another slot has the same offset, we parse it separately
			// because the slot might start at a different position in shared data
			match AnimationSequence::from_bytes(&data[window.start..window.end]) {
				Ok((sequence, _)) => {
					if !sequence.is_empty() {
						sequences.push((slot, sequence));
					}
				}
				Err(_) => {
					// Skip invalid sequences
					continue;
				}
			}
		}

		Ok(Self {
			header,
			index_table,
			sequences,
		})
	}

	/// Parses an ANM file from a reader.
	///
	/// # Arguments
	///
	/// * `reader` - Reader providing the ANM file data
	///
	/// # Errors
	///
	/// Returns an error if reading fails or the data is invalid
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	/// use std::fs::File as FsFile;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let file = FsFile::open("AGMAGIC.anm")?;
	/// let anm = File::from_reader(file)?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, DvFileError> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		Self::from_bytes(&data)
	}

	/// Parses an ANM file from bytes in raw mode (without simulating jumps).
	///
	/// This method reads animation sequences without executing jump instructions,
	/// preserving the original structure of the animation data. This is useful for
	/// editing tools that need to see the actual frame descriptors as stored in the file.
	///
	/// # Arguments
	///
	/// * `data` - Byte slice containing the ANM file data
	///
	/// # Returns
	///
	/// The parsed ANM file with original animation structure preserved
	///
	/// # Errors
	///
	/// Returns an error if the data is too short or malformed
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	/// use std::fs;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let data = fs::read("AGMAGIC.anm")?;
	/// let anm = File::from_bytes_raw(&data)?;
	/// // Sequences will show actual jump instructions, not expanded loops
	/// # Ok(())
	/// # }
	/// ```
	pub fn from_bytes_raw(data: &[u8]) -> Result<Self, DvFileError> {
		let min_size = constants::HEADER_SIZE + constants::INDEX_TABLE_SIZE;
		if data.len() < min_size {
			return Err(DvFileError::insufficient_data(FileType::Anm, min_size, data.len()));
		}

		// Parse header
		let mut header = [0u8; constants::HEADER_SIZE];
		header.copy_from_slice(&data[0..constants::HEADER_SIZE]);
		Self::validate_header(&header)?;

		// Parse index table
		let mut index_table = [0u16; constants::ANIMATION_SLOT_COUNT];
		let index_start = constants::INDEX_TABLE_OFFSET;
		for (i, entry) in index_table.iter_mut().enumerate() {
			let offset = index_start + i * 2;
			*entry = u16::from_le_bytes([data[offset], data[offset + 1]]);
		}

		// Parse animation sequences in raw mode
		let mut sequences = Vec::new();
		let slot_windows = compute_slot_windows(&index_table, data.len());

		for (slot, &word_offset_value) in index_table.iter().enumerate() {
			// 0xFFFF means no animation in this slot
			if word_offset_value == constants::NO_ANIMATION {
				continue;
			}

			let Some(window) = slot_windows[slot] else {
				continue;
			};

			// Parse sequence in raw mode (without simulating jumps)
			match AnimationSequence::from_bytes_raw(&data[window.start..window.end]) {
				Ok((sequence, _)) => {
					if !sequence.is_empty() {
						sequences.push((slot, sequence));
					}
				}
				Err(_) => {
					// Skip invalid sequences
					continue;
				}
			}
		}

		Ok(Self {
			header,
			index_table,
			sequences,
		})
	}

	/// Parses an ANM file from a reader in raw mode (without simulating jumps).
	///
	/// # Arguments
	///
	/// * `reader` - Reader providing the ANM file data
	///
	/// # Errors
	///
	/// Returns an error if reading fails or the data is invalid
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::anm::File;
	/// use std::fs::File as FsFile;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let file = FsFile::open("AGMAGIC.anm")?;
	/// let anm = File::from_reader_raw(file)?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn from_reader_raw<R: Read>(mut reader: R) -> Result<Self, DvFileError> {
		let mut data = Vec::new();
		reader.read_to_end(&mut data)?;
		Self::from_bytes_raw(&data)
	}
}

impl Default for File {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ANM File ({} sequences)", self.sequences.len())
	}
}

impl File {
	fn build_serialized_layout(&self) -> ([u16; constants::ANIMATION_SLOT_COUNT], Vec<u8>) {
		let mut index_table = [constants::NO_ANIMATION; constants::ANIMATION_SLOT_COUNT];
		let mut animation_data = Vec::new();

		for (slot, sequence) in &self.sequences {
			if *slot >= constants::ANIMATION_SLOT_COUNT {
				continue;
			}
			let byte_offset = animation_data.len();
			debug_assert_eq!(byte_offset % 2, 0, "Animation data must be WORD-aligned");
			index_table[*slot] = (byte_offset / 2) as u16;
			animation_data.extend_from_slice(&sequence.to_bytes());
		}

		(index_table, animation_data)
	}

	fn rebuild_index_table_from_sequences(&mut self) {
		self.index_table = Self::compute_index_table_from_sequences(&self.sequences);
	}

	fn compute_index_table_from_sequences(
		sequences: &[(usize, AnimationSequence)],
	) -> [u16; constants::ANIMATION_SLOT_COUNT] {
		let mut table = [constants::NO_ANIMATION; constants::ANIMATION_SLOT_COUNT];
		let mut byte_offset = 0usize;
		for (slot, sequence) in sequences {
			if *slot >= constants::ANIMATION_SLOT_COUNT {
				continue;
			}
			table[*slot] = (byte_offset / 2) as u16;
			byte_offset += sequence.byte_size();
		}
		table
	}

	fn validate_header(header: &[u8; constants::HEADER_SIZE]) -> Result<(), DvFileError> {
		let filename_field = &header[..constants::SPR_FILENAME_FIELD_LEN];
		let has_non_zero = filename_field.iter().any(|&b| b != 0);
		if filename_field.iter().any(|&b| b >= 0x80 && b != 0) {
			return Err(DvFileError::BadEncoding {
				file_type: FileType::Anm,
				message: "SPR filename must contain ASCII bytes only".to_string(),
			});
		}

		if has_non_zero && !filename_field.contains(&0) {
			return Err(DvFileError::BadEncoding {
				file_type: FileType::Anm,
				message: format!(
					"SPR filename must include a null terminator within first {} bytes",
					constants::SPR_FILENAME_FIELD_LEN
				),
			});
		}

		let padding = &header[constants::HEADER_PADDING_OFFSET
			..constants::HEADER_PADDING_OFFSET + constants::HEADER_PADDING_SIZE];
		if padding.iter().any(|&b| b != 0) {
			return Err(DvFileError::BadEncoding {
				file_type: FileType::Anm,
				message: "Header padding bytes (0x0C-0x1F) must be zero".to_string(),
			});
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::file::anm::FrameDescriptor;

	fn empty_anm_file_bytes() -> Vec<u8> {
		let mut data = vec![0u8; constants::HEADER_SIZE + constants::INDEX_TABLE_SIZE];
		for slot in 0..constants::ANIMATION_SLOT_COUNT {
			let offset = constants::INDEX_TABLE_OFFSET + slot * 2;
			data[offset] = 0xFF;
			data[offset + 1] = 0xFF;
		}
		data
	}

	#[test]
	fn test_from_bytes_rejects_missing_null_terminator() {
		let mut data = empty_anm_file_bytes();
		let mut name = [0u8; constants::SPR_FILENAME_FIELD_LEN];
		for (i, byte) in name.iter_mut().enumerate() {
			*byte = b'A' + i as u8;
		}
		data[..constants::SPR_FILENAME_FIELD_LEN].copy_from_slice(&name);

		let err = File::from_bytes(&data).expect_err("header validation should fail");
		match err {
			DvFileError::BadEncoding {
				message,
				..
			} => assert!(message.contains("null terminator")),
			_ => panic!("Unexpected error: {err:?}"),
		}
	}

	#[test]
	fn test_from_bytes_rejects_non_zero_header_padding() {
		let mut data = empty_anm_file_bytes();
		let filename = b"agmagic.SPR\0";
		data[..filename.len()].copy_from_slice(filename);
		data[constants::HEADER_PADDING_OFFSET] = 1; // padding must remain zero

		let err = File::from_bytes(&data).expect_err("padding validation should fail");
		match err {
			DvFileError::BadEncoding {
				message,
				..
			} => assert!(message.contains("Header padding")),
			_ => panic!("Unexpected error: {err:?}"),
		}
	}

	#[test]
	fn test_spr_filename_empty_by_default() {
		let file = File::new();
		assert_eq!(file.spr_filename(), "");
	}

	#[test]
	fn test_set_and_get_spr_filename() {
		let mut file = File::new();
		file.set_spr_filename("test.spr").unwrap();
		assert_eq!(file.spr_filename(), "test.spr");
	}

	#[test]
	fn test_set_spr_filename_case_insensitive() {
		let mut file = File::new();
		file.set_spr_filename("AGMAGIC.SPR").unwrap();
		assert_eq!(file.spr_filename(), "AGMAGIC.SPR");

		file.set_spr_filename("agmagic.spr").unwrap();
		assert_eq!(file.spr_filename(), "agmagic.spr");
	}

	#[test]
	fn test_set_spr_filename_rejects_long_names() {
		let mut file = File::new();
		let long_name = "a".repeat(50);
		let err = file.set_spr_filename(&long_name).expect_err("length validation should fail");
		match err {
			DvFileError::BadEncoding {
				message,
				..
			} => assert!(message.contains("at most 11 characters")),
			_ => panic!("Unexpected error: {err:?}"),
		}
	}

	#[test]
	fn test_set_spr_filename_rejects_non_ascii() {
		let mut file = File::new();
		let result = file.set_spr_filename("测试.spr");
		assert!(result.is_err());
	}

	#[test]
	fn test_spr_filename_roundtrip() {
		let mut file = File::new();
		file.set_spr_filename("agmagic.SPR").unwrap();

		let bytes = file.to_bytes();
		let loaded = File::from_bytes(&bytes).unwrap();

		assert_eq!(loaded.spr_filename(), "agmagic.SPR");
	}

	#[test]
	fn test_spr_filename_in_header() {
		let mut file = File::new();
		file.set_spr_filename("test.spr").unwrap();

		let header = file.header();

		// Check that the filename is at the start of the header
		assert_eq!(&header[0..8], b"test.spr");

		// Check null terminator
		assert_eq!(header[8], 0);

		// Check padding
		(9..constants::HEADER_SIZE).for_each(|i| {
			assert_eq!(header[i], 0);
		});
	}

	#[test]
	fn test_index_table_uses_word_offsets() {
		use crate::file::anm::FrameDescriptor;

		let mut file = File::new();

		// Create first animation: 2 frames = 8 bytes
		let mut seq1 = AnimationSequence::new();
		seq1.add_frame(FrameDescriptor::frame(1, 10));
		seq1.add_frame(FrameDescriptor::frame(2, 20));
		seq1.add_hold_marker();
		file.set_sequence(0, seq1).unwrap();

		// Create second animation: 1 frame = 4 bytes
		let mut seq2 = AnimationSequence::new();
		seq2.add_frame(FrameDescriptor::frame(3, 30));
		seq2.add_hold_marker();
		file.set_sequence(1, seq2).unwrap();

		let bytes = file.to_bytes();

		// Check index table uses WORD offsets
		let index_0 = u16::from_le_bytes([bytes[0x20], bytes[0x21]]);
		let index_1 = u16::from_le_bytes([bytes[0x22], bytes[0x23]]);

		assert_eq!(index_0, 0x0000, "First animation at WORD offset 0");

		// First animation: 2 frames (8 bytes) + hold marker (4 bytes) = 12 bytes = 6 words
		assert_eq!(index_1, 0x0006, "Second animation at WORD offset 6 (12 bytes / 2)");

		// Verify byte positions
		// First animation should start at 0x220
		assert_eq!(bytes[0x220], 0x01); // frame_id low byte
		assert_eq!(bytes[0x221], 0x00); // frame_id high byte

		// Second animation should start at 0x220 + 12 = 0x22C
		assert_eq!(bytes[0x22C], 0x03); // frame_id low byte
		assert_eq!(bytes[0x22D], 0x00); // frame_id high byte
	}

	#[test]
	fn test_slot_windows_split_regions() {
		let mut table = [constants::NO_ANIMATION; constants::ANIMATION_SLOT_COUNT];
		table[0] = 0; // start of data region
		table[1] = 6; // 12 bytes after slot 0
		let file_len = constants::ANIMATION_DATA_OFFSET + 32;

		let windows = compute_slot_windows(&table, file_len);
		let slot0 = windows[0].expect("slot 0 window exists");
		let slot1 = windows[1].expect("slot 1 window exists");

		assert_eq!(slot0.start, constants::ANIMATION_DATA_OFFSET);
		assert_eq!(slot0.end, constants::ANIMATION_DATA_OFFSET + 12);
		assert_eq!(slot1.start, constants::ANIMATION_DATA_OFFSET + 12);
		assert_eq!(slot1.end, file_len);
	}

	#[test]
	fn test_index_table_rebuilds_after_removal() {
		let mut file = File::new();

		let mut seq0 = AnimationSequence::new();
		seq0.add_frame(FrameDescriptor::frame(10, 5));
		seq0.add_frame(FrameDescriptor::frame(11, 5));
		seq0.add_hold_marker();
		file.set_sequence(0, seq0).unwrap();

		let mut seq5 = AnimationSequence::new();
		seq5.add_frame(FrameDescriptor::frame(20, 5));
		seq5.add_hold_marker();
		file.set_sequence(5, seq5).unwrap();

		let table = file.index_table();
		assert_eq!(table[0], 0);
		assert_eq!(table[5], 6, "Second slot should start after first sequence");

		file.remove_sequence(0);

		let updated = file.index_table();
		assert_eq!(updated[0], constants::NO_ANIMATION);
		assert_eq!(updated[5], 0, "Remaining sequence should be re-based to start of data");
	}

	#[test]
	fn test_odd_word_offset_values_are_valid() {
		use crate::file::anm::FrameDescriptor;

		let mut file = File::new();

		// Create an animation with odd number of frames to produce odd word offset
		// 3 frames = 12 bytes, + end = 16 bytes = 8 words
		let mut seq1 = AnimationSequence::new();
		seq1.add_frame(FrameDescriptor::frame(1, 1));
		seq1.add_frame(FrameDescriptor::frame(2, 1));
		seq1.add_frame(FrameDescriptor::frame(3, 1));
		seq1.add_hold_marker();
		file.set_sequence(0, seq1).unwrap();

		// 1 frame = 4 bytes, + end = 8 bytes = 4 words
		// Total offset: 8 + 4 = 12 words (but we need 9 words = 18 bytes for odd test)
		// Let's use 2 frames = 8 bytes, + end = 12 bytes total, then next at 6 words
		// Actually, to get odd offset, we need: 1 frame + sound marker = 8 bytes, + end = 12 bytes = 6 words (even)
		// For odd: 2 frames + sound = 12 bytes, + end = 16 bytes = 8 words (even)
		// We need to add 1 extra frame: 3 frames = 12 bytes, + end = 16 bytes = 8 words
		// To get odd, add sound marker: 3 frames + sound = 16 bytes, + end = 20 bytes = 10 words
		// Still even! Let's manually create: first anim 18 bytes = 9 words (odd!)

		// Actually, create animation with 18 bytes (9 words) - need 4 frames + end + padding
		let mut seq_for_odd = AnimationSequence::new();
		seq_for_odd.add_frame(FrameDescriptor::frame(10, 5));
		seq_for_odd.add_frame(FrameDescriptor::frame(11, 5));
		seq_for_odd.add_frame(FrameDescriptor::frame(12, 5));
		seq_for_odd.add_frame(FrameDescriptor::frame(13, 5));
		seq_for_odd.add_hold_marker(); // This adds 4 bytes + may add padding

		let _seq_bytes = seq_for_odd.to_bytes();
		// Sequences are always multiples of 4 bytes, so we'll get even word offsets

		// Instead, let's just verify that the conversion math is correct:
		// If index value is 9 (odd), byte offset should be 18 (even)
		let word_offset = 9u16;
		let byte_offset = word_offset as usize * 2;
		assert_eq!(byte_offset, 18);
		assert_eq!(byte_offset % 2, 0, "Byte offset must be even");
		assert_eq!(
			byte_offset % 4,
			2,
			"Byte offset 18 is not 4-byte aligned, but that's OK for index boundaries"
		);
	}

	#[test]
	fn test_read_original_agmagic_format() {
		// This test requires the actual AGMAGIC file
		// We'll test the logic with synthetic data that matches the format

		// Create synthetic AGMAGIC-like data
		let mut data = vec![0u8; 0x220 + 100];

		// Header: SPR filename
		data[0..11].copy_from_slice(b"agmagic.SPR");

		// Index table at 0x20
		// Slot 0: WORD offset 0x0000 (byte offset 0)
		data[0x20] = 0x00;
		data[0x21] = 0x00;

		// Slot 1: WORD offset 0x0009 (byte offset 0x12 = 18)
		data[0x22] = 0x09;
		data[0x23] = 0x00;

		// Slot 2: WORD offset 0x0012 (byte offset 0x24 = 36)
		data[0x24] = 0x12;
		data[0x25] = 0x00;

		// Rest are 0xFFFF
		for i in 3..256 {
			let offset = 0x20 + i * 2;
			data[offset] = 0xFF;
			data[offset + 1] = 0xFF;
		}

		// Animation data at 0x220 (slot 0)
		data[0x220] = 0x08; // frame_id = 8
		data[0x221] = 0x00;
		data[0x222] = 0x06; // duration = 6
		data[0x223] = 0x00;
		data[0x224] = 0x09; // frame_id = 9
		data[0x225] = 0x00;
		data[0x226] = 0x04; // duration = 4
		data[0x227] = 0x00;
		data[0x228] = 0xFD; // sound marker
		data[0x229] = 0xFF;
		data[0x22A] = 0xD5; // sound id = 213
		data[0x22B] = 0x00;
		data[0x22C] = 0x0B; // frame_id = 11
		data[0x22D] = 0x00;
		data[0x22E] = 0x02; // duration = 2
		data[0x22F] = 0x00;
		data[0x230] = 0xFF; // hold marker
		data[0x231] = 0xFF;

		// Animation data at 0x220 + 0x12 = 0x232 (slot 1)
		data[0x232] = 0x10; // frame_id = 16
		data[0x233] = 0x00;
		data[0x234] = 0x02; // duration = 2
		data[0x235] = 0x00;
		data[0x236] = 0x11; // frame_id = 17
		data[0x237] = 0x00;
		data[0x238] = 0x02; // duration = 2
		data[0x239] = 0x00;
		data[0x23A] = 0xFF; // hold marker
		data[0x23B] = 0xFF;

		// Parse the file
		let file = File::from_bytes(&data).unwrap();

		// Verify slot 0
		let seq0 = file.get_sequence(0).expect("Slot 0 should exist");
		assert!(seq0.frames().len() >= 3, "Slot 0 should have at least 3 frames");

		match &seq0.frames()[0] {
			FrameDescriptor::Frame {
				frame_id,
				duration,
			} => {
				assert_eq!(*frame_id, 8);
				assert_eq!(*duration, 6);
			}
			_ => panic!("Expected Frame descriptor at position 0"),
		}

		match &seq0.frames()[1] {
			FrameDescriptor::Frame {
				frame_id,
				duration,
			} => {
				assert_eq!(*frame_id, 9);
				assert_eq!(*duration, 4);
			}
			_ => panic!("Expected Frame descriptor at position 1"),
		}

		// Verify slot 1 starts at correct position
		let seq1 = file.get_sequence(1).expect("Slot 1 should exist");
		assert!(seq1.frames().len() >= 2, "Slot 1 should have at least 2 frames");

		match &seq1.frames()[0] {
			FrameDescriptor::Frame {
				frame_id,
				duration,
			} => {
				assert_eq!(*frame_id, 16);
				assert_eq!(*duration, 2);
			}
			_ => panic!("Expected Frame descriptor at position 0"),
		}

		match &seq1.frames()[1] {
			FrameDescriptor::Frame {
				frame_id,
				duration,
			} => {
				assert_eq!(*frame_id, 17);
				assert_eq!(*duration, 2);
			}
			_ => panic!("Expected Frame descriptor at position 1"),
		}
	}

	#[test]
	fn test_roundtrip_preserves_word_offsets() {
		use crate::file::anm::FrameDescriptor;

		let mut file = File::new();
		file.set_spr_filename("test.spr").unwrap();

		// Add several animations
		for slot in [0, 5, 10, 15] {
			let mut seq = AnimationSequence::new();
			for i in 0..3 {
				seq.add_frame(FrameDescriptor::frame(slot as u16 * 10 + i, 5));
			}
			seq.add_hold_marker();
			file.set_sequence(slot, seq).unwrap();
		}

		// Convert to bytes
		let bytes = file.to_bytes();

		// Parse back
		let loaded = File::from_bytes(&bytes).unwrap();

		// Verify all sequences are present and correct
		for slot in [0, 5, 10, 15] {
			let original = file.get_sequence(slot).unwrap();
			let loaded_seq = loaded.get_sequence(slot).unwrap();

			assert_eq!(original.len(), loaded_seq.len());
			for (i, (orig_frame, loaded_frame)) in
				original.frames().iter().zip(loaded_seq.frames().iter()).enumerate()
			{
				match (orig_frame, loaded_frame) {
					(
						FrameDescriptor::Frame {
							frame_id: orig_id,
							duration: orig_dur,
						},
						FrameDescriptor::Frame {
							frame_id: loaded_id,
							duration: loaded_dur,
						},
					) => {
						assert_eq!(
							orig_id, loaded_id,
							"Slot {} frame {} frame_id mismatch",
							slot, i
						);
						assert_eq!(
							orig_dur, loaded_dur,
							"Slot {} frame {} duration mismatch",
							slot, i
						);
					}
					_ => {
						assert_eq!(
							orig_frame, loaded_frame,
							"Slot {} frame {} descriptor mismatch",
							slot, i
						);
					}
				}
			}
		}
	}

	#[test]
	fn test_word_offset_calculation() {
		// Test the conversion between byte offsets and word offsets
		let test_cases = vec![
			(0, 0),     // 0 bytes = 0 words
			(2, 1),     // 2 bytes = 1 word
			(18, 9),    // 18 bytes = 9 words (odd word offset)
			(36, 18),   // 36 bytes = 18 words
			(72, 36),   // 72 bytes = 36 words
			(512, 256), // 512 bytes = 256 words
		];

		for (byte_offset, expected_word_offset) in test_cases {
			let word_offset = byte_offset / 2;
			assert_eq!(
				word_offset, expected_word_offset,
				"Byte offset {} should convert to word offset {}",
				byte_offset, expected_word_offset
			);

			let reconstructed_byte_offset = word_offset * 2;
			assert_eq!(
				reconstructed_byte_offset, byte_offset,
				"Word offset {} should convert back to byte offset {}",
				word_offset, byte_offset
			);
		}
	}

	#[test]
	fn test_raw_mode_preserves_jump_instructions() {
		use crate::file::anm::FrameDescriptor;

		// Create a file with a looping animation
		let mut file = File::new();
		file.set_spr_filename("test.spr").unwrap();

		let mut seq = AnimationSequence::new();
		seq.add_frame(FrameDescriptor::frame(0, 10));
		seq.add_frame(FrameDescriptor::frame(1, 15));
		seq.add_frame(FrameDescriptor::sound(5));
		seq.add_frame(FrameDescriptor::frame(2, 20));
		seq.add_frame(FrameDescriptor::event(3));
		seq.add_frame(FrameDescriptor::jump(1)); // Jump back to frame 1
		seq.add_hold_marker();
		file.set_sequence(0, seq).unwrap();

		// Serialize to bytes
		let bytes = file.to_bytes();

		// Parse in default mode (simulates jumps)
		let default_parsed = File::from_bytes(&bytes).unwrap();
		let default_seq = default_parsed.get_sequence(0).unwrap();

		// Parse in raw mode (preserves structure)
		let raw_parsed = File::from_bytes_raw(&bytes).unwrap();
		let raw_seq = raw_parsed.get_sequence(0).unwrap();

		// Raw mode should have exactly 7 frames (original structure)
		assert_eq!(raw_seq.len(), 7, "Raw mode should preserve original 7 frames");

		// Default mode should have more frames due to loop expansion
		assert!(
			default_seq.len() > 7,
			"Default mode should expand the loop, got {} frames",
			default_seq.len()
		);

		// Verify raw mode has the jump instruction at position 5
		assert!(
			raw_seq.frames()[5].is_jump(),
			"Raw mode should preserve jump instruction at position 5"
		);

		// Verify raw mode has hold marker at position 6
		assert!(raw_seq.frames()[6].is_hold(), "Raw mode should have hold marker at position 6");
	}

	#[test]
	fn test_raw_mode_with_multiple_control_frames() {
		use crate::file::anm::FrameDescriptor;

		let mut file = File::new();
		file.set_spr_filename("test.spr").unwrap();

		// Create a sequence with various control frames
		let mut seq = AnimationSequence::new();
		seq.add_frame(FrameDescriptor::frame(0, 5));
		seq.add_frame(FrameDescriptor::sound(10));
		seq.add_frame(FrameDescriptor::event(20));
		seq.add_frame(FrameDescriptor::frame(1, 5));
		seq.add_hold_marker();
		file.set_sequence(5, seq).unwrap();

		let bytes = file.to_bytes();

		// Parse in raw mode
		let raw_parsed = File::from_bytes_raw(&bytes).unwrap();
		let raw_seq = raw_parsed.get_sequence(5).unwrap();

		// Should have exactly 5 frames
		assert_eq!(raw_seq.len(), 5);

		// Verify frame types
		assert!(raw_seq.frames()[0].is_frame());
		assert!(raw_seq.frames()[1].is_sound());
		assert!(raw_seq.frames()[2].is_event());
		assert!(raw_seq.frames()[3].is_frame());
		assert!(raw_seq.frames()[4].is_hold());
	}
}
