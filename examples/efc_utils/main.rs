//! EFC (Effect File Collection) CLI Utility
//!
//! A command-line tool for managing, extracting, and playing sound effects from EFC files.
//!
//! # Features
//!
//! - **unpack**: Extract all sound effects from an EFC file to WAV files with JSON metadata
//! - **pack**: Combine WAV files and JSON metadata into an EFC file
//! - **verify**: Validate EFC encoder/decoder round-trip accuracy
//! - **extract**: Extract a specific sound effect to WAV file
//! - **play**: Play a specific sound effect from an EFC file
//!
//! # Metadata Format
//!
//! Sound effect metadata is stored in a JSON file with the following structure:
//! ```json
//! {
//!   "effect_count": 42,
//!   "effects": [
//!     {
//!       "id": 0,
//!       "sound_type": 1,
//!       "unknown_1": 0,
//!       "priority": 100,
//!       "sample_rate": 22050,
//!       "channels": 1,
//!       "unknown": 0,
//!       "sample_count": 8820,
//!       "duration_ms": 400,
//!       "filename": "effect_000.wav"
//!     }
//!   ]
//! }
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Unpack an EFC file to WAV files (auto output: input_effects/)
//! cargo run --example efc_utils unpack Dvine.EFC
//!
//! # Unpack with custom output directory
//! cargo run --example efc_utils unpack Dvine.EFC effects/
//!
//! # Pack WAV files to EFC (auto output: input.EFC)
//! cargo run --example efc_utils pack effects/
//!
//! # Pack with custom output path
//! cargo run --example efc_utils pack effects/ output.EFC
//!
//! # Verify encoder/decoder correctness
//! cargo run --example efc_utils verify Dvine.EFC
//!
//! # Extract a specific effect to WAV
//! cargo run --example efc_utils extract Dvine.EFC 42
//! cargo run --example efc_utils extract Dvine.EFC 42 sound.wav
//!
//! # Play a specific sound effect
//! cargo run --example efc_utils play Dvine.EFC 42
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::{DecodedSound, EfcFile, EfcFileBuilder};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "efc_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "EFC sound effect utility - pack, unpack, verify, extract, and play EFC files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Unpack an EFC file to individual WAV files
	Unpack {
		/// Input EFC file path
		#[arg(value_name = "INPUT_EFC")]
		input: PathBuf,

		/// Output directory path (optional, defaults to `input_effects/`)
		#[arg(value_name = "OUTPUT_DIR")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Pack WAV files from a directory into an EFC file
	Pack {
		/// Input directory containing WAV files
		#[arg(value_name = "INPUT_DIR")]
		input: PathBuf,

		/// Output EFC file path (optional, defaults to `input.EFC`)
		#[arg(value_name = "OUTPUT_EFC")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify EFC encoder/decoder round-trip accuracy
	Verify {
		/// Input EFC file path to verify
		#[arg(value_name = "INPUT_EFC")]
		input: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Save intermediate files for debugging
		#[arg(short, long)]
		save_intermediate: bool,
	},

	/// Extract a specific sound effect to WAV file
	Extract {
		/// Input EFC file path
		#[arg(value_name = "INPUT_EFC")]
		input: PathBuf,

		/// Effect ID to extract (0-255)
		#[arg(value_name = "EFFECT_ID")]
		id: usize,

		/// Output WAV file path (optional, defaults to `effect_<ID>.wav`)
		#[arg(value_name = "OUTPUT_WAV")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Play a specific sound effect
	Play {
		/// Input EFC file path
		#[arg(value_name = "INPUT_EFC")]
		input: PathBuf,

		/// Effect ID to play (0-255)
		#[arg(value_name = "EFFECT_ID")]
		id: usize,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},
}

/// Effect metadata for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EffectMetadata {
	/// Effect ID (0-255)
	id: usize,
	/// Sound type identifier
	sound_type: u8,
	/// Unknown field
	unknown_1: u8,
	/// Priority level
	priority: u16,
	/// Sample rate in Hz
	sample_rate: u32,
	/// Number of channels (1 or 2)
	channels: u16,
	/// Unknown field in ADPCM header
	unknown: u16,
	/// Number of PCM samples
	sample_count: u32,
	/// Duration in milliseconds
	duration_ms: u32,
	/// Associated WAV filename
	filename: String,
}

/// Complete EFC metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EfcMetadata {
	/// Total number of effects
	effect_count: usize,
	/// List of effect metadata
	effects: Vec<EffectMetadata>,
}

/// Save metadata to JSON file
fn save_metadata(path: &PathBuf, metadata: &EfcMetadata) -> Result<(), Box<dyn std::error::Error>> {
	let json = serde_json::to_string_pretty(metadata)?;
	fs::write(path, json)?;
	Ok(())
}

/// Load metadata from JSON file
fn load_metadata(path: &PathBuf) -> Result<EfcMetadata, Box<dyn std::error::Error>> {
	let json = fs::read_to_string(path)?;
	let metadata = serde_json::from_str(&json)?;
	Ok(metadata)
}

/// Save a decoded sound as WAV file
fn save_wav(sound: &DecodedSound, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	let mut file = std::fs::File::create(path)?;
	sound.write(&mut file)?;
	Ok(())
}

/// Load a WAV file and convert to `DecodedSound`
fn load_wav(
	path: &PathBuf,
	metadata: &EffectMetadata,
) -> Result<DecodedSound, Box<dyn std::error::Error>> {
	let mut reader = hound::WavReader::open(path)?;
	let spec = reader.spec();

	// Read PCM samples
	let pcm_data: Vec<i16> = reader.samples::<i16>().collect::<Result<Vec<_>, _>>()?;

	// Create ADPCM header
	let adpcm_header = dvine_rs::prelude::file::AdpcmDataHeader {
		sample_rate: spec.sample_rate,
		channels: spec.channels,
		unknown: metadata.unknown,
		step_table: [7; 89], // Default step table
		sample_count: pcm_data.len() as u32 / spec.channels as u32,
	};

	// Create sound header
	let sound_header = dvine_rs::prelude::file::SoundDataHeader {
		sound_type: metadata.sound_type,
		unknown_1: metadata.unknown_1,
		priority: metadata.priority,
	};

	Ok(DecodedSound {
		id: metadata.id,
		sound_header,
		adpcm_header,
		pcm_data,
	})
}

/// Handle unpack command
fn handle_unpack(
	input: PathBuf,
	output: Option<PathBuf>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Generate output directory if not specified
	let output_dir = output.unwrap_or_else(|| {
		let mut dir = input.clone();
		dir.set_extension("");
		let name = format!("{}_effects", dir.file_name().unwrap().to_string_lossy());
		dir.with_file_name(name)
	});

	if verbose {
		println!("🔓 Unpacking EFC file");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output_dir.display());
	}

	// Create output directory
	fs::create_dir_all(&output_dir)?;

	// Load EFC file
	if verbose {
		println!("\n📖 Loading EFC file...");
	}
	let mut efc = EfcFile::open(&input)?;
	let effect_count = efc.effect_count();

	if verbose {
		println!("   ✓ Loaded successfully");
		println!("   ✓ Total effects: {}", effect_count);
	}

	// Extract all effects
	if verbose {
		println!("\n🔧 Extracting effects...");
	}

	let mut metadata = EfcMetadata {
		effect_count,
		effects: Vec::new(),
	};

	let mut extracted_count = 0;
	for id in 0..256 {
		if !efc.has_effect(id) {
			continue;
		}

		let sound = efc.extract(id)?;
		let filename = format!("effect_{:03}.wav", id);
		let wav_path = output_dir.join(&filename);

		// Save WAV file
		save_wav(&sound, &wav_path)?;

		// Add to metadata
		metadata.effects.push(EffectMetadata {
			id,
			sound_type: sound.sound_header.sound_type,
			unknown_1: sound.sound_header.unknown_1,
			priority: sound.sound_header.priority,
			sample_rate: sound.adpcm_header.sample_rate,
			channels: sound.adpcm_header.channels,
			unknown: sound.adpcm_header.unknown,
			sample_count: sound.adpcm_header.sample_count,
			duration_ms: sound.duration_ms(),
			filename: filename.clone(),
		});

		extracted_count += 1;

		if verbose {
			println!(
				"   ✓ Effect {:3}: {} Hz, {} ch, {:4} ms -> {}",
				id,
				sound.adpcm_header.sample_rate,
				sound.adpcm_header.channels,
				sound.duration_ms(),
				filename
			);
		}
	}

	// Save metadata
	let metadata_path = output_dir.join("metadata.json");
	save_metadata(&metadata_path, &metadata)?;

	if verbose {
		println!("\n💾 Saved metadata to {}", metadata_path.display());
		println!("\n✅ Unpacking completed successfully!");
		println!("   Extracted {} effects", extracted_count);
	} else {
		println!(
			"✓ Unpacked {} -> {} ({} effects)",
			input.display(),
			output_dir.display(),
			extracted_count
		);
	}

	Ok(())
}

/// Handle pack command
fn handle_pack(
	input: PathBuf,
	output: Option<PathBuf>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Generate output path if not specified
	let output = output.unwrap_or_else(|| {
		let name = input.file_name().unwrap().to_string_lossy();
		input.parent().unwrap_or(std::path::Path::new(".")).join(format!("{}.EFC", name))
	});

	if verbose {
		println!("🔧 Packing EFC file");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
	}

	// Load metadata
	let metadata_path = input.join("metadata.json");
	if verbose {
		println!("\n📖 Loading metadata...");
	}
	let metadata = load_metadata(&metadata_path)?;

	if verbose {
		println!("   ✓ Loaded metadata");
		println!("   ✓ Total effects: {}", metadata.effect_count);
	}

	// Create builder
	let mut builder = EfcFileBuilder::new();

	if verbose {
		println!("\n🔧 Loading and encoding effects...");
	}

	for effect_meta in &metadata.effects {
		let wav_path = input.join(&effect_meta.filename);

		// Load WAV file
		let sound = load_wav(&wav_path, effect_meta)?;

		// Insert into builder
		builder.insert_effect(effect_meta.id, sound)?;

		if verbose {
			println!(
				"   ✓ Effect {:3}: {} Hz, {} ch, {:4} ms <- {}",
				effect_meta.id,
				effect_meta.sample_rate,
				effect_meta.channels,
				effect_meta.duration_ms,
				effect_meta.filename
			);
		}
	}

	// Save EFC file
	if verbose {
		println!("\n💾 Saving EFC file...");
	}
	builder.save_to_file(&output)?;

	let file_size = fs::metadata(&output)?.len();

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("   ✓ File size: {} bytes", file_size);
		println!("\n✅ Packing completed successfully!");
	} else {
		println!(
			"✓ Packed {} -> {} ({} effects, {} bytes)",
			input.display(),
			output.display(),
			metadata.effect_count,
			file_size
		);
	}

	Ok(())
}

/// Handle verify command
fn handle_verify(
	input: PathBuf,
	verbose: bool,
	save_intermediate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔍 Verifying EFC encoder/decoder round-trip");
		println!("   Input: {}", input.display());
	}

	// Step 1: Load original EFC file
	if verbose {
		println!("\n📖 Step 1: Loading original EFC file...");
	}
	let original_efc_data = fs::read(&input)?;
	let mut efc = EfcFile::open(&input)?;
	let effect_count = efc.effect_count();

	if verbose {
		println!("   ✓ Loaded successfully");
		println!("   ✓ Total effects: {}", effect_count);
		println!("   ✓ Original file size: {} bytes", original_efc_data.len());
	}

	// Step 2: Extract all effects
	if verbose {
		println!("\n🔓 Step 2: Extracting all effects...");
	}

	let mut extracted_effects = Vec::new();
	for id in 0..256 {
		if efc.has_effect(id) {
			let sound = efc.extract(id)?;

			if verbose {
				println!(
					"   ✓ Effect {:3}: {} samples, {} ms",
					id,
					sound.adpcm_header.sample_count,
					sound.duration_ms()
				);
			}

			extracted_effects.push(sound);
		}
	}

	if verbose {
		println!("   ✓ Extracted {} effects", extracted_effects.len());
	}

	// Optionally save intermediate WAV files
	if save_intermediate {
		let intermediate_dir = input.with_extension("_verify_intermediate");
		fs::create_dir_all(&intermediate_dir)?;

		for sound in &extracted_effects {
			let wav_path = intermediate_dir.join(format!("effect_{:03}.wav", sound.id));
			save_wav(sound, &wav_path)?;
		}

		if verbose {
			println!("   ✓ Saved intermediate WAV files to {}", intermediate_dir.display());
		}
	}

	// Step 3: Re-encode to EFC
	if verbose {
		println!("\n🔧 Step 3: Re-encoding to EFC format...");
	}

	let mut builder = EfcFileBuilder::new();
	for sound in &extracted_effects {
		builder.insert_effect(sound.id, sound.clone())?;
	}

	let reencoded_efc_data = builder.to_bytes()?;

	if verbose {
		println!("   ✓ Re-encoded to {} bytes", reencoded_efc_data.len());
		println!("   - Original size: {} bytes", original_efc_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_efc_data.len());

		let size_diff = reencoded_efc_data.len() as i64 - original_efc_data.len() as i64;
		if size_diff != 0 {
			println!(
				"   - Size difference: {:+} bytes ({:+.2}%)",
				size_diff,
				(size_diff as f64 / original_efc_data.len() as f64) * 100.0
			);
		}
	}

	// Optionally save intermediate re-encoded EFC
	if save_intermediate {
		let intermediate_efc = input.with_extension("reencoded.EFC");
		fs::write(&intermediate_efc, &reencoded_efc_data)?;
		if verbose {
			println!("   ✓ Saved intermediate EFC: {}", intermediate_efc.display());
		}
	}

	// Step 4: Decode re-encoded EFC
	if verbose {
		println!("\n🔓 Step 4: Decoding re-encoded EFC...");
	}

	let mut reencoded_efc = EfcFile::from_reader(Cursor::new(&reencoded_efc_data))?;

	if verbose {
		println!("   ✓ Loaded re-encoded EFC");
		println!("   ✓ Total effects: {}", reencoded_efc.effect_count());
	}

	// Step 5: Compare effects
	if verbose {
		println!("\n🔬 Step 5: Comparing effects...");
	}

	let mut all_match = true;
	let mut total_differences = 0;
	let mut max_difference = 0i32;

	for original_sound in &extracted_effects {
		let id = original_sound.id;
		let reencoded_sound = reencoded_efc.extract(id)?;

		// Compare headers
		let headers_match = original_sound.sound_header == reencoded_sound.sound_header
			&& original_sound.adpcm_header.sample_rate == reencoded_sound.adpcm_header.sample_rate
			&& original_sound.adpcm_header.channels == reencoded_sound.adpcm_header.channels
			&& original_sound.adpcm_header.sample_count
				== reencoded_sound.adpcm_header.sample_count;

		// Compare PCM data
		let pcm_match = original_sound.pcm_data.len() == reencoded_sound.pcm_data.len();

		if !pcm_match {
			if verbose {
				println!(
					"   ✗ Effect {}: PCM length mismatch ({} vs {})",
					id,
					original_sound.pcm_data.len(),
					reencoded_sound.pcm_data.len()
				);
			}
			all_match = false;
			continue;
		}

		// Check for differences in PCM data
		let mut diff_count = 0;
		let mut local_max_diff = 0i32;

		for (&orig, &reenc) in original_sound.pcm_data.iter().zip(&reencoded_sound.pcm_data) {
			let diff = (orig - reenc).abs();
			if diff > 0 {
				diff_count += 1;
				local_max_diff = local_max_diff.max(diff as i32);
				max_difference = max_difference.max(diff as i32);
			}
		}

		total_differences += diff_count;

		if !headers_match || diff_count > 0 {
			all_match = false;

			if verbose {
				if !headers_match {
					println!("   ✗ Effect {}: Headers mismatch", id);
				}
				if diff_count > 0 {
					println!(
						"   ⚠ Effect {}: {} / {} samples differ (max diff: {})",
						id,
						diff_count,
						original_sound.pcm_data.len(),
						local_max_diff
					);
				}
			}
		} else if verbose {
			println!("   ✓ Effect {}: Perfect match", id);
		}
	}

	// Summary
	if all_match {
		println!("\n✅ Verification PASSED: Perfect round-trip!");
		println!("   - All {} effects match exactly", extracted_effects.len());
		println!("   - Original size: {} bytes", original_efc_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_efc_data.len());
	} else {
		println!("\n⚠️  Verification COMPLETED with differences:");
		println!("   - Total effects: {}", extracted_effects.len());

		if total_differences > 0 {
			println!("   - Total sample differences: {}", total_differences);
			println!("   - Maximum difference: {}", max_difference);
			println!("\n   Note: Small differences are normal due to ADPCM compression.");
			println!("   The encoder may produce slightly different results while");
			println!("   maintaining audio quality.");
		}
	}

	Ok(())
}

/// Handle extract command
fn handle_extract(
	input: PathBuf,
	id: usize,
	output: Option<PathBuf>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Generate output path if not specified
	let output = output.unwrap_or_else(|| PathBuf::from(format!("effect_{:03}.wav", id)));

	if verbose {
		println!("🔓 Extracting sound effect");
		println!("   Input:  {}", input.display());
		println!("   Effect: {}", id);
		println!("   Output: {}", output.display());
	}

	// Load EFC file
	if verbose {
		println!("\n📖 Loading EFC file...");
	}
	let mut efc = EfcFile::open(&input)?;

	if !efc.has_effect(id) {
		eprintln!("❌ Error: Effect {} not found in EFC file", id);
		return Err(format!("Effect {} not found", id).into());
	}

	// Extract effect
	if verbose {
		println!("\n🔧 Extracting effect {}...", id);
	}
	let sound = efc.extract(id)?;

	if verbose {
		println!("   ✓ Sample rate: {} Hz", sound.adpcm_header.sample_rate);
		println!("   ✓ Channels: {}", sound.adpcm_header.channels);
		println!("   ✓ Samples: {}", sound.adpcm_header.sample_count);
		println!("   ✓ Duration: {} ms", sound.duration_ms());
	}

	// Save WAV file
	if verbose {
		println!("\n💾 Saving WAV file...");
	}
	save_wav(&sound, &output)?;

	let file_size = fs::metadata(&output)?.len();

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("   ✓ File size: {} bytes", file_size);
		println!("\n✅ Extraction completed successfully!");
	} else {
		println!(
			"✓ Extracted effect {} from {} -> {} ({} ms, {} bytes)",
			id,
			input.display(),
			output.display(),
			sound.duration_ms(),
			file_size
		);
	}

	Ok(())
}

/// Handle play command
fn handle_play(input: PathBuf, id: usize, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔊 Playing sound effect");
		println!("   Input:  {}", input.display());
		println!("   Effect: {}", id);
	}

	// Load EFC file
	if verbose {
		println!("\n📖 Loading EFC file...");
	}
	let mut efc = EfcFile::open(&input)?;

	if !efc.has_effect(id) {
		eprintln!("❌ Error: Effect {} not found in EFC file", id);
		return Err(format!("Effect {} not found", id).into());
	}

	// Extract effect
	if verbose {
		println!("\n🔧 Extracting effect {}...", id);
	}
	let sound = efc.extract(id)?;

	if verbose {
		println!("   ✓ Sample rate: {} Hz", sound.adpcm_header.sample_rate);
		println!("   ✓ Channels: {}", sound.adpcm_header.channels);
		println!("   ✓ Samples: {}", sound.adpcm_header.sample_count);
		println!("   ✓ Duration: {} ms", sound.duration_ms());
	}

	// Convert to WAV in memory
	if verbose {
		println!("\n🎵 Preparing audio...");
	}
	let mut wav_buffer = Vec::new();
	{
		let mut cursor = Cursor::new(&mut wav_buffer);
		sound.write(&mut cursor)?;
	}

	// Play the sound
	if verbose {
		println!("\n▶️  Playing sound effect {}...", id);
	} else {
		println!(
			"▶️  Playing effect {} ({} Hz, {} ch, {} ms)",
			id,
			sound.adpcm_header.sample_rate,
			sound.adpcm_header.channels,
			sound.duration_ms()
		);
	}

	// Initialize audio output
	let (_stream, stream_handle) = OutputStream::try_default()?;
	let sink = Sink::try_new(&stream_handle)?;

	// Decode and play
	let cursor = Cursor::new(wav_buffer);
	let source = Decoder::new(cursor)?;
	sink.append(source);

	// Wait for playback to finish
	sink.sleep_until_end();

	if verbose {
		println!("   ✓ Playback completed");
	}

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Unpack {
			input,
			output,
			verbose,
		} => handle_unpack(input, output, verbose),

		Commands::Pack {
			input,
			output,
			verbose,
		} => handle_pack(input, output, verbose),

		Commands::Verify {
			input,
			verbose,
			save_intermediate,
		} => handle_verify(input, verbose, save_intermediate),

		Commands::Extract {
			input,
			id,
			output,
			verbose,
		} => handle_extract(input, id, output, verbose),

		Commands::Play {
			input,
			id,
			verbose,
		} => handle_play(input, id, verbose),
	}
}
