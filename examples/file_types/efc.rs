use dvine_rs::prelude::file::EfcFile;

pub(super) fn test() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let efc_path = bin_root.join("Dvine.EFC");
	let output_dir = bin_root.join("efc_extract");

	// Create output directory if it doesn't exist
	if !output_dir.exists() {
		std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");
		println!("Created output directory: {}", output_dir.display());
	}

	// Open the EFC file
	let mut efc = match EfcFile::open(&efc_path) {
		Ok(f) => {
			println!("✓ Successfully opened: {}", efc_path.display());
			f
		}
		Err(e) => {
			eprintln!("✗ Failed to open EFC file: {}", e);
			eprintln!("  Path: {}", efc_path.display());
			eprintln!("  (This is expected if the file doesn't exist)");
			return;
		}
	};

	// Print summary
	let total_effects = efc.effect_count();
	println!("\n=== EFC File Summary ===");
	println!("Total effects: {}", total_effects);
	println!();

	// List all available effects using iterator
	println!("=== Available Effects ===");
	for info in efc.iter() {
		println!("Effect ID {:3}: offset 0x{:08X}", info.id, info.offset);
	}
	println!();

	// Extract and save each effect using iterator
	println!("=== Extracting Effects ===");
	let mut success_count = 0;
	let mut error_count = 0;

	for result in efc.iter_sounds() {
		match result {
			Ok(sound) => {
				// Print effect information
				println!("\nEffect ID {}:", sound.id);
				println!("  Sound type: 0x{:02X}", sound.sound_header.sound_type);
				println!("  Priority: {}", sound.sound_header.priority);
				println!("  Sample rate: {} Hz", sound.adpcm_header.sample_rate);
				println!("  Channels: {}", sound.adpcm_header.channels);
				println!("  Sample count: {}", sound.adpcm_header.sample_count);
				println!("  Duration: {} ms", sound.duration_ms());
				println!("  PCM samples: {}", sound.pcm_data.len());

				// Save as WAV file
				let wav_filename = format!("effect_{:03}.wav", sound.id);
				let wav_path = output_dir.join(wav_filename);

				match std::fs::File::create(&wav_path) {
					Ok(mut wav_file) => match sound.write(&mut wav_file) {
						Ok(_) => {
							println!("  ✓ Saved to: {}", wav_path.display());
							success_count += 1;
						}
						Err(e) => {
							eprintln!("  ✗ Failed to write WAV: {}", e);
							error_count += 1;
						}
					},
					Err(e) => {
						eprintln!("  ✗ Failed to create WAV file: {}", e);
						error_count += 1;
					}
				}
			}
			Err(e) => {
				eprintln!("\n✗ Failed to decode effect: {}", e);
				error_count += 1;
			}
		}
	}

	// Print final summary
	println!("\n=== Extraction Summary ===");
	println!("Total effects: {}", total_effects);
	println!("Successfully extracted: {}", success_count);
	println!("Errors: {}", error_count);
	println!("Output directory: {}", output_dir.display());
}
