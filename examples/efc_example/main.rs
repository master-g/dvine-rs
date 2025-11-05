//! EFC Encoder Validation Example
//!
//! This example validates the EFC encoder by:
//! 1. Loading an existing EFC file (Dvine.EFC)
//! 2. Extracting a valid sound effect
//! 3. Re-encoding it and inserting into a different ID slot
//! 4. Creating a new EFC file with both effects
//! 5. Using hexdump to verify the encoded data matches the original

use dvine_rs::prelude::file::{DecodedSound, EfcFile};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
	println!("=== EFC Encoder Validation Test ===\n");

	// Step 1: Load the original Dvine.EFC file
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = Path::new(&cargo_root).join("bin");
	let original_efc_path = bin_root.join("Dvine.EFC");

	println!("Step 1: Loading original EFC file");
	println!("  Path: {}", original_efc_path.display());

	let mut original_efc = EfcFile::open(&original_efc_path)?;
	println!("  ✓ Loaded successfully");
	println!("  Total effects: {}\n", original_efc.effect_count());

	// Step 2: Find and extract a valid effect
	println!("Step 2: Finding a valid effect to extract");
	let source_id = find_valid_effect(&original_efc)?;
	println!("  Found valid effect at ID: {}", source_id);

	let original_sound = original_efc.extract(source_id)?.clone();
	println!("  ✓ Extracted effect {}", source_id);
	println!("    - Sample rate: {} Hz", original_sound.adpcm_header.sample_rate);
	println!("    - Channels: {}", original_sound.adpcm_header.channels);
	println!("    - Sample count: {}", original_sound.adpcm_header.sample_count);
	println!("    - PCM data length: {}", original_sound.pcm_data.len());
	println!("    - Duration: {} ms\n", original_sound.duration_ms());

	// Step 3: Get the original raw data for comparison
	println!("Step 3: Reading original raw effect data from file");
	let original_raw_data = extract_raw_effect_data(&original_efc_path, source_id)?;
	println!("  ✓ Read {} bytes of raw data\n", original_raw_data.len());

	// Step 4: Clone and re-encode the sound to a different ID
	let target_id = find_empty_slot(&original_efc, source_id)?;
	println!("Step 4: Re-encoding effect to target ID: {}", target_id);

	let mut cloned_sound = original_sound.clone();
	cloned_sound.id = target_id;

	// Encode the cloned sound to bytes
	let encoded_bytes = cloned_sound.to_bytes()?;
	println!("  ✓ Encoded to {} bytes\n", encoded_bytes.len());

	// Step 5: Create a new EFC file with both effects
	println!("Step 5: Creating new EFC file with both effects");
	let mut new_efc = EfcFile::new();

	// Insert the original sound (clone it since we moved the original earlier)
	// We need to extract it again from the original file
	let original_for_new = original_efc.extract(source_id)?.clone();
	new_efc.insert_effect(source_id, original_for_new)?;
	println!("  ✓ Inserted original effect at ID {}", source_id);

	// Insert the re-encoded sound
	new_efc.insert_effect(target_id, cloned_sound)?;
	println!("  ✓ Inserted re-encoded effect at ID {}\n", target_id);

	// Step 6: Save the new EFC file
	let output_path = bin_root.join("test_encoded.EFC");
	println!("Step 6: Saving new EFC file");
	println!("  Path: {}", output_path.display());
	new_efc.save_to_file(&output_path)?;
	println!("  ✓ Saved successfully\n");

	// Step 7: Load the new file and extract both effects
	println!("Step 7: Loading new EFC file for verification");
	let mut loaded_efc = EfcFile::open(&output_path)?;
	println!("  ✓ Loaded successfully");
	println!("  Total effects: {}\n", loaded_efc.effect_count());

	// Extract the re-encoded effect's raw data
	println!("Step 8: Reading re-encoded effect data from new file");
	let reencoded_raw_data = extract_raw_effect_data(&output_path, target_id)?;
	println!("  ✓ Read {} bytes of raw data\n", reencoded_raw_data.len());

	// Step 9: Compare the raw data
	println!("Step 9: Comparing original and re-encoded data");
	println!("  Original data size:    {} bytes", original_raw_data.len());
	println!("  Re-encoded data size:  {} bytes", reencoded_raw_data.len());

	// The data might differ slightly in size due to padding, so we compare
	// the meaningful parts: headers and ADPCM data
	let comparison_result =
		compare_effect_data(&original_raw_data, &reencoded_raw_data, &original_sound);

	println!("\n{}", comparison_result);

	// Step 10: Hexdump comparison
	println!("\nStep 10: Hexdump Comparison");
	println!("{}", "=".repeat(80));

	// Show first 256 bytes of each
	let display_bytes = 256.min(original_raw_data.len()).min(reencoded_raw_data.len());

	println!("\n[Original Effect ID {}]", source_id);
	print_hexdump(&original_raw_data[..display_bytes]);

	println!("\n[Re-encoded Effect ID {}]", target_id);
	print_hexdump(&reencoded_raw_data[..display_bytes]);

	// Byte-by-byte comparison of headers
	println!("\n{}", "=".repeat(80));
	println!("Detailed Header Comparison:");
	println!("{}", "=".repeat(80));

	compare_headers(&original_raw_data, &reencoded_raw_data);

	// Step 11: Verify decoded PCM data matches
	println!("\n{}", "=".repeat(80));
	println!("Step 11: Verifying decoded PCM data");
	println!("{}", "=".repeat(80));

	let original_extracted = loaded_efc.extract(source_id)?.clone();
	let reencoded_extracted = loaded_efc.extract(target_id)?.clone();

	println!("Original effect {} - PCM samples: {}", source_id, original_extracted.pcm_data.len());
	println!(
		"Re-encoded effect {} - PCM samples: {}",
		target_id,
		reencoded_extracted.pcm_data.len()
	);

	if original_extracted.pcm_data.len() == reencoded_extracted.pcm_data.len() {
		let mut max_diff = 0i32;
		let mut total_diff = 0i64;
		let mut diff_count = 0;

		for (i, (&orig, &reenc)) in
			original_extracted.pcm_data.iter().zip(&reencoded_extracted.pcm_data).enumerate()
		{
			let diff = (orig - reenc).abs();
			if diff > 0 {
				diff_count += 1;
				total_diff += diff as i64;
				max_diff = max_diff.max(diff as i32);
			}

			// Show first few differences
			if diff_count <= 10 && diff > 0 {
				println!("  Sample {}: orig={}, reenc={}, diff={}", i, orig, reenc, diff);
			}
		}

		if diff_count > 0 {
			let avg_diff = total_diff as f64 / diff_count as f64;
			println!(
				"\n  Differences: {}/{} samples differ",
				diff_count,
				original_extracted.pcm_data.len()
			);
			println!("  Average difference: {:.2}", avg_diff);
			println!("  Maximum difference: {}", max_diff);
		} else {
			println!("\n  ✓ All PCM samples match exactly!");
		}
	} else {
		println!("\n  ✗ PCM data lengths differ!");
	}

	println!("\n{}", "=".repeat(80));
	println!("=== Test Complete ===");
	println!("{}", "=".repeat(80));

	Ok(())
}

/// Find a valid (non-empty) effect in the EFC file
fn find_valid_effect<R: Read + Seek>(efc: &EfcFile<R>) -> Result<usize, Box<dyn Error>> {
	for id in 0..256 {
		if efc.has_effect(id) {
			return Ok(id);
		}
	}
	Err("No valid effects found in EFC file".into())
}

/// Find an empty slot that's different from the source
fn find_empty_slot<R: Read + Seek>(
	efc: &EfcFile<R>,
	avoid_id: usize,
) -> Result<usize, Box<dyn Error>> {
	for id in 0..256 {
		if !efc.has_effect(id) && id != avoid_id {
			return Ok(id);
		}
	}
	Err("No empty slots found".into())
}

/// Extract raw effect data from an EFC file by reading the file directly
fn extract_raw_effect_data(efc_path: &Path, effect_id: usize) -> Result<Vec<u8>, Box<dyn Error>> {
	let mut file = File::open(efc_path)?;

	// Read index table
	let mut index_table = vec![0u8; 256 * 4];
	file.read_exact(&mut index_table)?;

	// Parse the offset for this effect
	let offset_pos = effect_id * 4;
	let offset = u32::from_le_bytes([
		index_table[offset_pos],
		index_table[offset_pos + 1],
		index_table[offset_pos + 2],
		index_table[offset_pos + 3],
	]);

	if offset == 0 {
		return Err(format!("Effect {} has no data (offset = 0)", effect_id).into());
	}

	// Find the next offset to determine size
	let mut next_offset = 0u32;
	for i in (effect_id + 1)..256 {
		let pos = i * 4;
		let off = u32::from_le_bytes([
			index_table[pos],
			index_table[pos + 1],
			index_table[pos + 2],
			index_table[pos + 3],
		]);
		if off != 0 {
			next_offset = off;
			break;
		}
	}

	// If no next offset, read to end of file
	if next_offset == 0 {
		let file_size = std::fs::metadata(efc_path)?.len() as u32;
		next_offset = file_size;
	}

	let size = next_offset - offset;

	// Read the effect data
	let mut data = vec![0u8; size as usize];
	file.seek(std::io::SeekFrom::Start(offset as u64))?;
	file.read_exact(&mut data)?;

	Ok(data)
}

/// Compare effect data and return a detailed report
fn compare_effect_data(original: &[u8], reencoded: &[u8], _sound: &DecodedSound) -> String {
	let mut report = String::new();

	// Compare sound header (first 4 bytes)
	report.push_str("  [Sound Data Header - 4 bytes]\n");
	if original.len() >= 4 && reencoded.len() >= 4 {
		if original[..4] == reencoded[..4] {
			report.push_str("    ✓ Sound header matches\n");
		} else {
			report.push_str("    ✗ Sound header differs\n");
		}
	}

	// Compare ADPCM header (next 0xC0 bytes)
	report.push_str("  [ADPCM Data Header - 0xC0 bytes]\n");
	if original.len() >= 4 + 0xC0 && reencoded.len() >= 4 + 0xC0 {
		if original[4..4 + 0xC0] == reencoded[4..4 + 0xC0] {
			report.push_str("    ✓ ADPCM header matches\n");
		} else {
			report.push_str("    ✗ ADPCM header differs\n");

			// Check specific fields
			let orig_sr = u32::from_le_bytes([original[4], original[5], original[6], original[7]]);
			let reenc_sr =
				u32::from_le_bytes([reencoded[4], reencoded[5], reencoded[6], reencoded[7]]);
			if orig_sr == reenc_sr {
				report.push_str(&format!("      ✓ Sample rate: {}\n", orig_sr));
			} else {
				report.push_str(&format!("      ✗ Sample rate: {} vs {}\n", orig_sr, reenc_sr));
			}

			let orig_ch = u16::from_le_bytes([original[8], original[9]]);
			let reenc_ch = u16::from_le_bytes([reencoded[8], reencoded[9]]);
			if orig_ch == reenc_ch {
				report.push_str(&format!("      ✓ Channels: {}\n", orig_ch));
			} else {
				report.push_str(&format!("      ✗ Channels: {} vs {}\n", orig_ch, reenc_ch));
			}
		}
	}

	// Compare ADPCM data
	report.push_str("  [ADPCM Data]\n");
	let header_size = 4 + 0xC0;
	if original.len() > header_size && reencoded.len() > header_size {
		let orig_adpcm = &original[header_size..];
		let reenc_adpcm = &reencoded[header_size..];

		report.push_str(&format!("    Original ADPCM size: {} bytes\n", orig_adpcm.len()));
		report.push_str(&format!("    Re-encoded ADPCM size: {} bytes\n", reenc_adpcm.len()));

		let compare_len = orig_adpcm.len().min(reenc_adpcm.len());
		let mut diff_count = 0;
		let mut first_diff_pos = None;

		for i in 0..compare_len {
			if orig_adpcm[i] != reenc_adpcm[i] {
				diff_count += 1;
				if first_diff_pos.is_none() {
					first_diff_pos = Some(i);
				}
			}
		}

		if diff_count == 0 && orig_adpcm.len() == reenc_adpcm.len() {
			report.push_str("    ✓ ADPCM data matches exactly!\n");
		} else {
			report.push_str(&format!("    ✗ ADPCM data differs: {} bytes differ\n", diff_count));
			if let Some(pos) = first_diff_pos {
				report.push_str(&format!("      First difference at byte {}\n", pos));
			}
		}
	}

	report
}

/// Print hexdump of data
fn print_hexdump(data: &[u8]) {
	for (i, chunk) in data.chunks(16).enumerate() {
		print!("  {:04X}: ", i * 16);

		// Hex bytes
		for (j, byte) in chunk.iter().enumerate() {
			print!("{:02X} ", byte);
			if j == 7 {
				print!(" ");
			}
		}

		// Padding
		for _ in chunk.len()..16 {
			print!("   ");
			if chunk.len() <= 8 {
				print!(" ");
			}
		}

		// ASCII representation
		print!(" |");
		for byte in chunk {
			if *byte >= 32 && *byte <= 126 {
				print!("{}", *byte as char);
			} else {
				print!(".");
			}
		}
		println!("|");

		// Limit output
		if i >= 15 {
			println!("  ... (truncated, showing first 256 bytes)");
			break;
		}
	}
}

/// Compare headers byte by byte
fn compare_headers(original: &[u8], reencoded: &[u8]) {
	let header_size = (4 + 0xC0).min(original.len()).min(reencoded.len());

	println!("\nSound Data Header (4 bytes):");
	println!("  Offset | Original | Re-encoded | Match");
	println!("  {}", "-".repeat(50));
	for i in 0..4.min(header_size) {
		let match_char = if original[i] == reencoded[i] {
			"✓"
		} else {
			"✗"
		};
		println!(
			"  0x{:04X} |   0x{:02X}   |    0x{:02X}    |  {}",
			i, original[i], reencoded[i], match_char
		);
	}

	println!("\nADPCM Data Header (0xC0 bytes) - Key fields:");
	println!("  Offset | Field          | Original | Re-encoded | Match");
	println!("  {}", "-".repeat(60));

	// Sample rate (offset 4-7)
	if header_size >= 8 {
		let orig_sr = u32::from_le_bytes([original[4], original[5], original[6], original[7]]);
		let reenc_sr = u32::from_le_bytes([reencoded[4], reencoded[5], reencoded[6], reencoded[7]]);
		let match_char = if orig_sr == reenc_sr {
			"✓"
		} else {
			"✗"
		};
		println!("  0x0004 | Sample Rate    | {:8} | {:10} | {}", orig_sr, reenc_sr, match_char);
	}

	// Channels (offset 8-9)
	if header_size >= 10 {
		let orig_ch = u16::from_le_bytes([original[8], original[9]]);
		let reenc_ch = u16::from_le_bytes([reencoded[8], reencoded[9]]);
		let match_char = if orig_ch == reenc_ch {
			"✓"
		} else {
			"✗"
		};
		println!("  0x0008 | Channels       | {:8} | {:10} | {}", orig_ch, reenc_ch, match_char);
	}

	// Sample count (offset 0xBC = 192 + 4 = 196)
	if header_size >= 200 {
		let orig_sc =
			u32::from_le_bytes([original[196], original[197], original[198], original[199]]);
		let reenc_sc =
			u32::from_le_bytes([reencoded[196], reencoded[197], reencoded[198], reencoded[199]]);
		let match_char = if orig_sc == reenc_sc {
			"✓"
		} else {
			"✗"
		};
		println!("  0x00C0 | Sample Count   | {:8} | {:10} | {}", orig_sc, reenc_sc, match_char);
	}

	// Step table comparison
	if header_size >= 4 + 8 + 178 {
		let step_start = 4 + 8;
		let step_end = step_start + 178;
		let steps_match = original[step_start..step_end] == reencoded[step_start..step_end];
		let match_char = if steps_match {
			"✓"
		} else {
			"✗"
		};
		println!(
			"  0x000C | Step Table     |      --- |        --- | {} ({} bytes)",
			match_char, 178
		);
	}
}
