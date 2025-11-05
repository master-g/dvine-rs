//! Example: Creating and manipulating EFC (sound effect) files
//!
//! This example demonstrates how to:
//! - Create a new EFC file from scratch
//! - Insert sound effects with custom parameters
//! - Load PCM data from WAV files (simulated here)
//! - Save the EFC file
//! - Load it back and verify the contents

use dvine_rs::prelude::file::{
	AdpcmDataHeader, DecodedSound, EfcFile, EfcFileBuilder, SoundDataHeader,
};
use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
	println!("=== EFC File Builder Example ===\n");

	// Create a new file builder
	let mut builder = EfcFileBuilder::new();
	println!("Created new file builder");
	println!("Initial effect count: {}\n", builder.effect_count());

	// Create a standard step table for IMA ADPCM
	// This is a typical step table used in IMA ADPCM encoding
	let mut step_table = [0i16; 89];
	(0..89).for_each(|i| {
		step_table[i] = 7 + i as i16 * 8;
	});

	// Example 1: Create a simple beep sound effect
	println!("Creating Effect #10: Simple Beep");
	let beep_sound = create_beep_effect(10, &step_table);
	builder.insert_effect(10, beep_sound)?;
	println!("  - Inserted at ID 10");
	println!("  - Duration: ~100 samples at 22050 Hz");
	println!("  - Priority: 100\n");

	// Example 2: Create a low-frequency rumble
	println!("Creating Effect #25: Low Rumble");
	let rumble_sound = create_rumble_effect(25, &step_table);
	builder.insert_effect(25, rumble_sound)?;
	println!("  - Inserted at ID 25");
	println!("  - Duration: ~200 samples at 22050 Hz");
	println!("  - Priority: 50\n");

	// Example 3: Create a high-priority alert sound
	println!("Creating Effect #100: Alert");
	let alert_sound = create_alert_effect(100, &step_table);
	builder.insert_effect(100, alert_sound)?;
	println!("  - Inserted at ID 100");
	println!("  - Duration: ~150 samples at 44100 Hz");
	println!("  - Priority: 200 (high priority)\n");

	// Display current state
	println!("Current file builder status:");
	println!("  - Total effects: {}", builder.effect_count());
	println!("  - Has effect 10: {}", builder.has_effect(10));
	println!("  - Has effect 25: {}", builder.has_effect(25));
	println!("  - Has effect 100: {}\n", builder.has_effect(100));

	// Save to file
	let output_path = "bin/efc_builder_output.EFC";
	println!("Saving EFC file to: {}", output_path);
	builder.save_to_file(output_path)?;
	println!("  - Saved successfully!\n");

	// Load it back and verify
	println!("Loading EFC file back for verification...");
	let mut loaded_efc = EfcFile::open(output_path)?;
	println!("  - Loaded successfully!");
	println!("  - Effect count: {}", loaded_efc.effect_count());

	// Verify each effect
	println!("\nVerifying loaded effects:");
	for id in [10, 25, 100] {
		if let Ok(sound) = loaded_efc.extract(id) {
			println!("  Effect {}:", id);
			println!("    - Sound type: {}", sound.sound_header.sound_type);
			println!("    - Priority: {}", sound.sound_header.priority);
			println!("    - Sample rate: {} Hz", sound.adpcm_header.sample_rate);
			println!("    - Channels: {}", sound.adpcm_header.channels);
			println!("    - Sample count: {}", sound.adpcm_header.sample_count);
			println!("    - Duration: {} ms", sound.duration_ms());
			println!("    - PCM data length: {}", sound.pcm_data.len());
		}
	}

	// Demonstrate removing an effect
	println!("\nRemoving effect #25...");
	let mut builder_modified = EfcFileBuilder::new();
	builder_modified.insert_effect(10, create_beep_effect(10, &step_table))?;
	builder_modified.insert_effect(25, create_rumble_effect(25, &step_table))?;
	builder_modified.insert_effect(100, create_alert_effect(100, &step_table))?;

	println!("  - Before removal: {} effects", builder_modified.effect_count());
	builder_modified.remove_effect(25);
	println!("  - After removal: {} effects", builder_modified.effect_count());
	println!("  - Has effect 25: {}", builder_modified.has_effect(25));

	// Save modified version
	let modified_path = "bin/efc_builder_modified.EFC";
	println!("\nSaving modified EFC file to: {}", modified_path);
	builder_modified.save_to_file(modified_path)?;
	println!("  - Saved successfully!");

	// Demonstrate iterating over effects
	println!("\nIterating over effects in modified file:");
	let loaded_modified = EfcFile::open(modified_path)?;
	for info in loaded_modified.iter_info() {
		println!("  - Effect {} at offset 0x{:08X}", info.id, info.offset);
	}

	println!("\n=== Example completed successfully! ===");

	Ok(())
}

/// Creates a simple beep sound effect
fn create_beep_effect(id: usize, step_table: &[i16; 89]) -> DecodedSound {
	// Simulate a simple beep: sine-like wave
	let sample_count = 100;
	let mut pcm_data = Vec::with_capacity(sample_count);

	for i in 0..sample_count {
		let t = i as f32 / sample_count as f32;
		let amplitude = 5000.0 * (t * std::f32::consts::PI * 10.0).sin();
		pcm_data.push(amplitude as i16);
	}

	DecodedSound {
		id,
		sound_header: SoundDataHeader {
			sound_type: 1,
			unknown_1: 0,
			priority: 100,
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table: *step_table,
			sample_count: sample_count as u32,
		},
		pcm_data,
	}
}

/// Creates a low-frequency rumble effect
fn create_rumble_effect(id: usize, step_table: &[i16; 89]) -> DecodedSound {
	// Simulate a rumble: low frequency oscillation with decay
	let sample_count = 200;
	let mut pcm_data = Vec::with_capacity(sample_count);

	for i in 0..sample_count {
		let t = i as f32 / sample_count as f32;
		let decay = 1.0 - t; // Linear decay
		let amplitude = 8000.0 * decay * (t * std::f32::consts::PI * 3.0).sin();
		pcm_data.push(amplitude as i16);
	}

	DecodedSound {
		id,
		sound_header: SoundDataHeader {
			sound_type: 2,
			unknown_1: 0,
			priority: 50, // Lower priority
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 22050,
			channels: 1,
			unknown: 0,
			step_table: *step_table,
			sample_count: sample_count as u32,
		},
		pcm_data,
	}
}

/// Creates a high-priority alert sound
fn create_alert_effect(id: usize, step_table: &[i16; 89]) -> DecodedSound {
	// Simulate an alert: alternating high frequency tones
	let sample_count = 150;
	let mut pcm_data = Vec::with_capacity(sample_count);

	for i in 0..sample_count {
		let t = i as f32 / sample_count as f32;
		let freq = if (i / 30) % 2 == 0 {
			20.0
		} else {
			15.0
		};
		let amplitude = 10000.0 * (t * std::f32::consts::PI * freq).sin();
		pcm_data.push(amplitude as i16);
	}

	DecodedSound {
		id,
		sound_header: SoundDataHeader {
			sound_type: 3,
			unknown_1: 1,
			priority: 200, // High priority
		},
		adpcm_header: AdpcmDataHeader {
			sample_rate: 44100, // Higher sample rate for better quality
			channels: 1,
			unknown: 0,
			step_table: *step_table,
			sample_count: sample_count as u32,
		},
		pcm_data,
	}
}
