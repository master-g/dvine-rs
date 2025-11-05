//! File construction and serialization for EFC files.
//!
//! This module contains functionality for creating new EFC files,
//! modifying existing files, and serializing them to disk.

use std::fs::File as FsFile;
use std::io::{Cursor, Write};
use std::path::Path;

use crate::file::{DvFileError, FileType};

use super::File;
use super::constants::MAX_EFFECTS;
use super::encoder;
use super::types::DecodedSound;

impl DecodedSound {
	/// Encodes the PCM data back to ADPCM format
	///
	/// Returns the complete effect data including headers and ADPCM data
	pub fn to_bytes(&self) -> Result<Vec<u8>, DvFileError> {
		let mut buffer = Vec::new();

		// Write sound data header (4 bytes)
		buffer.push(self.sound_header.sound_type);
		buffer.push(self.sound_header.unknown_1);
		buffer.extend_from_slice(&self.sound_header.priority.to_le_bytes());

		// Write ADPCM data header (0xC0 bytes)
		// Track position within ADPCM header
		let adpcm_header_start = buffer.len();

		buffer.extend_from_slice(&self.adpcm_header.sample_rate.to_le_bytes());
		buffer.extend_from_slice(&self.adpcm_header.channels.to_le_bytes());
		buffer.extend_from_slice(&self.adpcm_header.unknown.to_le_bytes());

		// Write step table (89 entries * 2 bytes = 178 bytes)
		for &step in &self.adpcm_header.step_table {
			buffer.extend_from_slice(&step.to_le_bytes());
		}

		// Padding to reach offset 0xBC within ADPCM header
		// 0xBC = 188 bytes from start of ADPCM header
		let current_adpcm_len = buffer.len() - adpcm_header_start;
		if current_adpcm_len < 0xBC {
			let padding_needed = 0xBC - current_adpcm_len;
			buffer.resize(buffer.len() + padding_needed, 0);
		}

		// Write sample count at offset 0xBC (within ADPCM header)
		buffer.extend_from_slice(&self.adpcm_header.sample_count.to_le_bytes());

		// Encode PCM to ADPCM
		let adpcm_data = encoder::encode_ima_adpcm(
			&self.pcm_data,
			&self.adpcm_header.step_table,
			self.adpcm_header.channels,
		)?;

		// Write ADPCM data
		buffer.extend_from_slice(&adpcm_data);

		Ok(buffer)
	}
}

impl Default for File<Cursor<Vec<u8>>> {
	fn default() -> Self {
		Self::new()
	}
}

impl File<Cursor<Vec<u8>>> {
	/// Creates a new empty `.EFC` file
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::efc::File;
	///
	/// let efc = File::new();
	/// assert_eq!(efc.effect_count(), 0);
	/// ```
	pub fn new() -> Self {
		let index_table = [0u32; MAX_EFFECTS];
		let cache = Box::new(std::array::from_fn(|_| None));
		let reader = Cursor::new(Vec::new());

		Self {
			reader,
			index_table,
			cache,
		}
	}

	/// Inserts or updates a sound effect at the given ID
	///
	/// # Arguments
	/// * `id` - Effect ID (0 to 255)
	/// * `sound` - The decoded sound to insert
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::efc::{File, DecodedSound, SoundDataHeader, AdpcmDataHeader};
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut efc = File::new();
	///
	/// let sound = DecodedSound {
	///     id: 0,
	///     sound_header: SoundDataHeader {
	///         sound_type: 1,
	///         unknown_1: 0,
	///         priority: 100,
	///     },
	///     adpcm_header: AdpcmDataHeader {
	///         sample_rate: 22050,
	///         channels: 1,
	///         unknown: 0,
	///         step_table: [7; 89],
	///         sample_count: 1000,
	///     },
	///     pcm_data: vec![0i16; 1000],
	/// };
	///
	/// efc.insert_effect(42, sound)?;
	/// assert!(efc.has_effect(42));
	/// # Ok(())
	/// # }
	/// ```
	pub fn insert_effect(&mut self, id: usize, sound: DecodedSound) -> Result<(), DvFileError> {
		if id >= MAX_EFFECTS {
			return Err(DvFileError::EntryNotFound {
				file_type: FileType::Efc,
				message: format!("Effect ID {} out of range (max {})", id, MAX_EFFECTS - 1),
			});
		}

		// Store in cache
		self.cache[id] = Some(Box::new(sound));

		// Mark as present in index table (actual offset will be calculated in to_bytes)
		self.index_table[id] = 1; // Non-zero to indicate presence

		Ok(())
	}

	/// Removes a sound effect at the given ID
	///
	/// # Arguments
	/// * `id` - Effect ID (0 to 255)
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::efc::File;
	///
	/// let mut efc = File::new();
	/// efc.remove_effect(42);
	/// assert!(!efc.has_effect(42));
	/// ```
	pub fn remove_effect(&mut self, id: usize) {
		if id < MAX_EFFECTS {
			self.cache[id] = None;
			self.index_table[id] = 0;
		}
	}

	/// Serializes the `.EFC` file to bytes
	///
	/// This method rebuilds the entire file structure including the index table
	/// and all effect data.
	///
	/// # Examples
	///
	/// ```
	/// use dvine_types::file::efc::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let efc = File::new();
	/// let bytes = efc.to_bytes()?;
	/// assert_eq!(bytes.len(), 256 * 4); // Index table only for empty file
	/// # Ok(())
	/// # }
	/// ```
	pub fn to_bytes(&self) -> Result<Vec<u8>, DvFileError> {
		let mut buffer = Vec::new();

		// Reserve space for index table (256 * 4 = 1024 bytes)
		let index_table_size = MAX_EFFECTS * 4;
		buffer.resize(index_table_size, 0);

		// Write effects and build index table
		let mut new_index_table = [0u32; MAX_EFFECTS];
		let mut current_offset = index_table_size as u32;

		for id in 0..MAX_EFFECTS {
			if let Some(ref sound) = self.cache[id] {
				// Record offset in index table
				new_index_table[id] = current_offset;

				// Encode effect data
				let effect_data = sound.to_bytes()?;

				// Write effect data
				buffer.extend_from_slice(&effect_data);

				// Update offset for next effect
				current_offset += effect_data.len() as u32;
			}
		}

		// Write index table at the beginning
		for (i, &offset) in new_index_table.iter().enumerate() {
			let pos = i * 4;
			buffer[pos..pos + 4].copy_from_slice(&offset.to_le_bytes());
		}

		Ok(buffer)
	}

	/// Writes the `.EFC` file to the given writer
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::efc::File;
	/// use std::fs::File as FsFile;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let efc = File::new();
	/// let mut file = FsFile::create("output.EFC")?;
	/// efc.write_to(&mut file)?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), DvFileError> {
		let bytes = self.to_bytes()?;
		writer.write_all(&bytes)?;
		Ok(())
	}

	/// Saves the `.EFC` file to the given path
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::efc::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let efc = File::new();
	/// efc.save_to_file("output.EFC")?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), DvFileError> {
		let mut file = FsFile::create(path)?;
		self.write_to(&mut file)
	}
}
