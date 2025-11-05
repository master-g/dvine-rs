//! Decoder implementation for EFC files.

use std::io;

/// IMA ADPCM index adjustment table
const IMA_INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// Decode IMA ADPCM data to 16-bit PCM samples
///
/// # Arguments
/// * `adpcm_data` - The compressed ADPCM data
/// * `step_table` - The IMA ADPCM step table (89 entries)
/// * `channels` - Number of audio channels (1 = mono, 2 = stereo)
/// * `sample_count` - Expected number of PCM samples to decode
///
/// # Returns
/// A vector of 16-bit PCM samples
pub fn decode_ima_adpcm(
	adpcm_data: &[u8],
	step_table: &[i16; 89],
	channels: u16,
	sample_count: u32,
) -> io::Result<Vec<i16>> {
	let total_samples = sample_count as usize * channels as usize;
	let mut pcm_data = Vec::with_capacity(total_samples);

	if adpcm_data.len() < 4 {
		return Err(io::Error::new(io::ErrorKind::InvalidData, "ADPCM data too short"));
	}

	// Read initial predictor and step index (first 4 bytes)
	let mut predictor = i16::from_le_bytes([adpcm_data[0], adpcm_data[1]]) as i32;
	let mut step_index = adpcm_data[2] as usize;

	if step_index > 88 {
		step_index = 88;
	}

	// Output first sample
	pcm_data.push(predictor as i16);

	// Decode remaining samples starting from byte 4
	let mut adpcm_pos = 4;

	while adpcm_pos < adpcm_data.len() && pcm_data.len() < total_samples {
		let byte = adpcm_data[adpcm_pos];
		adpcm_pos += 1;

		// Decode two 4-bit samples from one byte
		for nibble in 0..2 {
			if pcm_data.len() >= total_samples {
				break;
			}

			let code = if nibble == 0 {
				byte & 0x0F
			} else {
				(byte >> 4) & 0x0F
			};

			let step = step_table[step_index] as i32;
			let mut diff = step >> 3;

			if code & 1 != 0 {
				diff += step >> 2;
			}
			if code & 2 != 0 {
				diff += step >> 1;
			}
			if code & 4 != 0 {
				diff += step;
			}
			if code & 8 != 0 {
				diff = -diff;
			}

			predictor += diff;

			// Clamp to 16-bit range
			predictor = predictor.clamp(-32768, 32767);

			pcm_data.push(predictor as i16);

			// Update step index
			step_index =
				(step_index as i32 + IMA_INDEX_TABLE[code as usize] as i32).clamp(0, 88) as usize;
		}
	}

	Ok(pcm_data)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_decode_empty() {
		let data = vec![0, 0, 0, 0];
		let step_table = [0i16; 89];
		let result = decode_ima_adpcm(&data, &step_table, 1, 1);
		assert!(result.is_ok());
	}

	#[test]
	fn test_decode_invalid_data() {
		let data = vec![0, 0, 0]; // Too short
		let step_table = [0i16; 89];
		let result = decode_ima_adpcm(&data, &step_table, 1, 1);
		assert!(result.is_err());
	}
}
