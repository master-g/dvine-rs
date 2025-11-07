//! SPR (Sprite) CLI Utility
//!
//! A command-line tool for managing, extracting, and verifying sprite animations from SPR files.
//!
//! # Features
//!
//! - **unpack**: Extract all frames from an SPR file to PNG images with JSON metadata
//! - **pack**: Combine PNG images and JSON metadata into an SPR file
//! - **verify**: Validate SPR encoder/decoder round-trip accuracy
//! - **extract-frame**: Extract a specific frame to PNG files (sprite and mask)
//! - **info**: Display information about an SPR file
//!
//! # Metadata Format
//!
//! Frame metadata is stored in a JSON file with the following structure:
//! ```json
//! {
//!   "frame_count": 23,
//!   "frames": [
//!     {
//!       "index": 0,
//!       "width": 128,
//!       "height": 256,
//!       "hotspot_x": 64,
//!       "hotspot_y": 128,
//!       "sprite_filename": "frame_000_sprite.png",
//!       "mask_filename": "frame_000_mask.png"
//!     }
//!   ]
//! }
//! ```
//!
//! # Palette
//!
//! SPR files require a palette file (SPR.PAL) containing 80 colors.
//! The palette file should be in the same directory as the SPR file or
//! specified explicitly with the `--palette` option.
//!
//! # Usage
//!
//! ```bash
//! # Unpack an SPR file (requires SPR.PAL in same directory)
//! cargo run --example spr_utils -- unpack KATIA.SPR
//!
//! # Unpack with custom palette and output directory
//! cargo run --example spr_utils -- unpack KATIA.SPR -p bin/SPR.PAL -o frames/
//!
//! # Pack PNG files to SPR
//! cargo run --example spr_utils -- pack frames/ output.SPR -p bin/SPR.PAL
//!
//! # Verify encoder/decoder correctness
//! cargo run --example spr_utils -- verify KATIA.SPR -p bin/SPR.PAL
//!
//! # Extract a specific frame
//! cargo run --example spr_utils -- extract-frame KATIA.SPR 5 -p bin/SPR.PAL
//!
//! # Show SPR file information
//! cargo run --example spr_utils -- info KATIA.SPR
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::{SprFile, SprFrame, SprFrameEntry, SprPalette};
use image::{ImageBuffer, Luma, RgbImage, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "spr_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "SPR sprite utility - pack, unpack, verify, and manage SPR files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Unpack an SPR file to individual PNG images
	Unpack {
		/// Input SPR file path
		#[arg(value_name = "INPUT_SPR")]
		input: PathBuf,

		/// Output directory path (optional, defaults to `input_frames/`)
		#[arg(short, long, value_name = "OUTPUT_DIR")]
		output: Option<PathBuf>,

		/// Path to SPR.PAL palette file
		#[arg(short, long, value_name = "PALETTE")]
		palette: Option<PathBuf>,

		/// Export sprite sheet instead of individual frames
		#[arg(short, long)]
		spritesheet: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Pack PNG images from a directory into an SPR file
	Pack {
		/// Input directory containing PNG files
		#[arg(value_name = "INPUT_DIR")]
		input: PathBuf,

		/// Output SPR file path
		#[arg(value_name = "OUTPUT_SPR")]
		output: PathBuf,

		/// Path to SPR.PAL palette file (required for packing)
		#[arg(short, long, value_name = "PALETTE")]
		palette: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify SPR encoder/decoder round-trip accuracy
	Verify {
		/// Input SPR file path to verify
		#[arg(value_name = "INPUT_SPR")]
		input: PathBuf,

		/// Path to SPR.PAL palette file
		#[arg(short, long, value_name = "PALETTE")]
		palette: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Save intermediate files for debugging
		#[arg(short, long)]
		save_intermediate: bool,
	},

	/// Extract a specific frame to PNG files
	ExtractFrame {
		/// Input SPR file path
		#[arg(value_name = "INPUT_SPR")]
		input: PathBuf,

		/// Frame index to extract (0-based)
		#[arg(value_name = "FRAME_INDEX")]
		index: usize,

		/// Output filename prefix (optional, defaults to `frame_<INDEX>`)
		#[arg(short, long, value_name = "OUTPUT")]
		output: Option<String>,

		/// Path to SPR.PAL palette file
		#[arg(short, long, value_name = "PALETTE")]
		palette: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Display information about an SPR file
	Info {
		/// Input SPR file path
		#[arg(value_name = "INPUT_SPR")]
		input: PathBuf,

		/// Show detailed frame information
		#[arg(short, long)]
		detailed: bool,
	},
}

/// Frame metadata for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrameMetadata {
	/// Frame index
	index: usize,
	/// Frame width in pixels
	width: u32,
	/// Frame height in pixels
	height: u32,
	/// Hotspot X coordinate
	hotspot_x: u32,
	/// Hotspot Y coordinate
	hotspot_y: u32,
	/// Sprite PNG filename
	sprite_filename: String,
	/// Mask PNG filename
	mask_filename: String,
}

/// Complete SPR metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SprMetadata {
	/// Total number of frames
	frame_count: usize,
	/// List of frame metadata
	frames: Vec<FrameMetadata>,
}

/// Find palette file in common locations
fn find_palette(
	explicit_path: Option<PathBuf>,
	spr_path: &std::path::Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
	if let Some(path) = explicit_path {
		if path.exists() {
			return Ok(path);
		}
		return Err(format!("Palette file not found: {}", path.display()).into());
	}

	// Try same directory as SPR file
	if let Some(parent) = spr_path.parent() {
		let pal_path = parent.join("SPR.PAL");
		if pal_path.exists() {
			return Ok(pal_path);
		}
	}

	// Try current directory
	let pal_path = PathBuf::from("SPR.PAL");
	if pal_path.exists() {
		return Ok(pal_path);
	}

	Err("Palette file (SPR.PAL) not found. Please specify with --palette option.".into())
}

/// Save metadata to JSON file
fn save_metadata(path: &PathBuf, metadata: &SprMetadata) -> Result<(), Box<dyn std::error::Error>> {
	let json = serde_json::to_string_pretty(metadata)?;
	fs::write(path, json)?;
	Ok(())
}

/// Load metadata from JSON file
fn load_metadata(path: &PathBuf) -> Result<SprMetadata, Box<dyn std::error::Error>> {
	let json = fs::read_to_string(path)?;
	let metadata = serde_json::from_str(&json)?;
	Ok(metadata)
}

/// Save frame sprite as RGB PNG
fn save_sprite_png(
	frame: &SprFrame,
	palette: &SprPalette,
	path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
	let rgb_data = frame.apply_palette_rgb(palette);
	let img: RgbImage = ImageBuffer::from_raw(frame.width(), frame.height(), rgb_data)
		.ok_or("Failed to create sprite image")?;
	img.save(path)?;
	Ok(())
}

/// Save frame mask as grayscale PNG
fn save_mask_png(frame: &SprFrame, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	let mask_data = frame.mask_pixels();
	let img: image::GrayImage =
		ImageBuffer::from_raw(frame.width(), frame.height(), mask_data.to_vec())
			.ok_or("Failed to create mask image")?;
	img.save(path)?;
	Ok(())
}

/// Load sprite PNG and convert to indexed color
fn load_sprite_png(
	path: &PathBuf,
	palette: &SprPalette,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
	let img = image::open(path)?.to_rgb8();
	let pixels = img.as_raw();

	// Convert RGB to palette indices
	let mut indexed = Vec::with_capacity(pixels.len() / 3);
	for chunk in pixels.chunks(3) {
		let r = chunk[0];
		let g = chunk[1];
		let b = chunk[2];

		// Find closest palette color
		let mut best_index = 0u8;
		let mut best_distance = u32::MAX;

		for i in 0..80u8 {
			let (pr, pg, pb, _) = palette.get(i);
			let dr = (r as i32 - pr as i32).unsigned_abs();
			let dg = (g as i32 - pg as i32).unsigned_abs();
			let db = (b as i32 - pb as i32).unsigned_abs();
			let distance = dr * dr + dg * dg + db * db;

			if distance < best_distance {
				best_distance = distance;
				best_index = i;
			}
		}

		// SPR uses indices 176-255 for palette 0-79
		indexed.push(176 + best_index);
	}

	Ok(indexed)
}

/// Load mask PNG
fn load_mask_png(path: &PathBuf) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
	let img = image::open(path)?.to_luma8();
	Ok(img.into_raw())
}

/// Handle unpack command
fn handle_unpack(
	input: PathBuf,
	output: Option<PathBuf>,
	palette_path: Option<PathBuf>,
	spritesheet: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	// Generate output directory if not specified
	let output_dir = output.unwrap_or_else(|| {
		let mut dir = input.clone();
		dir.set_extension("");
		let name = format!("{}_frames", dir.file_name().unwrap().to_string_lossy());
		dir.with_file_name(name)
	});

	if verbose {
		println!("🔓 Unpacking SPR file");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output_dir.display());
	}

	// Load palette
	let palette_path = find_palette(palette_path, &input)?;
	if verbose {
		println!("   Palette: {}", palette_path.display());
	}
	let palette = SprPalette::from_file(&palette_path)?;

	// Create output directory
	fs::create_dir_all(&output_dir)?;

	// Load SPR file
	if verbose {
		println!("\n📖 Loading SPR file...");
	}
	let spr = SprFile::open(&input)?;
	let total_frames = spr.frame_count();

	if verbose {
		println!("   ✓ Loaded successfully");
		println!("   ✓ Total frames: {}", total_frames);
	}

	// Filter valid frames
	let valid_frames: Vec<_> = spr.iter().enumerate().filter(|(_, f)| f.is_valid()).collect();
	let valid_count = valid_frames.len();

	if verbose {
		println!("   ✓ Valid frames: {}", valid_count);
	}

	if valid_count == 0 {
		println!("⚠ No valid frames to export");
		return Ok(());
	}

	if spritesheet {
		// Create sprite sheet
		create_sprite_sheets(&valid_frames, &palette, &output_dir, verbose)?;
	} else {
		// Extract individual frames
		if verbose {
			println!("\n🔧 Extracting frames...");
		}

		let mut metadata = SprMetadata {
			frame_count: valid_count,
			frames: Vec::new(),
		};

		for (index, frame) in &valid_frames {
			let sprite_filename = format!("frame_{:03}_sprite.png", index);
			let mask_filename = format!("frame_{:03}_mask.png", index);

			let sprite_path = output_dir.join(&sprite_filename);
			let mask_path = output_dir.join(&mask_filename);

			save_sprite_png(frame, &palette, &sprite_path)?;
			save_mask_png(frame, &mask_path)?;

			metadata.frames.push(FrameMetadata {
				index: *index,
				width: frame.width(),
				height: frame.height(),
				hotspot_x: frame.hotspot_x(),
				hotspot_y: frame.hotspot_y(),
				sprite_filename,
				mask_filename,
			});

			if verbose {
				println!(
					"   ✓ Frame {:3}: {}x{} (hotspot: {}, {}) -> frame_{:03}_*.png",
					index,
					frame.width(),
					frame.height(),
					frame.hotspot_x(),
					frame.hotspot_y(),
					index
				);
			}
		}

		// Save metadata
		let metadata_path = output_dir.join("metadata.json");
		save_metadata(&metadata_path, &metadata)?;

		if verbose {
			println!("\n💾 Saved metadata to {}", metadata_path.display());
		}
	}

	if verbose {
		println!("\n✅ Unpacking completed successfully!");
		println!("   Extracted {} frames", valid_count);
	} else {
		println!(
			"✓ Unpacked {} -> {} ({} frames)",
			input.display(),
			output_dir.display(),
			valid_count
		);
	}

	Ok(())
}

/// Create sprite sheets
fn create_sprite_sheets(
	valid_frames: &[(usize, SprFrame)],
	palette: &SprPalette,
	output_dir: &std::path::Path,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("\n🎨 Creating sprite sheets...");
	}

	if valid_frames.is_empty() {
		return Ok(());
	}

	// Find maximum dimensions
	let mut max_width = 0u32;
	let mut max_height = 0u32;

	for (_, frame) in valid_frames {
		max_width = max_width.max(frame.width());
		max_height = max_height.max(frame.height());
	}

	// Calculate grid layout
	let frame_count = valid_frames.len();
	let grid_cols = (frame_count as f64).sqrt().ceil() as usize;
	let grid_rows = frame_count.div_ceil(grid_cols);

	if verbose {
		println!(
			"   Grid: {}x{} cells, cell size: {}x{} px",
			grid_cols, grid_rows, max_width, max_height
		);
	}

	let padding = 2u32;
	let cell_width = max_width + padding * 2;
	let cell_height = max_height + padding * 2;

	let sheet_width = grid_cols as u32 * cell_width;
	let sheet_height = grid_rows as u32 * cell_height;

	// Create sprite sheet with checkerboard background
	let mut sprite_sheet: RgbaImage = ImageBuffer::new(sheet_width, sheet_height);
	for y in 0..sheet_height {
		for x in 0..sheet_width {
			let checker = ((x / 8) + (y / 8)) % 2 == 0;
			let gray = if checker {
				200
			} else {
				220
			};
			sprite_sheet.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
		}
	}

	// Create mask sheet
	let mut mask_sheet: image::GrayImage =
		ImageBuffer::from_pixel(sheet_width, sheet_height, Luma([0u8]));

	// Place frames in grid
	for (grid_idx, (_, frame)) in valid_frames.iter().enumerate() {
		let grid_row = grid_idx / grid_cols;
		let grid_col = grid_idx % grid_cols;

		let cell_x = grid_col as u32 * cell_width;
		let cell_y = grid_row as u32 * cell_height;

		let frame_x = cell_x + (cell_width - frame.width()) / 2;
		let frame_y = cell_y + (cell_height - frame.height()) / 2;

		// Get sprite data with mask
		let rgba_data = frame.apply_palette_with_mask(palette);

		// Copy sprite pixels
		for y in 0..frame.height() {
			for x in 0..frame.width() {
				let pixel_idx = ((y * frame.width() + x) * 4) as usize;
				let r = rgba_data[pixel_idx];
				let g = rgba_data[pixel_idx + 1];
				let b = rgba_data[pixel_idx + 2];
				let a = rgba_data[pixel_idx + 3];

				let px = frame_x + x;
				let py = frame_y + y;

				if px < sheet_width && py < sheet_height {
					// Only draw opaque pixels (a == 0 means opaque in SPR)
					if a == 0 {
						sprite_sheet.put_pixel(px, py, Rgba([r, g, b, 255]));
					}
				}
			}
		}

		// Copy mask pixels
		let mask_data = frame.mask_pixels();
		for y in 0..frame.height() {
			for x in 0..frame.width() {
				let pixel_idx = (y * frame.width() + x) as usize;
				let mask_value = mask_data[pixel_idx];

				let px = frame_x + x;
				let py = frame_y + y;

				if px < sheet_width && py < sheet_height {
					mask_sheet.put_pixel(px, py, Luma([mask_value]));
				}
			}
		}
	}

	// Save sheets
	let sprite_sheet_path = output_dir.join("spritesheet.png");
	sprite_sheet.save(&sprite_sheet_path)?;

	let mask_sheet_path = output_dir.join("masksheet.png");
	mask_sheet.save(&mask_sheet_path)?;

	if verbose {
		println!("   ✓ Sprite sheet: {}x{}", sheet_width, sheet_height);
		println!("   ✓ Saved to: spritesheet.png and masksheet.png");
	}

	Ok(())
}

/// Handle pack command
fn handle_pack(
	input: PathBuf,
	output: PathBuf,
	palette_path: PathBuf,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔧 Packing SPR file");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
		println!("   Palette: {}", palette_path.display());
	}

	// Load palette
	let palette = SprPalette::from_file(&palette_path)?;

	// Load metadata
	let metadata_path = input.join("metadata.json");
	if verbose {
		println!("\n📖 Loading metadata...");
	}
	let metadata = load_metadata(&metadata_path)?;

	if verbose {
		println!("   ✓ Loaded metadata");
		println!("   ✓ Total frames: {}", metadata.frame_count);
	}

	// Create SPR file
	let mut spr = SprFile::new();

	if verbose {
		println!("\n🔧 Loading and encoding frames...");
	}

	for frame_meta in &metadata.frames {
		let sprite_path = input.join(&frame_meta.sprite_filename);
		let mask_path = input.join(&frame_meta.mask_filename);

		// Load sprite and mask
		let sprite_pixels = load_sprite_png(&sprite_path, &palette)?;
		let mask_pixels = load_mask_png(&mask_path)?;

		// Create frame entry
		let entry = SprFrameEntry::new(
			0, // color_offset will be set by add_frame
			0, // mask_offset will be set by add_frame
			frame_meta.width,
			frame_meta.height,
			frame_meta.hotspot_x,
			frame_meta.hotspot_y,
		);

		let frame = SprFrame::new(entry, sprite_pixels, mask_pixels);
		spr.add_frame(frame)?;

		if verbose {
			println!(
				"   ✓ Frame {:3}: {}x{} (hotspot: {}, {})",
				frame_meta.index,
				frame_meta.width,
				frame_meta.height,
				frame_meta.hotspot_x,
				frame_meta.hotspot_y
			);
		}
	}

	// Save SPR file
	if verbose {
		println!("\n💾 Saving SPR file...");
	}
	spr.save(&output)?;

	let file_size = fs::metadata(&output)?.len();

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("   ✓ File size: {} bytes", file_size);
		println!("\n✅ Packing completed successfully!");
	} else {
		println!(
			"✓ Packed {} -> {} ({} frames, {} bytes)",
			input.display(),
			output.display(),
			metadata.frame_count,
			file_size
		);
	}

	Ok(())
}

/// Handle verify command
fn handle_verify(
	input: PathBuf,
	palette_path: Option<PathBuf>,
	verbose: bool,
	save_intermediate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔍 Verifying SPR encoder/decoder round-trip");
		println!("   Input: {}", input.display());
	}

	// Load palette
	let palette_path = find_palette(palette_path, &input)?;
	if verbose {
		println!("   Palette: {}", palette_path.display());
	}
	let palette = SprPalette::from_file(&palette_path)?;

	// Step 1: Load original SPR file
	if verbose {
		println!("\n📖 Step 1: Loading original SPR file...");
	}
	let original_spr_data = fs::read(&input)?;
	let spr = SprFile::open(&input)?;
	let frame_count = spr.frame_count();

	if verbose {
		println!("   ✓ Loaded successfully");
		println!("   ✓ Total frames: {}", frame_count);
		println!("   ✓ Original file size: {} bytes", original_spr_data.len());
	}

	// Step 2: Extract all valid frames
	if verbose {
		println!("\n🔓 Step 2: Extracting all frames...");
	}

	let mut extracted_frames = Vec::new();
	for (index, frame) in spr.iter().enumerate() {
		if frame.is_valid() {
			if verbose {
				println!(
					"   ✓ Frame {:3}: {}x{} (hotspot: {}, {})",
					index,
					frame.width(),
					frame.height(),
					frame.hotspot_x(),
					frame.hotspot_y()
				);
			}
			extracted_frames.push((index, frame));
		}
	}

	if verbose {
		println!("   ✓ Extracted {} valid frames", extracted_frames.len());
	}

	// Optionally save intermediate files
	if save_intermediate {
		let intermediate_dir = input.with_extension("_verify_intermediate");
		fs::create_dir_all(&intermediate_dir)?;

		for (index, frame) in &extracted_frames {
			let sprite_path = intermediate_dir.join(format!("frame_{:03}_sprite.png", index));
			let mask_path = intermediate_dir.join(format!("frame_{:03}_mask.png", index));
			save_sprite_png(frame, &palette, &sprite_path)?;
			save_mask_png(frame, &mask_path)?;
		}

		if verbose {
			println!("   ✓ Saved intermediate files to {}", intermediate_dir.display());
		}
	}

	// Step 3: Re-encode to SPR
	if verbose {
		println!("\n🔧 Step 3: Re-encoding to SPR format...");
	}

	let mut new_spr = SprFile::new();
	for (_, frame) in &extracted_frames {
		new_spr.add_frame(frame.clone())?;
	}

	let reencoded_spr_data = new_spr.to_bytes();

	if verbose {
		println!("   ✓ Re-encoded to {} bytes", reencoded_spr_data.len());
		println!("   - Original size: {} bytes", original_spr_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_spr_data.len());

		let size_diff = reencoded_spr_data.len() as i64 - original_spr_data.len() as i64;
		if size_diff != 0 {
			println!(
				"   - Size difference: {:+} bytes ({:+.2}%)",
				size_diff,
				(size_diff as f64 / original_spr_data.len() as f64) * 100.0
			);
		}
	}

	// Optionally save intermediate re-encoded SPR
	if save_intermediate {
		let intermediate_spr = input.with_extension("reencoded.spr");
		fs::write(&intermediate_spr, &reencoded_spr_data)?;
		if verbose {
			println!("   ✓ Saved intermediate SPR: {}", intermediate_spr.display());
		}
	}

	// Step 4: Load re-encoded SPR
	if verbose {
		println!("\n🔓 Step 4: Loading re-encoded SPR...");
	}

	let reencoded_spr = SprFile::from_bytes(&reencoded_spr_data)?;

	if verbose {
		println!("   ✓ Loaded re-encoded SPR");
		println!("   ✓ Total frames: {}", reencoded_spr.frame_count());
	}

	// Step 5: Compare frames
	if verbose {
		println!("\n🔬 Step 5: Comparing frames...");
	}

	let mut all_match = true;
	let mut sprite_diffs = 0;
	let mut mask_diffs = 0;

	for (index, original_frame) in &extracted_frames {
		if let Some(reencoded_frame) = reencoded_spr.get_frame(*index) {
			// Compare dimensions
			let dims_match = original_frame.width() == reencoded_frame.width()
				&& original_frame.height() == reencoded_frame.height()
				&& original_frame.hotspot_x() == reencoded_frame.hotspot_x()
				&& original_frame.hotspot_y() == reencoded_frame.hotspot_y();

			if !dims_match {
				all_match = false;
				if verbose {
					println!("   ✗ Frame {}: Metadata mismatch", index);
				}
				continue;
			}

			// Compare sprite pixels
			let sprite_match = original_frame.sprite_pixels() == reencoded_frame.sprite_pixels();
			if !sprite_match {
				sprite_diffs += 1;
				all_match = false;
			}

			// Compare mask pixels
			let mask_match = original_frame.mask_pixels() == reencoded_frame.mask_pixels();
			if !mask_match {
				mask_diffs += 1;
				all_match = false;
			}

			if verbose && (!sprite_match || !mask_match) {
				println!(
					"   ⚠ Frame {}: sprite={}, mask={}",
					index,
					if sprite_match {
						"✓"
					} else {
						"✗"
					},
					if mask_match {
						"✓"
					} else {
						"✗"
					}
				);
			} else if verbose {
				println!("   ✓ Frame {}: Perfect match", index);
			}
		} else {
			all_match = false;
			if verbose {
				println!("   ✗ Frame {}: Missing in re-encoded file", index);
			}
		}
	}

	// Summary
	if all_match {
		println!("\n✅ Verification PASSED: Perfect round-trip!");
		println!("   - All {} frames match exactly", extracted_frames.len());
		println!("   - Original size: {} bytes", original_spr_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_spr_data.len());
	} else {
		println!("\n⚠️  Verification COMPLETED with differences:");
		println!("   - Total frames: {}", extracted_frames.len());
		if sprite_diffs > 0 {
			println!("   - Sprite differences: {} frames", sprite_diffs);
		}
		if mask_diffs > 0 {
			println!("   - Mask differences: {} frames", mask_diffs);
		}
	}

	Ok(())
}

/// Handle extract-frame command
fn handle_extract_frame(
	input: PathBuf,
	index: usize,
	output: Option<String>,
	palette_path: Option<PathBuf>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let output_prefix = output.unwrap_or_else(|| format!("frame_{:03}", index));

	if verbose {
		println!("🔓 Extracting frame");
		println!("   Input:  {}", input.display());
		println!("   Frame:  {}", index);
		println!("   Output: {}_sprite.png, {}_mask.png", output_prefix, output_prefix);
	}

	// Load palette
	let palette_path = find_palette(palette_path, &input)?;
	if verbose {
		println!("   Palette: {}", palette_path.display());
	}
	let palette = SprPalette::from_file(&palette_path)?;

	// Load SPR file
	if verbose {
		println!("\n📖 Loading SPR file...");
	}
	let spr = SprFile::open(&input)?;

	// Get frame
	let frame =
		spr.get_frame(index).ok_or_else(|| format!("Frame {} not found or invalid", index))?;

	if verbose {
		println!("\n🔧 Extracting frame {}...", index);
		println!("   ✓ Size: {}x{}", frame.width(), frame.height());
		println!("   ✓ Hotspot: ({}, {})", frame.hotspot_x(), frame.hotspot_y());
	}

	// Save files
	let sprite_path = PathBuf::from(format!("{}_sprite.png", output_prefix));
	let mask_path = PathBuf::from(format!("{}_mask.png", output_prefix));

	save_sprite_png(&frame, &palette, &sprite_path)?;
	save_mask_png(&frame, &mask_path)?;

	if verbose {
		println!("\n💾 Saved files:");
		println!("   ✓ {}", sprite_path.display());
		println!("   ✓ {}", mask_path.display());
		println!("\n✅ Extraction completed successfully!");
	} else {
		println!(
			"✓ Extracted frame {} -> {}_sprite.png, {}_mask.png ({}x{})",
			index,
			output_prefix,
			output_prefix,
			frame.width(),
			frame.height()
		);
	}

	Ok(())
}

/// Handle info command
fn handle_info(input: PathBuf, detailed: bool) -> Result<(), Box<dyn std::error::Error>> {
	println!("📄 SPR File Information");
	println!("   File: {}", input.display());

	let spr = SprFile::open(&input)?;
	let file_size = fs::metadata(&input)?.len();

	println!("\n📊 Summary:");
	println!("   Total frames: {}", spr.frame_count());
	println!("   File size: {} bytes ({:.2} KB)", file_size, file_size as f64 / 1024.0);

	// Count valid frames
	let valid_frames: Vec<_> = spr.iter().enumerate().filter(|(_, f)| f.is_valid()).collect();
	println!("   Valid frames: {}", valid_frames.len());
	println!("   Invalid frames: {}", spr.frame_count() as usize - valid_frames.len());

	if detailed && !valid_frames.is_empty() {
		println!("\n📋 Frame Details:");
		println!("   {:<5} {:<10} {:<15} {:<15}", "Index", "Size", "Hotspot", "Data Size");
		println!("   {}", "-".repeat(60));

		for (index, frame) in valid_frames {
			let sprite_size = frame.sprite_pixels().len();
			let mask_size = frame.mask_pixels().len();
			println!(
				"   {:<5} {:<10} ({:>4}, {:>4})    sprite: {} B, mask: {} B",
				index,
				format!("{}x{}", frame.width(), frame.height()),
				frame.hotspot_x(),
				frame.hotspot_y(),
				sprite_size,
				mask_size
			);
		}
	}

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Unpack {
			input,
			output,
			palette,
			spritesheet,
			verbose,
		} => handle_unpack(input, output, palette, spritesheet, verbose),

		Commands::Pack {
			input,
			output,
			palette,
			verbose,
		} => handle_pack(input, output, palette, verbose),

		Commands::Verify {
			input,
			palette,
			verbose,
			save_intermediate,
		} => handle_verify(input, palette, verbose, save_intermediate),

		Commands::ExtractFrame {
			input,
			index,
			output,
			palette,
			verbose,
		} => handle_extract_frame(input, index, output, palette, verbose),

		Commands::Info {
			input,
			detailed,
		} => handle_info(input, detailed),
	}
}
