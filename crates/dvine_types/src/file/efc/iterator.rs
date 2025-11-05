//! Iterator implementations for EFC files.
//!
//! This module provides iterators for traversing sound effects in EFC files,
//! both with and without decoding.

use std::io::{Read, Seek};

use crate::file::DvFileError;

use super::File;
use super::constants::MAX_EFFECTS;
use super::types::{DecodedSound, EffectInfo, SoundDataHeader};

/// Iterator over effect information
///
/// This iterator provides basic information about each effect without decoding.
pub struct EffectInfoIter<'a> {
	pub(super) index_table: &'a [u32; MAX_EFFECTS],
	pub(super) current_id: usize,
}

impl<'a> Iterator for EffectInfoIter<'a> {
	type Item = EffectInfo;

	fn next(&mut self) -> Option<Self::Item> {
		// Find next valid effect
		while self.current_id < MAX_EFFECTS {
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
/// Returns owned `DecodedSound` instances.
pub struct DecodedSoundIter<'a, R> {
	pub(super) file: &'a mut File<R>,
	pub(super) current_id: usize,
}

impl<'a, R: Read + Seek> Iterator for DecodedSoundIter<'a, R> {
	type Item = Result<DecodedSound, DvFileError>;

	fn next(&mut self) -> Option<Self::Item> {
		// Find next valid effect
		while self.current_id < MAX_EFFECTS {
			let id = self.current_id;
			self.current_id += 1;

			if self.file.index_table[id] != 0 {
				// Extract the effect (now returns owned value)
				return Some(self.file.extract(id));
			}
		}

		None
	}
}
