//! ANM file structure and I/O operations.
//!
//! This module defines the main `File` struct which represents a complete ANM
//! (Animation) file with header, index table, and animation sequences.

use std::io::Read;

use crate::file::{DvFileError, FileType};

use super::{constants, sequence::AnimationSequence};

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
/// seq.add_end_marker();
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

	/// Index table mapping slots to animation data offsets (word offsets)
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
			index_table: [0u16; constants::ANIMATION_SLOT_COUNT],
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

	/// Returns a reference to the index table.
	///
	/// The index table contains word offsets (multiply by 2 to get byte offsets).
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
	///
	/// The word offset from the animation data start (0x220), or None if slot is out of range.
	/// Multiply by 2 to get byte offset.
	pub fn get_slot_offset(&self, slot: usize) -> Option<u16> {
		if slot < constants::ANIMATION_SLOT_COUNT {
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
	pub fn sequences_mut(&mut self) -> &mut Vec<(usize, AnimationSequence)> {
		&mut self.sequences
	}

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
	/// seq.add_end_marker();
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
			Some(self.sequences.remove(pos).1)
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
	/// seq.add_end_marker();
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
		// Rebuild index table and animation data
		let mut index_table = [constants::NO_ANIMATION; constants::ANIMATION_SLOT_COUNT];
		let mut animation_data = Vec::new();

		for (slot, sequence) in &self.sequences {
			// Set offset in index table (convert byte offset to word offset by dividing by 2)
			let byte_offset = animation_data.len();
			let word_offset = (byte_offset / 2) as u16;
			index_table[*slot] = word_offset;

			// Append sequence data
			animation_data.extend_from_slice(&sequence.to_bytes());
		}

		// Build complete file
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

		for (slot, &offset_value) in index_table.iter().enumerate() {
			// 0xFFFF means no animation in this slot
			if offset_value == constants::NO_ANIMATION {
				continue;
			}

			// Convert word offset to byte offset (multiply by 2)
			let byte_offset = (offset_value as usize) * 2;
			let data_offset = constants::ANIMATION_DATA_OFFSET + byte_offset;

			if data_offset >= data.len() {
				continue; // Invalid offset, skip
			}

			// Parse sequence for this slot independently
			// Even if another slot has the same offset, we parse it separately
			// because the slot might start at a different position in shared data
			match AnimationSequence::from_bytes(&data[data_offset..]) {
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
