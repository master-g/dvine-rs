//! Sound effect file support for `dvine-rs` project.
//!
//! This module provides comprehensive support for reading, decoding, creating, and modifying
//! `.EFC` (Effect) files, which contain sound effects encoded in IMA ADPCM format.
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
//! # Features
//!
//! - **Reading & Decoding**: Extract and decode ADPCM-compressed sound effects to PCM
//! - **Writing & Encoding**: Create new EFC files and encode PCM data to ADPCM
//! - **Modification**: Insert, update, and remove sound effects
//! - **Export**: Save decoded sounds as WAV files
//! - **Iteration**: Iterate over effects with or without decoding
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
//! ## Creating a new EFC file
//!
//! ```no_run
//! use dvine_types::file::efc::{File, DecodedSound, SoundDataHeader, AdpcmDataHeader};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new empty EFC file
//! let mut efc = File::new();
//!
//! // Create a step table for ADPCM encoding
//! let mut step_table = [0i16; 89];
//! for i in 0..89 {
//!     step_table[i] = 7 + i as i16 * 8;
//! }
//!
//! // Create a sound effect with PCM data
//! let sound = DecodedSound {
//!     id: 10,
//!     sound_header: SoundDataHeader {
//!         sound_type: 1,
//!         unknown_1: 0,
//!         priority: 100,
//!     },
//!     adpcm_header: AdpcmDataHeader {
//!         sample_rate: 22050,
//!         channels: 1,
//!         unknown: 0,
//!         step_table,
//!         sample_count: 1000,
//!     },
//!     pcm_data: vec![0i16; 1000],
//! };
//!
//! // Insert the sound effect
//! efc.insert_effect(10, sound)?;
//!
//! // Save to file
//! efc.save_to_file("output.EFC")?;
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
//! ## Modifying an existing EFC file
//!
//! ```no_run
//! use dvine_types::file::efc::File;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Open an existing file
//! let mut efc = File::open("SOUND.EFC")?;
//!
//! // Extract an effect
//! let sound = efc.extract(10)?.clone();
//!
//! // Modify it (e.g., change priority)
//! let mut modified_sound = sound;
//! modified_sound.sound_header.priority = 200;
//!
//! // Create a new file with modifications
//! let mut new_efc = File::new();
//! new_efc.insert_effect(10, modified_sound)?;
//!
//! // Save the modified file
//! new_efc.save_to_file("MODIFIED.EFC")?;
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

// Module declarations
mod builder;
mod constants;
mod file;
mod iterator;
mod types;

/// Decoder module for IMA ADPCM decompression
pub mod decoder;

/// Encoder module for IMA ADPCM compression
pub mod encoder;

// Re-export public types and constants
pub use self::constants::*;
pub use self::file::File;
pub use self::iterator::{DecodedSoundIter, EffectInfoIter};
pub use self::types::{AdpcmDataHeader, DecodedSound, EffectInfo, SoundDataHeader};

#[cfg(test)]
mod tests;
