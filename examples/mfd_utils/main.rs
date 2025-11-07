//! MFD (Multi-Frame Data) CLI Utility
//!
//! A command-line tool for packing, unpacking, and verifying MFD cursor animation files.
//!
//! # Features
//!
//! - **unpack**: Extract all frames from an MFD file to BMP images with JSON metadata
//! - **pack**: Combine BMP images and JSON metadata into an MFD file
//! - **verify**: Validate MFD encoder/decoder round-trip accuracy
//!
//! # Grayscale Mapping
//!
//! BMP files use grayscale values to represent pixel types:
//! - **White (255)**: Transparent pixel (index 0)
//! - **Gray (128)**: Outline pixel (index 1)
//! - **Black (0)**: Fill pixel (index 255/0xFF)
//!
//! # Metadata Format
//!
//! Frame metadata is stored in a single JSON file with the following structure:
//! ```json
//! {
//!   "frame_count": 23,
//!   "frames": [
//!     {
//!       "index": 0,
//!       "width": 24,
//!       "height": 24,
//!       "x_offset": 0,
//!       "y_offset": 0,
//!       "filename": "frame_0000.bmp"
//!     }
//!   ],
//!   "animation_sequences": [0, 11, 24],
//!   "animation_index_table": [
//!     {
//!       "frame_index": 0,
//!       "duration": 6
//!     },
//!     {
//!       "frame_index": null,
//!       "duration": 0
//!     }
//!   ],
//!   "header": "c033000003000000170000001a000000"
//! }
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Unpack an MFD file to BMPs (auto output: input_frames/)
//! cargo run --example mfd_utils unpack cursor.mfd
//!
//! # Unpack with custom output directory
//! cargo run --example mfd_utils unpack cursor.mfd frames/
//!
//! # Pack BMP files to MFD (auto output: input.mfd)
//! cargo run --example mfd_utils pack frames/
//!
//! # Pack with custom output path
//! cargo run --example mfd_utils pack frames/ cursor.mfd
//!
//! # Verify encoder/decoder correctness
//! cargo run --example mfd_utils verify cursor.mfd
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::mfd::{AnimationEntry, File as MfdFile, FileBuilder, Frame};
use image::{GrayImage, ImageBuffer, Luma};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mfd_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "MFD cursor animation utility - pack, unpack, and verify MFD files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Unpack an MFD file to individual BMP images
	Unpack {
		/// Input MFD file path
		#[arg(value_name = "INPUT_MFD")]
		input: PathBuf,

		/// Output directory path (optional, defaults to `input_frames/`)
		#[arg(value_name = "OUTPUT_DIR")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Pack BMP images from a directory into an MFD file
	Pack {
		/// Input directory containing BMP files
		#[arg(value_name = "INPUT_DIR")]
		input: PathBuf,

		/// Output MFD file path (optional, defaults to `input.mfd`)
		#[arg(value_name = "OUTPUT_MFD")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify MFD encoder/decoder round-trip accuracy
	Verify {
		/// Input MFD file path to verify
		#[arg(value_name = "INPUT_MFD")]
		input: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Save intermediate files for debugging
		#[arg(short, long)]
		save_intermediate: bool,
	},
}

/// Frame metadata for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrameMetadata {
	/// Frame index in the sequence
	index: usize,
	/// Frame width in pixels
	width: u16,
	/// Frame height in pixels
	height: u16,
	/// X offset (hotspot)
	x_offset: i16,
	/// Y offset (hotspot)
	y_offset: i16,
	/// Associated BMP filename
	filename: String,
}

/// Complete MFD metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MfdMetadata {
	/// Total number of frames
	frame_count: usize,
	/// Frame metadata for each frame
	frames: Vec<FrameMetadata>,
	/// Animation sequence start indices (typically 3: normal, busy, special)
	#[serde(skip_serializing_if = "Option::is_none")]
	animation_sequences: Option<Vec<u32>>,
	/// Animation index table entries (`frame_index` + `duration` pairs with loop markers)
	#[serde(skip_serializing_if = "Option::is_none")]
	animation_index_table: Option<Vec<AnimationEntry>>,
	/// File header (16 bytes, hex-encoded)
	#[serde(with = "hex_array")]
	header: [u8; 16],
}

/// Hex serialization module for header array
mod hex_array {
	use serde::{Deserialize, Deserializer, Serializer};

	pub fn serialize<S>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&hex::encode(bytes))
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 16], D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
		if bytes.len() != 16 {
			return Err(serde::de::Error::custom("header must be 16 bytes"));
		}
		let mut array = [0u8; 16];
		array.copy_from_slice(&bytes);
		Ok(array)
	}
}

/// Convert indexed pixel to grayscale value
/// - 0 (transparent) -> 255 (white)
/// - 1 (outline) -> 128 (gray)
/// - 255/0xFF (fill) -> 0 (black)
fn indexed_to_grayscale(pixel: u8) -> u8 {
	match pixel {
		0 => 255, // transparent -> white
		1 => 128, // outline -> gray
		_ => 0,   // fill (0xFF or others) -> black
	}
}

/// Convert grayscale value to indexed pixel
/// - 255 (white) -> 0 (transparent)
/// - 128 (gray) -> 1 (outline)
/// - 0 (black) -> 255/0xFF (fill)
/// - Other values -> closest match
fn grayscale_to_indexed(gray: u8) -> u8 {
	if gray > 192 {
		0 // white-ish -> transparent
	} else if gray > 64 {
		1 // gray-ish -> outline
	} else {
		0xFF // black-ish -> fill
	}
}

/// Save frame as grayscale BMP
fn save_frame_bmp(path: &PathBuf, frame: &Frame) -> Result<(), Box<dyn std::error::Error>> {
	let width = frame.width() as u32;
	let height = frame.height() as u32;

	let mut img: GrayImage = ImageBuffer::new(width, height);

	for (i, &pixel) in frame.pixels().iter().enumerate() {
		let x = (i % width as usize) as u32;
		let y = (i / width as usize) as u32;
		let gray = indexed_to_grayscale(pixel);
		img.put_pixel(x, y, Luma([gray]));
	}

	img.save(path)?;
	Ok(())
}

/// Load frame from grayscale BMP
fn load_frame_bmp(path: &PathBuf) -> Result<(Vec<u8>, u16, u16), Box<dyn std::error::Error>> {
	let img = image::open(path)?.to_luma8();
	let (width, height) = img.dimensions();

	let mut pixels = Vec::with_capacity((width * height) as usize);
	for y in 0..height {
		for x in 0..width {
			let gray = img.get_pixel(x, y).0[0];
			pixels.push(grayscale_to_indexed(gray));
		}
	}

	Ok((pixels, width as u16, height as u16))
}

/// Save metadata to JSON file
fn save_metadata(path: &PathBuf, metadata: &MfdMetadata) -> Result<(), Box<dyn std::error::Error>> {
	let json = serde_json::to_string_pretty(metadata)?;
	fs::write(path, json)?;
	Ok(())
}

/// Load metadata from JSON file
fn load_metadata(path: &PathBuf) -> Result<MfdMetadata, Box<dyn std::error::Error>> {
	let json = fs::read_to_string(path)?;
	let metadata: MfdMetadata = serde_json::from_str(&json)?;
	Ok(metadata)
}

/// Handle unpack command
fn handle_unpack(
	input: PathBuf,
	output: Option<PathBuf>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Generate output directory if not specified
	let output_dir = output.unwrap_or_else(|| {
		let stem = input.file_stem().unwrap_or_default();
		PathBuf::from(format!("{}_frames", stem.to_string_lossy()))
	});

	if verbose {
		println!("🔓 Unpacking MFD file");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output_dir.display());
	}

	// Load MFD file
	if verbose {
		println!("\n📖 Loading MFD file...");
	}
	let mfd = MfdFile::open(&input)?;

	let frame_count = mfd.frame_count();
	if verbose {
		println!("   ✓ Loaded MFD file");
		println!("   - Frame count: {}", frame_count);
		let file_size = mfd.to_bytes()?.len();
		println!("   - File size: {} bytes", file_size);
		if let Some(sequences) = mfd.animation_sequences() {
			println!("   - Animation sequences: {}", sequences.len());
		}
		if let Some(anim_table) = mfd.animation_index_table() {
			println!("   - Animation table entries: {}", anim_table.len());
		}
	}

	// Create output directory
	if verbose {
		println!("\n📁 Creating output directory...");
	}
	fs::create_dir_all(&output_dir)?;
	if verbose {
		println!("   ✓ Created {}", output_dir.display());
	}

	// Extract frames
	if verbose {
		println!("\n📦 Extracting frames...");
	}

	let mut frame_metadata_list = Vec::new();
	let mut success_count = 0;
	let mut total_bytes = 0;

	for (i, frame) in mfd.frames().iter().enumerate() {
		let filename = format!("frame_{:04}.bmp", i);
		let output_path = output_dir.join(&filename);

		// Save as BMP
		save_frame_bmp(&output_path, frame)?;

		// Collect metadata
		frame_metadata_list.push(FrameMetadata {
			index: i,
			width: frame.width(),
			height: frame.height(),
			x_offset: frame.x_offset(),
			y_offset: frame.y_offset(),
			filename: filename.clone(),
		});

		total_bytes += frame.pixel_count();
		success_count += 1;

		if verbose {
			println!(
				"   ✓ Frame {:4}: {}x{} offset=({:3},{:3}) -> {}",
				i,
				frame.width(),
				frame.height(),
				frame.x_offset(),
				frame.y_offset(),
				filename
			);
		}
	}

	// Save metadata as JSON
	let metadata = MfdMetadata {
		frame_count,
		frames: frame_metadata_list,
		animation_sequences: mfd.animation_sequences().map(<[u32]>::to_vec),
		animation_index_table: mfd.animation_index_table().map(<[AnimationEntry]>::to_vec),
		header: *mfd.header(),
	};

	let metadata_path = output_dir.join("metadata.json");
	save_metadata(&metadata_path, &metadata)?;

	if verbose {
		println!("\n💾 Saved metadata: {}", metadata_path.display());
		if let Some(ref sequences) = metadata.animation_sequences {
			println!("   - Animation sequences: {:?}", sequences);
		}
		if let Some(ref anim_table) = metadata.animation_index_table {
			println!("   - Animation table entries: {}", anim_table.len());
			for (i, entry) in anim_table.iter().take(5).enumerate() {
				if let Some(frame_idx) = entry.frame_index {
					println!("     [{}] Frame {} -> duration {}", i, frame_idx, entry.duration);
				} else {
					println!("     [{}] LOOP_MARKER -> duration {}", i, entry.duration);
				}
			}
			if anim_table.len() > 5 {
				println!("     ... and {} more entries", anim_table.len() - 5);
			}
		}
	}

	if verbose {
		println!("\n✅ Unpacking completed successfully!");
		println!("   - Extracted {} frames", success_count);
		println!("   - Total pixels: {} bytes", total_bytes);
		println!("   - Output directory: {}", output_dir.display());
	} else {
		println!(
			"✓ Unpacked {} -> {} ({} frames)",
			input.display(),
			output_dir.display(),
			success_count
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
		let dir_name = input.file_name().unwrap_or_default().to_string_lossy().to_string();
		PathBuf::from(format!("{}.mfd", dir_name))
	});

	if verbose {
		println!("🔒 Packing BMP files into MFD");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
	}

	// Load metadata
	let metadata_path = input.join("metadata.json");
	if !metadata_path.exists() {
		return Err(format!("Metadata file not found: {}", metadata_path.display()).into());
	}

	if verbose {
		println!("\n📖 Loading metadata...");
	}
	let metadata = load_metadata(&metadata_path)?;

	if verbose {
		println!("   ✓ Loaded metadata");
		println!("   - Frame count: {}", metadata.frame_count);
		if let Some(ref sequences) = metadata.animation_sequences {
			println!("   - Animation sequences: {:?}", sequences);
		}
		if let Some(ref anim_table) = metadata.animation_index_table {
			println!("   - Animation table entries: {}", anim_table.len());
		}
	}

	// Load all frames
	if verbose {
		println!("\n📦 Loading frames...");
	}

	let mut builder = FileBuilder::new();

	for frame_meta in &metadata.frames {
		let path = input.join(&frame_meta.filename);

		if !path.exists() {
			return Err(format!("Frame file not found: {}", path.display()).into());
		}

		let (pixels, width, height) = load_frame_bmp(&path)?;

		// Validate dimensions match metadata
		if width != frame_meta.width || height != frame_meta.height {
			eprintln!(
				"⚠ Warning: Frame {} dimensions mismatch: expected {}x{}, got {}x{}",
				frame_meta.index, frame_meta.width, frame_meta.height, width, height
			);
		}

		if verbose {
			println!(
				"   ✓ Frame {:4}: {}x{} offset=({:3},{:3}) from {} ({} pixels)",
				frame_meta.index,
				frame_meta.width,
				frame_meta.height,
				frame_meta.x_offset,
				frame_meta.y_offset,
				frame_meta.filename,
				pixels.len()
			);
		}

		let frame = Frame::new(
			frame_meta.width,
			frame_meta.height,
			frame_meta.x_offset,
			frame_meta.y_offset,
			pixels,
		);
		builder.add_frame(frame)?;
	}

	// Add animation data if present
	if let Some(ref sequences) = metadata.animation_sequences {
		if verbose {
			println!("\n🎬 Adding animation sequences...");
			println!("   - Sequences: {:?}", sequences);
		}
		builder.animation_sequences(sequences.clone());
	}

	if let Some(ref anim_table) = metadata.animation_index_table {
		if verbose {
			println!("   - Animation table entries: {}", anim_table.len());
		}
		builder.animation_index_table(anim_table.clone());
	}

	// Set header
	builder.header(metadata.header);

	// Build MFD file
	if verbose {
		println!("\n🔧 Building MFD file...");
	}

	let mfd = builder.build()?;
	let mfd_bytes = mfd.to_bytes()?;

	if verbose {
		println!("   ✓ Built MFD file ({} bytes)", mfd_bytes.len());
	}

	// Save MFD file
	if verbose {
		println!("\n💾 Saving MFD file...");
	}
	mfd.save(&output)?;

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("\n✅ Packing completed successfully!");
		println!("   - Packed {} frames", metadata.frame_count);
		println!("   - Output size: {} bytes", mfd_bytes.len());
		if metadata.animation_sequences.is_some() {
			println!("   - Included animation sequences");
		}
		if metadata.animation_index_table.is_some() {
			println!("   - Included animation index table");
		}
	} else {
		println!(
			"✓ Packed {} -> {} ({} frames, {} bytes)",
			input.display(),
			output.display(),
			metadata.frame_count,
			mfd_bytes.len()
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
		println!("🔍 Verifying MFD encoder/decoder round-trip");
		println!("   Input: {}", input.display());
	}

	// Step 1: Decode original MFD file
	if verbose {
		println!("\n📖 Step 1: Loading original MFD file...");
	}
	let original_mfd_data = fs::read(&input)?;
	let mfd = MfdFile::open(&input)?;

	let frame_count = mfd.frame_count();
	if verbose {
		println!("   ✓ Loaded {} frames", frame_count);
		if let Some(sequences) = mfd.animation_sequences() {
			println!("   ✓ Animation sequences: {:?}", sequences);
		}
		if let Some(anim_table) = mfd.animation_index_table() {
			println!("   ✓ Loaded {} animation table entries", anim_table.len());
		}
	}

	// Optionally save intermediate BMPs
	if save_intermediate {
		let intermediate_dir = input.with_extension("decoded");
		fs::create_dir_all(&intermediate_dir)?;

		let mut frame_metadata_list = Vec::new();

		for (i, frame) in mfd.frames().iter().enumerate() {
			let filename = format!("frame_{:04}.bmp", i);
			let bmp_path = intermediate_dir.join(&filename);
			save_frame_bmp(&bmp_path, frame)?;

			frame_metadata_list.push(FrameMetadata {
				index: i,
				width: frame.width(),
				height: frame.height(),
				x_offset: frame.x_offset(),
				y_offset: frame.y_offset(),
				filename,
			});
		}

		// Save metadata
		let metadata = MfdMetadata {
			frame_count,
			frames: frame_metadata_list,
			animation_sequences: mfd.animation_sequences().map(<[u32]>::to_vec),
			animation_index_table: mfd.animation_index_table().map(<[AnimationEntry]>::to_vec),
			header: *mfd.header(),
		};

		let metadata_path = intermediate_dir.join("metadata.json");
		save_metadata(&metadata_path, &metadata)?;

		if verbose {
			println!("   ✓ Saved intermediate files: {}", intermediate_dir.display());
		}
	}

	// Step 2: Re-encode to MFD
	if verbose {
		println!("\n🔧 Step 2: Re-encoding to MFD format...");
	}

	let mut builder = FileBuilder::new();
	for frame in mfd.frames() {
		// Clone the frame data
		let new_frame = Frame::new(
			frame.width(),
			frame.height(),
			frame.x_offset(),
			frame.y_offset(),
			frame.pixels().to_vec(),
		);
		builder.add_frame(new_frame)?;
	}

	// Clone animation data if present
	if let Some(sequences) = mfd.animation_sequences() {
		builder.animation_sequences(sequences.to_vec());
	}
	if let Some(anim_table) = mfd.animation_index_table() {
		builder.animation_index_table(anim_table.to_vec());
	}

	let reencoded_mfd = builder.build()?;
	let reencoded_mfd_data = reencoded_mfd.to_bytes()?;

	if verbose {
		println!("   ✓ Re-encoded to {} bytes", reencoded_mfd_data.len());
		println!("   - Original MFD size: {} bytes", original_mfd_data.len());
		println!("   - Re-encoded MFD size: {} bytes", reencoded_mfd_data.len());
	}

	// Optionally save intermediate re-encoded MFD
	if save_intermediate {
		let intermediate_mfd = input.with_extension("reencoded.mfd");
		fs::write(&intermediate_mfd, &reencoded_mfd_data)?;
		if verbose {
			println!("   ✓ Saved intermediate MFD: {}", intermediate_mfd.display());
		}
	}

	// Step 3: Compare frame data
	if verbose {
		println!("\n🔬 Step 3: Comparing frame data...");
	}

	let mut all_match = true;
	let mut total_diffs = 0;

	for (i, (orig_frame, reenc_frame)) in
		mfd.frames().iter().zip(reencoded_mfd.frames().iter()).enumerate()
	{
		// Compare dimensions
		if orig_frame.width() != reenc_frame.width() || orig_frame.height() != reenc_frame.height()
		{
			println!("   ❌ Frame {} dimension mismatch!", i);
			all_match = false;
			continue;
		}

		// Compare offsets
		if orig_frame.x_offset() != reenc_frame.x_offset()
			|| orig_frame.y_offset() != reenc_frame.y_offset()
		{
			println!(
				"   ⚠ Frame {} offset mismatch: ({},{}) vs ({},{})",
				i,
				orig_frame.x_offset(),
				orig_frame.y_offset(),
				reenc_frame.x_offset(),
				reenc_frame.y_offset()
			);
		}

		// Compare pixels
		let orig_pixels = orig_frame.pixels();
		let reenc_pixels = reenc_frame.pixels();

		if orig_pixels != reenc_pixels {
			let mut diff_count = 0;
			for (j, (orig, reenc)) in orig_pixels.iter().zip(reenc_pixels.iter()).enumerate() {
				if orig != reenc {
					diff_count += 1;
					if verbose && diff_count <= 3 {
						println!("   ⚠ Frame {} pixel {}: expected {}, got {}", i, j, orig, reenc);
					}
				}
			}
			println!("   ❌ Frame {} has {} differing pixels", i, diff_count);
			total_diffs += diff_count;
			all_match = false;
		} else if verbose {
			println!("   ✓ Frame {} matches perfectly", i);
		}
	}

	// Step 4: Compare animation data
	if verbose {
		println!("\n🎬 Step 4: Comparing animation data...");
	}

	// Compare animation sequences
	let orig_seq = mfd.animation_sequences();
	let reenc_seq = reencoded_mfd.animation_sequences();

	match (orig_seq, reenc_seq) {
		(Some(orig), Some(reenc)) => {
			if orig == reenc {
				if verbose {
					println!("   ✓ Animation sequences match: {:?}", orig);
				}
			} else {
				println!("   ❌ Animation sequences mismatch!");
				println!("      Original: {:?}", orig);
				println!("      Re-encoded: {:?}", reenc);
				all_match = false;
			}
		}
		(None, None) => {
			if verbose {
				println!("   ✓ No animation sequences (both files)");
			}
		}
		(Some(_orig), None) => {
			println!("   ❌ Original has animation sequences, re-encoded doesn't");
			all_match = false;
		}
		(None, Some(_reenc)) => {
			println!("   ❌ Re-encoded has animation sequences, original doesn't");
			all_match = false;
		}
	}

	// Compare animation index table
	let orig_anim = mfd.animation_index_table();
	let reenc_anim = reencoded_mfd.animation_index_table();

	match (orig_anim, reenc_anim) {
		(Some(orig), Some(reenc)) => {
			if orig == reenc {
				if verbose {
					println!("   ✓ Animation index table matches ({} entries)", orig.len());
				}
			} else {
				println!("   ❌ Animation index table mismatch!");
				println!("      Original: {} entries", orig.len());
				println!("      Re-encoded: {} entries", reenc.len());
				all_match = false;
			}
		}
		(None, None) => {
			if verbose {
				println!("   ✓ No animation index table (both files)");
			}
		}
		(Some(orig), None) => {
			println!(
				"   ❌ Original has animation index table ({} entries), re-encoded doesn't",
				orig.len()
			);
			all_match = false;
		}
		(None, Some(reenc)) => {
			println!(
				"   ❌ Re-encoded has animation index table ({} entries), original doesn't",
				reenc.len()
			);
			all_match = false;
		}
	}

	if all_match {
		println!("\n✅ Verification PASSED: Encoder/decoder are working correctly!");
		println!("   - Frame count: {}", frame_count);
		println!("   - Original size: {} bytes", original_mfd_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_mfd_data.len());

		if original_mfd_data == reencoded_mfd_data {
			println!("   - Files are byte-for-byte identical! 🎉");
		} else {
			let size_diff = reencoded_mfd_data.len() as i64 - original_mfd_data.len() as i64;
			println!(
				"   - Size difference: {} bytes ({:+.2}%)",
				size_diff,
				(size_diff as f64 / original_mfd_data.len() as f64) * 100.0
			);
		}
	} else {
		println!("\n❌ Verification FAILED: Frame data mismatch!");
		println!("   - Total differing pixels: {}", total_diffs);
		return Err("Verification failed".into());
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
	}
}
