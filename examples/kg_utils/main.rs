//! KG Image Format CLI Utility
//!
//! A command-line tool for encoding, decoding, and verifying KG image files.
//!
//! # Features
//!
//! - **encode**: Convert BMP images to KG format
//! - **decode**: Convert KG images to BMP format
//! - **verify**: Validate KG encoder/decoder round-trip accuracy
//!
//! # Usage
//!
//! ```bash
//! # Encode a BMP file to KG
//! cargo run --example kg_utils encode input.bmp output.kg
//!
//! # Decode a KG file to BMP
//! cargo run --example kg_utils decode input.kg output.bmp
//!
//! # Verify encoder/decoder correctness
//! cargo run --example kg_utils verify input.kg
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::kg::{File as KgFile, compress};
use image::{ImageBuffer, RgbImage};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kg_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "KG image format utility - encode, decode, and verify KG files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Encode a BMP file to KG format
	Encode {
		/// Input BMP file path
		#[arg(value_name = "INPUT_BMP")]
		input: PathBuf,

		/// Output KG file path
		#[arg(value_name = "OUTPUT_KG")]
		output: PathBuf,

		/// Flip image vertically (Y-axis) before encoding
		#[arg(short, long)]
		flip: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Decode a KG file to BMP format
	Decode {
		/// Input KG file path
		#[arg(value_name = "INPUT_KG")]
		input: PathBuf,

		/// Output BMP file path
		#[arg(value_name = "OUTPUT_BMP")]
		output: PathBuf,

		/// Flip image vertically (Y-axis) after decoding
		#[arg(short, long)]
		flip: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify KG encoder/decoder round-trip accuracy
	Verify {
		/// Input KG file path to verify
		#[arg(value_name = "INPUT_KG")]
		input: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Save intermediate files for debugging
		#[arg(short, long)]
		save_intermediate: bool,
	},
}

/// Load a BMP file and convert to RGB data
fn load_bmp(path: &PathBuf, flip: bool) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
	let img = image::open(path)?;
	let mut rgb_img = img.to_rgb8();

	if flip {
		image::imageops::flip_vertical_in_place(&mut rgb_img);
	}

	let (width, height) = rgb_img.dimensions();
	let pixels = rgb_img.into_raw();

	Ok((pixels, width, height))
}

/// Save RGB data as BMP
fn save_bmp(
	path: &PathBuf,
	rgb_data: &[u8],
	width: u32,
	height: u32,
	flip: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data.to_vec())
		.ok_or("Failed to create image buffer")?;

	if flip {
		image::imageops::flip_vertical_in_place(&mut img);
	}

	img.save(path)?;
	Ok(())
}

/// Count unique colors in RGB data
fn count_unique_colors(rgb_data: &[u8]) -> usize {
	let mut colors = std::collections::HashSet::new();
	for chunk in rgb_data.chunks(3) {
		colors.insert((chunk[0], chunk[1], chunk[2]));
		if colors.len() > 256 {
			break;
		}
	}
	colors.len()
}

/// Handle encode command
fn handle_encode(
	input: PathBuf,
	output: PathBuf,
	flip: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔄 Encoding BMP to KG format");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
		if flip {
			println!("   Flip:   Enabled (Y-axis)");
		}
	}

	// Load BMP
	if verbose {
		println!("\n📖 Loading BMP file...");
	}
	let (rgb_data, width, height) = load_bmp(&input, flip)?;

	if verbose {
		println!("   ✓ Loaded {}x{} image ({} bytes)", width, height, rgb_data.len());
	}

	// Check color count
	let color_count = count_unique_colors(&rgb_data);
	if verbose {
		println!("\n🎨 Analyzing colors...");
		println!("   ✓ Found {} unique colors", color_count);
	}

	if color_count > 256 {
		eprintln!("❌ Error: Image has {} unique colors (maximum 256)", color_count);
		eprintln!("   Consider using color quantization to reduce the palette.");
		return Err("Too many colors".into());
	}

	// Encode to KG
	if verbose {
		println!("\n🔧 Encoding to KG format...");
	}
	let compressed_data = compress(&rgb_data, width as u16, height as u16)?;

	let original_size = rgb_data.len();
	let compressed_size = compressed_data.len();
	let ratio = (compressed_size as f64 / original_size as f64) * 100.0;

	if verbose {
		println!("   ✓ Compressed to {} bytes", compressed_size);
		println!("   ✓ Compression ratio: {:.2}% ({:.2}x)", ratio, 1.0 / (ratio / 100.0));
	}

	// Save KG file
	if verbose {
		println!("\n💾 Saving KG file...");
	}
	fs::write(&output, &compressed_data)?;

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("\n✅ Encoding completed successfully!");
	} else {
		println!(
			"✓ Encoded {} -> {} ({:.2}% compression)",
			input.display(),
			output.display(),
			ratio
		);
	}

	Ok(())
}

/// Handle decode command
fn handle_decode(
	input: PathBuf,
	output: PathBuf,
	flip: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔄 Decoding KG to BMP format");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
		if flip {
			println!("   Flip:   Enabled (Y-axis)");
		}
	}

	// Load KG file
	if verbose {
		println!("\n📖 Loading KG file...");
	}
	let kg_file = KgFile::open(&input)?;

	let width = kg_file.header().width() as u32;
	let height = kg_file.header().height() as u32;

	if verbose {
		println!("   ✓ Loaded KG file");
		println!("   - Dimensions: {}x{}", width, height);
		println!("   - Compression type: {}", kg_file.header().compression_type());
		println!("   - File size: {} bytes", kg_file.header().file_size());
	}

	// Get RGB data
	if verbose {
		println!("\n🔓 Decoding image data...");
	}
	let rgb_data = kg_file.pixels();

	if verbose {
		println!("   ✓ Decoded {} bytes", rgb_data.len());
	}

	// Save BMP
	if verbose {
		println!("\n💾 Saving BMP file...");
	}
	save_bmp(&output, rgb_data, width, height, flip)?;

	if verbose {
		println!("   ✓ Saved to {}", output.display());
		println!("\n✅ Decoding completed successfully!");
	} else {
		println!("✓ Decoded {} -> {} ({}x{})", input.display(), output.display(), width, height);
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
		println!("🔍 Verifying KG encoder/decoder round-trip");
		println!("   Input: {}", input.display());
	}

	// Step 1: Decode original KG file
	if verbose {
		println!("\n📖 Step 1: Decoding original KG file...");
	}
	let original_kg_data = fs::read(&input)?;
	let kg_file = KgFile::open(&input)?;

	let width = kg_file.header().width();
	let height = kg_file.header().height();
	let decoded_rgb = kg_file.pixels().to_vec();

	if verbose {
		println!("   ✓ Decoded {}x{} image ({} bytes)", width, height, decoded_rgb.len());
	}

	// Optionally save intermediate BMP
	if save_intermediate {
		let intermediate_bmp = input.with_extension("decoded.bmp");
		save_bmp(&intermediate_bmp, &decoded_rgb, width as u32, height as u32, false)?;
		if verbose {
			println!("   ✓ Saved intermediate BMP: {}", intermediate_bmp.display());
		}
	}

	// Step 2: Re-encode to KG
	if verbose {
		println!("\n🔧 Step 2: Re-encoding to KG format...");
	}
	let reencoded_kg_data = compress(&decoded_rgb, width, height)?;

	if verbose {
		println!("   ✓ Re-encoded to {} bytes", reencoded_kg_data.len());
		println!("   - Original KG size: {} bytes", original_kg_data.len());
		println!("   - Re-encoded KG size: {} bytes", reencoded_kg_data.len());
	}

	// Optionally save intermediate re-encoded KG
	if save_intermediate {
		let intermediate_kg = input.with_extension("reencoded.kg");
		fs::write(&intermediate_kg, &reencoded_kg_data)?;
		if verbose {
			println!("   ✓ Saved intermediate KG: {}", intermediate_kg.display());
		}
	}

	// Step 3: Decode re-encoded KG
	if verbose {
		println!("\n🔓 Step 3: Decoding re-encoded KG...");
	}
	let reencoded_kg_file = KgFile::from_reader(&mut reencoded_kg_data.as_slice())?;
	let redecoded_rgb = reencoded_kg_file.pixels();

	if verbose {
		println!("   ✓ Decoded {} bytes", redecoded_rgb.len());
	}

	// Step 4: Compare pixel data
	if verbose {
		println!("\n🔬 Step 4: Comparing pixel data...");
	}

	let pixel_match = decoded_rgb == redecoded_rgb;

	if pixel_match {
		if verbose {
			println!("   ✓ Pixel-perfect match!");
		}
		println!("\n✅ Verification PASSED: Encoder/decoder are working correctly!");
		println!("   - Dimensions: {}x{}", width, height);
		println!("   - Original size: {} bytes", original_kg_data.len());
		println!("   - Re-encoded size: {} bytes", reencoded_kg_data.len());

		let size_diff = reencoded_kg_data.len() as i64 - original_kg_data.len() as i64;
		if size_diff != 0 {
			println!(
				"   - Size difference: {} bytes ({:+.2}%)",
				size_diff,
				(size_diff as f64 / original_kg_data.len() as f64) * 100.0
			);
			println!("\n   Note: Size difference is normal - different compression patterns");
			println!("   can produce different file sizes while maintaining pixel accuracy.");
		}
	} else {
		// Count differences
		let mut diff_count = 0;
		let mut max_diff = 0u32;
		let mut first_diff_idx = None;

		for (i, (orig, redec)) in decoded_rgb.iter().zip(redecoded_rgb.iter()).enumerate() {
			if orig != redec {
				diff_count += 1;
				let diff = (*orig as i32 - *redec as i32).unsigned_abs();
				max_diff = max_diff.max(diff);
				if first_diff_idx.is_none() {
					first_diff_idx = Some(i);
				}

				// Print first few differences for debugging
				if verbose && diff_count <= 5 {
					let pixel_idx = i / 3;
					let component = ["R", "G", "B"][i % 3];
					println!(
						"   ⚠ Pixel {} {}: expected {}, got {} (diff: {})",
						pixel_idx, component, orig, redec, diff
					);
				}
			}
		}

		println!("\n❌ Verification FAILED: Pixel data mismatch!");
		println!("   - Differing bytes: {} / {}", diff_count, decoded_rgb.len());
		println!("   - Maximum difference: {}", max_diff);
		if let Some(idx) = first_diff_idx {
			println!("   - First difference at byte: {}", idx);
		}

		return Err("Verification failed".into());
	}

	// Step 5: Header comparison
	if verbose {
		println!("\n📋 Step 5: Comparing headers...");
		println!("   Original header:");
		println!("     - Width: {}", kg_file.header().width());
		println!("     - Height: {}", kg_file.header().height());
		println!("     - Compression: {}", kg_file.header().compression_type());

		println!("   Re-encoded header:");
		println!("     - Width: {}", reencoded_kg_file.header().width());
		println!("     - Height: {}", reencoded_kg_file.header().height());
		println!("     - Compression: {}", reencoded_kg_file.header().compression_type());

		if kg_file.header().width() == reencoded_kg_file.header().width()
			&& kg_file.header().height() == reencoded_kg_file.header().height()
		{
			println!("   ✓ Headers match!");
		}
	}

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Encode {
			input,
			output,
			flip,
			verbose,
		} => handle_encode(input, output, flip, verbose),

		Commands::Decode {
			input,
			output,
			flip,
			verbose,
		} => handle_decode(input, output, flip, verbose),

		Commands::Verify {
			input,
			verbose,
			save_intermediate,
		} => handle_verify(input, verbose, save_intermediate),
	}
}
