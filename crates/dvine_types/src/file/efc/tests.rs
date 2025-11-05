//! Unit tests for EFC file operations

use super::*;
use std::io::{Cursor, Read};

fn create_test_efc() -> File<std::io::BufReader<Cursor<Vec<u8>>>> {
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

	let reader = std::io::BufReader::new(Cursor::new(data));
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

#[test]
fn test_new() {
	let builder = FileBuilder::new();
	assert_eq!(builder.effect_count(), 0);
	assert!(!builder.has_effect(0));
	assert!(!builder.has_effect(255));
}

#[test]
fn test_insert_effect() {
	let mut builder = FileBuilder::new();

	let sound = DecodedSound {
		id: 42,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 0,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table: [7; 89],
			sample_count: 10,
		},
		pcm_data: vec![0i16; 10],
	};

	assert!(!builder.has_effect(42));
	builder.insert_effect(42, sound).unwrap();
	assert!(builder.has_effect(42));
	assert_eq!(builder.effect_count(), 1);
}

#[test]
fn test_insert_effect_out_of_range() {
	let mut builder = FileBuilder::new();

	let sound = DecodedSound {
		id: 0,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 0,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table: [7; 89],
			sample_count: 10,
		},
		pcm_data: vec![0i16; 10],
	};

	let result = builder.insert_effect(256, sound);
	assert!(result.is_err());
}

#[test]
fn test_remove_effect() {
	let mut builder = FileBuilder::new();

	let sound = DecodedSound {
		id: 10,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 0,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table: [7; 89],
			sample_count: 10,
		},
		pcm_data: vec![0i16; 10],
	};

	builder.insert_effect(10, sound).unwrap();
	assert!(builder.has_effect(10));
	assert_eq!(builder.effect_count(), 1);

	builder.remove_effect(10);
	assert!(!builder.has_effect(10));
	assert_eq!(builder.effect_count(), 0);
}

#[test]
fn test_to_bytes_empty() {
	let builder = FileBuilder::new();
	let bytes = builder.to_bytes().unwrap();

	// Should only contain the index table (256 * 4 = 1024 bytes)
	assert_eq!(bytes.len(), 256 * 4);

	// All entries should be 0
	for i in 0..256 {
		let offset = u32::from_le_bytes([
			bytes[i * 4],
			bytes[i * 4 + 1],
			bytes[i * 4 + 2],
			bytes[i * 4 + 3],
		]);
		assert_eq!(offset, 0);
	}
}

#[test]
fn test_to_bytes_with_effects() {
	let mut builder = FileBuilder::new();

	// Create a simple step table
	let mut step_table = [0i16; 89];
	for i in 0..89 {
		step_table[i] = 7 + i as i16 * 8;
	}

	let sound = DecodedSound {
		id: 0,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 2,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table,
			sample_count: 10,
		},
		pcm_data: vec![0, 100, 200, 150, 50, -100, -200, -150, -50, 0],
	};

	builder.insert_effect(0, sound).unwrap();

	let bytes = builder.to_bytes().unwrap();

	// Should contain index table + at least one effect
	assert!(bytes.len() > 256 * 4);

	// First entry should point to offset after index table
	let offset = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
	assert_eq!(offset, 256 * 4);
}

#[test]
fn test_decoded_sound_to_bytes() {
	let mut step_table = [0i16; 89];
	for i in 0..89 {
		step_table[i] = 7 + i as i16 * 8;
	}

	let sound = DecodedSound {
		id: 0,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 2,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table,
			sample_count: 10,
		},
		pcm_data: vec![0, 100, 200, 150, 50, -100, -200, -150, -50, 0],
	};

	let bytes = sound.to_bytes().unwrap();

	// Should contain headers + ADPCM data
	// 4 bytes (sound header) + 0xC0 bytes (ADPCM header) + ADPCM data
	assert!(bytes.len() >= 4 + 0xC0);

	// Check sound header
	assert_eq!(bytes[0], 1); // sound_type
	assert_eq!(bytes[1], 2); // unknown_1
	assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 100); // priority
}

#[test]
fn test_encode_decode_roundtrip() {
	let mut step_table = [0i16; 89];
	for i in 0..89 {
		step_table[i] = 7 + i as i16 * 8;
	}

	let original_sound = DecodedSound {
		id: 5,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 2,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table,
			sample_count: 10,
		},
		pcm_data: vec![0, 100, 200, 150, 50, -100, -200, -150, -50, 0],
	};

	// Encode to bytes
	let bytes = original_sound.to_bytes().unwrap();

	// Decode headers from bytes
	let mut cursor = Cursor::new(&bytes);
	let sound_header = SoundDataHeader::from_reader(&mut cursor).unwrap();
	let adpcm_header = AdpcmDataHeader::from_reader(&mut cursor).unwrap();

	// Read ADPCM data
	let mut adpcm_data = Vec::new();
	cursor.read_to_end(&mut adpcm_data).unwrap();

	// Decode ADPCM
	let decoded_pcm = decoder::decode_ima_adpcm(
		&adpcm_data,
		&adpcm_header.step_table,
		adpcm_header.channels,
		adpcm_header.sample_count,
	)
	.unwrap();

	// Check headers match
	assert_eq!(sound_header.sound_type, original_sound.sound_header.sound_type);
	assert_eq!(sound_header.priority, original_sound.sound_header.priority);
	assert_eq!(adpcm_header.sample_rate, original_sound.adpcm_header.sample_rate);
	assert_eq!(adpcm_header.channels, original_sound.adpcm_header.channels);
	assert_eq!(adpcm_header.sample_count, original_sound.adpcm_header.sample_count);

	// Check PCM data length matches
	assert_eq!(decoded_pcm.len(), original_sound.pcm_data.len());

	// First sample should match exactly (it's the predictor)
	assert_eq!(decoded_pcm[0], original_sound.pcm_data[0]);
}

#[test]
fn test_write_and_read_roundtrip() {
	let mut builder = FileBuilder::new();

	let mut step_table = [0i16; 89];
	for i in 0..89 {
		step_table[i] = 7 + i as i16 * 8;
	}

	let sound1 = DecodedSound {
		id: 10,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 0,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table,
			sample_count: 10,
		},
		pcm_data: vec![0, 100, 200, 150, 50, -100, -200, -150, -50, 0],
	};

	let sound2 = DecodedSound {
		id: 20,
		sound_header: SoundDataHeader {
			sound_type: 2,
			unknown_1: 1,
			priority: 200,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 44100,
			channels: 2,
			unknown: 1,
			step_table,
			sample_count: 10,
		},
		pcm_data: vec![1000, -1000, 500, -500, 250, -250, 125, -125, 0, 0],
	};

	builder.insert_effect(10, sound1).unwrap();
	builder.insert_effect(20, sound2).unwrap();

	// Serialize to bytes
	let bytes = builder.to_bytes().unwrap();

	// Read back
	let cursor = std::io::BufReader::new(Cursor::new(bytes));
	let mut efc2 = File::from_reader(cursor).unwrap();

	assert_eq!(efc2.effect_count(), 2);
	assert!(efc2.has_effect(10));
	assert!(efc2.has_effect(20));

	// Extract and verify
	let extracted1 = efc2.extract(10).unwrap();
	assert_eq!(extracted1.id, 10);
	assert_eq!(extracted1.sound_header.sound_type, 1);
	assert_eq!(extracted1.sound_header.priority, 100);
	assert_eq!(extracted1.adpcm_header.sample_rate, 22050);
	assert_eq!(extracted1.adpcm_header.channels, 1);

	let extracted2 = efc2.extract(20).unwrap();
	assert_eq!(extracted2.id, 20);
	assert_eq!(extracted2.sound_header.sound_type, 2);
	assert_eq!(extracted2.sound_header.priority, 200);
	assert_eq!(extracted2.adpcm_header.sample_rate, 44100);
	assert_eq!(extracted2.adpcm_header.channels, 2);
}
