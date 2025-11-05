//! Data type definitions for EFC files.
//!
//! This module contains the core data structures used to represent
//! sound effects and their metadata in EFC files.

use std::fmt::Display;
use std::io::{Read, Seek, Write};

use crate::file::DvFileError;

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
