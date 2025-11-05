//! Encoder implementation for EFC files.

use std::io;

/// IMA ADPCM index adjustment table
const IMA_INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// Encode 16-bit PCM samples to IMA ADPCM data
///
/// # Arguments
/// * `pcm_data` - The 16-bit PCM samples to encode
/// * `step_table` - The IMA ADPCM step table (89 entries)
/// * `channels` - Number of audio channels (1 = mono, 2 = stereo)
///
/// # Returns
/// A vector of compressed ADPCM data
pub fn encode_ima_adpcm(
	pcm_data: &[i16],
	step_table: &[i16; 89],
	_channels: u16,
) -> io::Result<Vec<u8>> {
	if pcm_data.is_empty() {
		return Err(io::Error::new(io::ErrorKind::InvalidInput, "PCM data is empty"));
	}

	// Calculate output size: 4 bytes header + ceil(samples / 2) bytes
	let sample_count = pcm_data.len();
	let adpcm_size = 4 + sample_count.div_ceil(2);
	let mut adpcm_data = Vec::with_capacity(adpcm_size);

	// Initialize predictor and step index
	let mut predictor = pcm_data[0] as i32;
	let mut step_index = 0usize;

	// Write initial predictor and step index (first 4 bytes)
	adpcm_data.extend_from_slice(&predictor.to_le_bytes()[0..2]);
	adpcm_data.push(step_index as u8);
	adpcm_data.push(0); // Reserved byte

	// Encode samples starting from index 1 (skip first sample which is the predictor)
	let mut pcm_pos = 1;
	while pcm_pos < sample_count {
		let mut byte = 0u8;

		// Encode two 4-bit samples into one byte
		for nibble in 0..2 {
			if pcm_pos >= sample_count {
				break;
			}

			let sample = pcm_data[pcm_pos] as i32;
			let diff = sample - predictor;
			let step = step_table[step_index] as i32;

			// Encode the difference
			let mut code = 0u8;
			let mut encoded_diff = step >> 3;

			if diff < 0 {
				code = 8;
			}

			let abs_diff = diff.abs();

			if abs_diff >= step {
				code |= 4;
				encoded_diff += step;
			}
			if abs_diff >= step >> 1 {
				code |= 2;
				encoded_diff += step >> 1;
			}
			if abs_diff >= step >> 2 {
				code |= 1;
				encoded_diff += step >> 2;
			}

			// Apply sign
			if diff < 0 {
				encoded_diff = -encoded_diff;
			}

			// Update predictor
			predictor += encoded_diff;
			predictor = predictor.clamp(-32768, 32767);

			// Update step index
			step_index =
				(step_index as i32 + IMA_INDEX_TABLE[code as usize] as i32).clamp(0, 88) as usize;

			// Pack nibble into byte
			if nibble == 0 {
				byte = code & 0x0F;
			} else {
				byte |= (code & 0x0F) << 4;
			}

			pcm_pos += 1;
		}

		adpcm_data.push(byte);
	}

	Ok(adpcm_data)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::file::efc::decoder::decode_ima_adpcm;

	#[test]
	fn test_encode_empty() {
		let pcm_data: Vec<i16> = vec![];
		let step_table = [0i16; 89];
		let result = encode_ima_adpcm(&pcm_data, &step_table, 1);
		assert!(result.is_err());
	}

	#[test]
	fn test_encode_single_sample() {
		let pcm_data = vec![100i16];
		let step_table = [7i16; 89];
		let result = encode_ima_adpcm(&pcm_data, &step_table, 1);
		assert!(result.is_ok());
		let adpcm = result.unwrap();
		assert_eq!(adpcm.len(), 4); // Header only
	}

	#[test]
	fn test_encode_decode_roundtrip() {
		// Create a simple step table
		let mut step_table = [0i16; 89];
		(0..89).for_each(|i| {
			step_table[i] = 7 + i as i16 * 8;
		});

		// Create test PCM data
		let pcm_data: Vec<i16> = vec![0, 100, 200, 150, 50, -100, -200, -150, -50, 0];

		// Encode
		let encoded = encode_ima_adpcm(&pcm_data, &step_table, 1).unwrap();
		assert!(encoded.len() >= 4);

		// Decode
		let decoded = decode_ima_adpcm(&encoded, &step_table, 1, pcm_data.len() as u32).unwrap();

		// Check that we got the right number of samples
		assert_eq!(decoded.len(), pcm_data.len());

		// The first sample should match exactly (it's the predictor)
		assert_eq!(decoded[0], pcm_data[0]);

		// Other samples won't match exactly due to lossy compression,
		// but they should be reasonably close
		for i in 1..pcm_data.len() {
			let diff = (decoded[i] - pcm_data[i]).abs();
			// Allow some error due to lossy compression
			assert!(
				diff < 500,
				"Sample {} difference too large: expected {}, got {}, diff {}",
				i,
				pcm_data[i],
				decoded[i],
				diff
			);
		}
	}

	#[test]
	fn test_encode_extremes() {
		let mut step_table = [0i16; 89];
		(0..89).for_each(|i| {
			step_table[i] = 7 + i as i16 * 8;
		});

		let pcm_data = vec![i16::MIN, i16::MAX, 0, i16::MIN / 2, i16::MAX / 2];
		let result = encode_ima_adpcm(&pcm_data, &step_table, 1);
		assert!(result.is_ok());

		let encoded = result.unwrap();
		let decoded = decode_ima_adpcm(&encoded, &step_table, 1, pcm_data.len() as u32).unwrap();
		assert_eq!(decoded.len(), pcm_data.len());
	}
}
