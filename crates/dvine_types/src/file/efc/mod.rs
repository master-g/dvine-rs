//! Sound effect file support for `dvine-rs` project.
//!
//! This module provides support for reading and decoding `.EFC` (Effect) files,
//! which contain sound effects encoded in IMA ADPCM format.
//!
//! # File Structure
//!
//! EFC files begin with an index table of 256 entries (4 bytes each), where each
//! entry is a file offset to a sound effect. A value of 0 indicates no effect at
//! that index.
//!
//! Each sound effect entry contains:
//! - Sound data header (4 bytes)
//! - ADPCM data header (0xC0 bytes) including step table and sample count
//! - IMA ADPCM encoded audio data
//!
//! # Examples
//!
//! ## Opening and extracting a sound effect
//!
//! ```no_run
//! use dvine_types::file::efc::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open an EFC file
//! let mut efc = File::open("SOUND.EFC")?;
//!
//! // Check if an effect exists
//! if efc.has_effect(42) {
//!     // Extract and decode the effect
//!     let sound = efc.extract(42)?;
//!
//!     println!("Effect ID: {}", sound.id);
//!     println!("Duration: {} ms", sound.duration_ms());
//!     println!("Sample rate: {} Hz", sound.adpcm_header.sample_rate);
//!     println!("Channels: {}", sound.adpcm_header.channels);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Exporting to WAV file
//!
//! ```no_run
//! use dvine_types::file::efc::File;
//! use std::fs::File as FsFile;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut efc = File::open("SOUND.EFC")?;
//! let sound = efc.extract(42)?;
//!
//! // Write to WAV file
//! let mut wav_file = FsFile::create("effect_42.wav")?;
//! sound.write(&mut wav_file)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Listing all available effects
//!
//! ```no_run
//! use dvine_types::file::efc::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut efc = File::open("SOUND.EFC")?;
//!
//! println!("Total effects: {}", efc.effect_count());
//!
//! for effect_info in efc.list_effects() {
//!     println!("Effect ID {}: offset 0x{:08X}", effect_info.id, effect_info.offset);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Using iterators
//!
//! ```no_run
//! use dvine_types::file::efc::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut efc = File::open("SOUND.EFC")?;
//!
//! // Iterate over effect info (no decoding)
//! for info in efc.iter_info() {
//!     println!("Effect {}: offset 0x{:08X}", info.id, info.offset);
//! }
//!
//! // Iterate over decoded sounds (decodes on-demand)
//! for result in efc.iter_sounds() {
//!     match result {
//!         Ok(sound) => {
//!             println!("Effect {}: {} ms, {} Hz",
//!                 sound.id, sound.duration_ms(), sound.adpcm_header.sample_rate);
//!         }
//!         Err(e) => eprintln!("Error decoding: {}", e),
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod decoder;

use std::{
	fmt::Display,
	io::{Read, Seek, Write},
};

use crate::file::{DvFileError, efc::constants::MAX_EFFECTS};

/// Constants used in `.EFC` files
pub mod constants {
	/// Maximum number of effects supported in `.EFC` files
	pub const MAX_EFFECTS: usize = 256;
}

/// File structure for `.EFC` files
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File<R> {
	/// Underlying reader for file operations
	reader: R,

	/// Index table mapping effect IDs to file offsets
	index_table: [u32; constants::MAX_EFFECTS],

	/// Cache for decoded effects
	cache: Box<[Option<Box<DecodedSound>>; MAX_EFFECTS]>,
}

/// Sound data header structure
/// Located at the start of each sound effect entry in the `.EFC` file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundDataHeader {
	/// Sound type identifier, we don't know how it works yet
	pub sound_type: u8,
	/// Unknown field, purpose is unclear
	pub unknown_1: u8,
	/// Priority level of the sound effect
	pub priority: u16,
}

impl Display for SoundDataHeader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"SoundDataHeader:\n\
			- Sound Type: {}\n\
			- Unknown 1: {}\n\
			- Priority: {}",
			self.sound_type, self.unknown_1, self.priority
		)
	}
}

impl SoundDataHeader {
	/// Reads a `SoundDataHeader` from the given reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut buffer = [0u8; 4];
		reader.read_exact(&mut buffer)?;
		let priority = u16::from_le_bytes([buffer[2], buffer[3]]);
		Ok(Self {
			sound_type: buffer[0],
			unknown_1: buffer[1],
			priority,
		})
	}
}

/// ADPCM sound data header structure
/// Located at offset + 4 from the start of each sound effect entry
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdpcmDataHeader {
	/// Sample rate in Hz (typically 22050)
	pub sample_rate: u32,
	/// Number of channels (1 for mono, 2 for stereo)
	pub channels: u16,
	/// Unknown field, purpose is unclear
	pub unknown: u16,
	/// IMA ADPCM step table (89 entries)
	pub step_table: [i16; 89],
	/// Number of PCM samples (located at offset 0xBC from ADPCM header start)
	pub sample_count: u32,
}

impl Display for AdpcmDataHeader {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"AdpcmDataHeader:\n\
			- Sample Rate: {} Hz\n\
			- Channels: {}\n\
			- Unknown: {}\n\
			- Sample Count: {}",
			self.sample_rate, self.channels, self.unknown, self.sample_count
		)
	}
}

impl AdpcmDataHeader {
	/// Reads `AdpcmDataHeader` from the given reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut buffer = [0u8; 0xC0];
		reader.read_exact(&mut buffer)?;

		let sample_rate = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
		let channels = u16::from_le_bytes([buffer[4], buffer[5]]);
		let unknown = u16::from_le_bytes([buffer[6], buffer[7]]);

		// parse step table
		let mut step_table = [0i16; 89];
		(0..89).for_each(|i| {
			let offset = 8 + i * 2;
			step_table[i] = i16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
		});

		let sample_count =
			u32::from_le_bytes([buffer[0xBC], buffer[0xBD], buffer[0xBE], buffer[0xBF]]);

		Ok(Self {
			sample_rate,
			channels,
			unknown,
			step_table,
			sample_count,
		})
	}

	/// Read `AdpcmDataHeader` from bytes
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, DvFileError> {
		let mut cursor = std::io::Cursor::new(bytes);
		Self::from_reader(&mut cursor)
	}
}

/// Information about a sound effect in the `.EFC` file
/// this structure is constructed when reading the effect index table
/// not directly from the file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectInfo {
	/// Effect ID (0 to 255)
	pub id: usize,
	/// Offset in the file where the effect data starts
	pub offset: u32,
	/// Sound data header located at the start of the effect data
	pub header: SoundDataHeader,
}

/// Decoded sound effect data ready for playback or output
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecodedSound {
	/// Effect ID
	pub id: usize,
	/// Sound data header
	pub sound_header: SoundDataHeader,
	/// ADPCM data header
	pub adpcm_header: AdpcmDataHeader,
	/// Decoded PCM data samples
	pub pcm_data: Vec<i16>,
}

impl DecodedSound {
	/// Calculates the duration of the sound effect in milliseconds
	pub fn duration_ms(&self) -> u32 {
		let total_samples = self.adpcm_header.sample_count;
		let sample_rate = self.adpcm_header.sample_rate;
		if sample_rate == 0 {
			return 0;
		}
		(total_samples * 1000) / sample_rate
	}

	/// Writes the decoded sound effect as a WAV file to the given writer
	pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), DvFileError> {
		let spec = hound::WavSpec {
			channels: self.adpcm_header.channels,
			sample_rate: self.adpcm_header.sample_rate,
			bits_per_sample: 16,
			sample_format: hound::SampleFormat::Int,
		};

		let mut wav_writer = hound::WavWriter::new(writer, spec)?;

		for &sample in &self.pcm_data {
			wav_writer.write_sample(sample)?;
		}

		wav_writer.finalize()?;

		Ok(())
	}
}

impl<R: Read + Seek> File<R> {
	/// Reads an `.EFC` file from the given reader
	pub fn from_reader(mut reader: R) -> Result<Self, DvFileError> {
		let mut index_table = [0u32; constants::MAX_EFFECTS];
		for entry in &mut index_table {
			let mut buffer = [0u8; 4];
			reader.read_exact(&mut buffer)?;
			*entry = u32::from_le_bytes(buffer);
		}

		let cache = Box::new(std::array::from_fn(|_| None));

		Ok(Self {
			reader,
			index_table,
			cache,
		})
	}

	/// Gets the offset and size of the effect with the given ID
	fn get_offset(&mut self, id: usize) -> Result<(u32, u32), DvFileError> {
		if id >= constants::MAX_EFFECTS {
			return Err(DvFileError::EntryNotFound {
				file_type: super::FileType::Efc,
				message: format!("Effect ID {} out of range", id),
			});
		}
		let offset = self.index_table[id];
		if offset == 0 {
			return Err(DvFileError::EntryNotFound {
				file_type: super::FileType::Efc,
				message: format!("Effect ID {} is not a valid entry", id),
			});
		}

		// Find the next valid offset (skip entries with offset 0)
		let mut next_offset = 0u32;
		for i in (id + 1)..constants::MAX_EFFECTS {
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
			file_type: super::FileType::Efc,
			message: format!(
				"Effect ID {} has invalid offset: next_offset {} < offset {}",
				id, next_offset, offset
			),
		})?;

		Ok((offset, size))
	}

	/// Extracts and decodes the sound effect with the given ID
	pub fn extract(&mut self, id: usize) -> Result<&DecodedSound, DvFileError> {
		// Check cache first
		if self.cache[id].is_some() {
			return Ok(self.cache[id].as_ref().unwrap());
		}

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

		// Create decoded sound
		let decoded = DecodedSound {
			id,
			sound_header,
			adpcm_header,
			pcm_data,
		};

		// Store in cache
		self.cache[id] = Some(Box::new(decoded));

		// Return reference
		Ok(self.cache[id].as_ref().unwrap())
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
		id < constants::MAX_EFFECTS && self.index_table[id] != 0
	}

	/// Returns the number of available effects
	pub fn effect_count(&self) -> usize {
		self.index_table.iter().filter(|&&offset| offset != 0).count()
	}

	/// Clears the cache for a specific effect
	pub fn clear_cache(&mut self, id: usize) {
		if id < constants::MAX_EFFECTS {
			self.cache[id] = None;
		}
	}

	/// Clears all cached effects
	pub fn clear_all_cache(&mut self) {
		for entry in self.cache.iter_mut() {
			*entry = None;
		}
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
	/// Returns cloned `DecodedSound` instances. For cached access without cloning,
	/// use `extract()` directly.
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

/// Iterator over effect information
///
/// This iterator provides basic information about each effect without decoding.
pub struct EffectInfoIter<'a> {
	index_table: &'a [u32; constants::MAX_EFFECTS],
	current_id: usize,
}

impl<'a> Iterator for EffectInfoIter<'a> {
	type Item = EffectInfo;

	fn next(&mut self) -> Option<Self::Item> {
		// Find next valid effect
		while self.current_id < constants::MAX_EFFECTS {
			let id = self.current_id;
			let offset = self.index_table[id];
			self.current_id += 1;

			if offset != 0 {
				return Some(EffectInfo {
					id,
					offset,
					// Placeholder header - we can't read it without mutating the file
					header: SoundDataHeader {
						sound_type: 0,
						unknown_1: 0,
						priority: 0,
					},
				});
			}
		}

		None
	}
}

/// Iterator over decoded sounds
///
/// This iterator decodes effects on-demand. If an effect fails to decode,
/// the iterator returns an error for that effect and continues to the next one.
///
/// Returns owned `DecodedSound` instances (cloned from cache).
pub struct DecodedSoundIter<'a, R> {
	file: &'a mut File<R>,
	current_id: usize,
}

impl<'a, R: Read + Seek> Iterator for DecodedSoundIter<'a, R> {
	type Item = Result<DecodedSound, DvFileError>;

	fn next(&mut self) -> Option<Self::Item> {
		// Find next valid effect
		while self.current_id < constants::MAX_EFFECTS {
			let id = self.current_id;
			self.current_id += 1;

			if self.file.index_table[id] != 0 {
				// Try to extract the effect
				let result = self.file.extract(id);
				return Some(match result {
					Ok(sound) => Ok(sound.clone()),
					Err(e) => Err(e),
				});
			}
		}

		None
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

#[cfg(test)]
mod tests {
	use super::*;
	use std::io::Cursor;

	fn create_test_efc() -> File<Cursor<Vec<u8>>> {
		// Create a minimal EFC file with index table
		let mut data = Vec::new();

		// Index table (256 entries, 4 bytes each)
		// Effect 0 at offset 0x400
		data.extend_from_slice(&0x400u32.to_le_bytes());
		// Effect 1 at offset 0x500
		data.extend_from_slice(&0x500u32.to_le_bytes());
		// Rest are 0 (no effect)
		for _ in 2..256 {
			data.extend_from_slice(&0u32.to_le_bytes());
		}

		let reader = Cursor::new(data);
		File::from_reader(reader).unwrap()
	}

	#[test]
	fn test_iter_info() {
		let efc = create_test_efc();
		let effects: Vec<_> = efc.iter_info().collect();

		assert_eq!(effects.len(), 2);
		assert_eq!(effects[0].id, 0);
		assert_eq!(effects[0].offset, 0x400);
		assert_eq!(effects[1].id, 1);
		assert_eq!(effects[1].offset, 0x500);
	}

	#[test]
	fn test_iter_alias() {
		let efc = create_test_efc();
		let effects: Vec<_> = efc.iter().collect();

		assert_eq!(effects.len(), 2);
	}

	#[test]
	fn test_effect_count() {
		let efc = create_test_efc();
		assert_eq!(efc.effect_count(), 2);
	}

	#[test]
	fn test_has_effect() {
		let efc = create_test_efc();
		assert!(efc.has_effect(0));
		assert!(efc.has_effect(1));
		assert!(!efc.has_effect(2));
		assert!(!efc.has_effect(255));
	}

	#[test]
	fn test_list_effects() {
		let efc = create_test_efc();
		let effects = efc.list_effects();

		assert_eq!(effects.len(), 2);
		assert_eq!(effects[0].id, 0);
		assert_eq!(effects[1].id, 1);
	}
}
