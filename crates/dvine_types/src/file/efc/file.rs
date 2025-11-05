//! Core file operations for EFC files.
//!
//! This module contains the main `File` structure and operations for
//! reading and extracting sound effects from EFC files.

use std::io::{Read, Seek};

use crate::file::{DvFileError, FileType};

use super::constants::MAX_EFFECTS;
use super::decoder;
use super::iterator::{DecodedSoundIter, EffectInfoIter};
use super::types::{AdpcmDataHeader, DecodedSound, EffectInfo, SoundDataHeader};

/// File structure for `.EFC` files
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File<R> {
	/// Underlying reader for file operations
	pub(super) reader: R,

	/// Index table mapping effect IDs to file offsets
	pub(super) index_table: [u32; MAX_EFFECTS],
}

impl<R: Read + Seek> File<R> {
	/// Reads an `.EFC` file from the given reader
	pub fn from_reader(mut reader: R) -> Result<Self, DvFileError> {
		let mut index_table = [0u32; MAX_EFFECTS];
		for entry in &mut index_table {
			let mut buffer = [0u8; 4];
			reader.read_exact(&mut buffer)?;
			*entry = u32::from_le_bytes(buffer);
		}

		Ok(Self {
			reader,
			index_table,
		})
	}

	/// Gets the offset and size of the effect with the given ID
	fn get_offset(&mut self, id: usize) -> Result<(u32, u32), DvFileError> {
		if id >= MAX_EFFECTS {
			return Err(DvFileError::EntryNotFound {
				file_type: FileType::Efc,
				message: format!("Effect ID {} out of range", id),
			});
		}
		let offset = self.index_table[id];
		if offset == 0 {
			return Err(DvFileError::EntryNotFound {
				file_type: FileType::Efc,
				message: format!("Effect ID {} is not a valid entry", id),
			});
		}

		// Find the next valid offset (skip entries with offset 0)
		let mut next_offset = 0u32;
		for i in (id + 1)..MAX_EFFECTS {
			if self.index_table[i] != 0 {
				next_offset = self.index_table[i];
				break;
			}
		}

		// If no next valid offset found, use file size
		if next_offset == 0 {
			next_offset = self.reader.seek(std::io::SeekFrom::End(0))? as u32;
		}

		// Calculate size with overflow check
		let size = next_offset.checked_sub(offset).ok_or_else(|| DvFileError::EntryNotFound {
			file_type: FileType::Efc,
			message: format!(
				"Effect ID {} has invalid offset: next_offset {} < offset {}",
				id, next_offset, offset
			),
		})?;

		Ok((offset, size))
	}

	/// Extracts and decodes the sound effect with the given ID
	///
	/// Returns an owned `DecodedSound`. If you need to extract the same effect
	/// multiple times, consider caching the result yourself.
	pub fn extract(&mut self, id: usize) -> Result<DecodedSound, DvFileError> {
		let (offset, size) = self.get_offset(id)?;

		// move reader to the effect offset
		self.reader.seek(std::io::SeekFrom::Start(offset as u64))?;

		// parse effect header
		let sound_header = SoundDataHeader::from_reader(&mut self.reader)?;

		// parse adpcm header
		let adpcm_header = AdpcmDataHeader::from_reader(&mut self.reader)?;

		// Calculate ADPCM data size
		// Size = 4-byte header + ceil(sample_count * channels / 2) bytes of nibbles
		let mut adpcm_size =
			4 + (adpcm_header.sample_count as usize * adpcm_header.channels as usize).div_ceil(2);

		// Adjust size based on available data
		// The size from offset calculation already accounts for the headers (4 + 0xC0 bytes)
		// so we need to subtract those from the total size
		let header_size = 4 + 0xC0;
		if adpcm_size > size as usize - header_size {
			adpcm_size = size as usize - header_size;
		}

		// Read ADPCM data
		let mut adpcm_data = vec![0u8; adpcm_size];
		self.reader.read_exact(&mut adpcm_data)?;

		// Decode ADPCM to PCM
		let pcm_data = decoder::decode_ima_adpcm(
			&adpcm_data,
			&adpcm_header.step_table,
			adpcm_header.channels,
			adpcm_header.sample_count,
		)?;

		// Create and return decoded sound
		Ok(DecodedSound {
			id,
			sound_header,
			adpcm_header,
			pcm_data,
		})
	}

	/// Returns a list of all available effect IDs and their offsets
	pub fn list_effects(&self) -> Vec<EffectInfo> {
		let mut effects = Vec::new();

		for (id, &offset) in self.index_table.iter().enumerate() {
			if offset != 0 {
				effects.push(EffectInfo {
					id,
					offset,
					// We would need to read the header to get this, but we can't
					// do that without mutating self. For now, use a placeholder.
					header: SoundDataHeader {
						sound_type: 0,
						unknown_1: 0,
						priority: 0,
					},
				});
			}
		}

		effects
	}

	/// Checks if an effect with the given ID exists
	pub fn has_effect(&self, id: usize) -> bool {
		id < MAX_EFFECTS && self.index_table[id] != 0
	}

	/// Returns the number of available effects
	pub fn effect_count(&self) -> usize {
		self.index_table.iter().filter(|&&offset| offset != 0).count()
	}

	/// Returns an iterator over effect information
	///
	/// This iterator returns basic information about each effect without decoding.
	/// It's useful for quickly listing available effects.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::efc::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let efc = File::open("SOUND.EFC")?;
	///
	/// for info in efc.iter_info() {
	///     println!("Effect {}: offset 0x{:08X}", info.id, info.offset);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub fn iter_info(&self) -> EffectInfoIter<'_> {
		EffectInfoIter {
			index_table: &self.index_table,
			current_id: 0,
		}
	}

	/// Returns an iterator over decoded sounds
	///
	/// This iterator decodes effects on-demand as they are accessed.
	/// Each call to `next()` performs decoding, so consider caching results
	/// if you need to access the same effect multiple times.
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::efc::File;
	///
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut efc = File::open("SOUND.EFC")?;
	///
	/// for result in efc.iter_sounds() {
	///     match result {
	///         Ok(sound) => {
	///             println!("Effect {}: {} ms", sound.id, sound.duration_ms());
	///         }
	///         Err(e) => {
	///             eprintln!("Failed to decode effect: {}", e);
	///         }
	///     }
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub fn iter_sounds(&mut self) -> DecodedSoundIter<'_, R> {
		DecodedSoundIter {
			file: self,
			current_id: 0,
		}
	}

	/// Returns an iterator over all effects in the file
	///
	/// This is an alias for `iter_info()` that allows using the file as an iterable.
	pub fn iter(&self) -> EffectInfoIter<'_> {
		self.iter_info()
	}
}

impl File<std::io::BufReader<std::fs::File>> {
	/// Opens an `.EFC` file from the given path
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let file = std::fs::File::open(path)?;
		let reader = std::io::BufReader::new(file);
		Self::from_reader(reader)
	}
}
