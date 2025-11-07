//! KG Encoder/Decoder Round-trip Test
//!
//! This example demonstrates and validates the KG image encoder by:
//! 1. Loading BMP images from bin/raw_bmp directory
//! 2. Encoding them to KG format
//! 3. Decoding the KG data back to RGB
//! 4. Comparing the original and decoded data for pixel-perfect accuracy
//! 5. Optionally saving the encoded KG files and decoded BMP files
//!
//! This serves as both a comprehensive test and a usage example for the KG encoder.

use dvine_rs::prelude::file::kg::{File as KgFile, compress};
use image::{ImageBuffer, RgbImage};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Statistics for a single test
#[derive(Debug)]
struct TestStats {
	filename: String,
	width: u32,
	height: u32,
	original_size: usize,
	compressed_size: usize,
	compression_ratio: f64,
	pixel_perfect: bool,
	error_message: Option<String>,
}

impl TestStats {
	fn print_summary(&self) {
		println!("\n┌─────────────────────────────────────────────────────────");
		println!("│ File: {}", self.filename);
		println!("├─────────────────────────────────────────────────────────");
		println!("│ Dimensions: {}x{}", self.width, self.height);
		println!("│ Original size: {} bytes", self.original_size);
		println!("│ Compressed size: {} bytes", self.compressed_size);
		println!(
			"│ Compression ratio: {:.2}% ({:.2}x)",
			self.compression_ratio * 100.0,
			1.0 / self.compression_ratio
		);
		println!(
			"│ Pixel-perfect: {}",
			if self.pixel_perfect {
				"✓ YES"
			} else {
				"✗ NO"
			}
		);
		if let Some(err) = &self.error_message {
			println!("│ Error: {}", err);
		}
		println!("└─────────────────────────────────────────────────────────");
	}
}

/// Load a BMP file and convert to RGB data
fn load_bmp(path: &Path) -> Result<(Vec<u8>, u32, u32), Box<dyn std::error::Error>> {
	let img = image::open(path)?;
	let rgb_img = img.to_rgb8();
	let (width, height) = rgb_img.dimensions();
	let pixels = rgb_img.into_raw();

	Ok((pixels, width, height))
}

/// Save RGB data as BMP
fn save_bmp(
	path: &Path,
	rgb_data: &[u8],
	width: u32,
	height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
	let img: RgbImage = ImageBuffer::from_raw(width, height, rgb_data.to_vec())
		.ok_or("Failed to create image buffer")?;
	img.save(path)?;
	Ok(())
}

/// Test a single BMP file through the encode-decode pipeline
fn test_bmp_file(
	bmp_path: &Path,
	output_dir: Option<&Path>,
) -> Result<TestStats, Box<dyn std::error::Error>> {
	let filename = bmp_path.file_name().unwrap().to_string_lossy().to_string();

	println!("\n🔍 Testing: {}", filename);
	println!("   Loading BMP...");

	// Load BMP
	let (original_rgb, width, height) = load_bmp(bmp_path)?;
	let original_size = original_rgb.len();

	println!("   ✓ Loaded {}x{} image ({} bytes)", width, height, original_size);

	// Check color count before encoding
	println!("   Checking unique colors...");
	let mut unique_colors = std::collections::HashSet::new();
	for chunk in original_rgb.chunks(3) {
		unique_colors.insert((chunk[0], chunk[1], chunk[2]));
		if unique_colors.len() > 256 {
			return Ok(TestStats {
				filename,
				width,
				height,
				original_size,
				compressed_size: 0,
				compression_ratio: 0.0,
				pixel_perfect: false,
				error_message: Some(format!(
					"Image has {} unique colors (max 256)",
					unique_colors.len()
				)),
			});
		}
	}
	println!("   ✓ Image has {} unique colors (within 256 limit)", unique_colors.len());

	// Encode to KG
	println!("   Encoding to KG format...");
	let compressed_data = compress(&original_rgb, width as u16, height as u16)?;
	let compressed_size = compressed_data.len();
	let compression_ratio = compressed_size as f64 / original_size as f64;

	println!(
		"   ✓ Compressed to {} bytes ({:.2}% of original)",
		compressed_size,
		compression_ratio * 100.0
	);

	// Optionally save KG file
	if let Some(out_dir) = output_dir {
		let kg_path =
			out_dir.join(format!("{}.kg", bmp_path.file_stem().unwrap().to_string_lossy()));
		fs::write(&kg_path, &compressed_data)?;
		println!("   ✓ Saved KG file: {}", kg_path.display());
	}

	// Decode KG
	println!("   Decoding KG data...");
	let decoded = KgFile::from_reader(&mut compressed_data.as_slice())?;

	// Verify dimensions
	assert_eq!(decoded.header().width() as u32, width, "Width mismatch after decode");
	assert_eq!(decoded.header().height() as u32, height, "Height mismatch after decode");

	let decoded_rgb = decoded.pixels();

	println!(
		"   ✓ Decoded to {}x{} image ({} bytes)",
		decoded.header().width(),
		decoded.header().height(),
		decoded_rgb.len()
	);

	// Compare pixels
	println!("   Comparing pixels...");
	let pixel_perfect = original_rgb == decoded_rgb;

	if !pixel_perfect {
		// Count differing pixels
		let mut diff_count = 0;
		let mut max_diff = 0u32;

		for (i, (orig, dec)) in original_rgb.iter().zip(decoded_rgb.iter()).enumerate() {
			if orig != dec {
				diff_count += 1;
				let diff = (*orig as i32 - *dec as i32).unsigned_abs();
				max_diff = max_diff.max(diff);

				// Print first few differences for debugging
				if diff_count <= 5 {
					let pixel_idx = i / 3;
					let component = ["R", "G", "B"][i % 3];
					println!(
						"   ⚠ Pixel {} {}: expected {}, got {} (diff: {})",
						pixel_idx, component, orig, dec, diff
					);
				}
			}
		}

		println!("   ✗ {} bytes differ (max diff: {})", diff_count, max_diff);
	} else {
		println!("   ✓ Pixel-perfect match!");
	}

	// Optionally save decoded BMP
	if let Some(out_dir) = output_dir {
		let decoded_path = out_dir
			.join(format!("{}_decoded.bmp", bmp_path.file_stem().unwrap().to_string_lossy()));
		save_bmp(&decoded_path, decoded_rgb, width, height)?;
		println!("   ✓ Saved decoded BMP: {}", decoded_path.display());
	}

	Ok(TestStats {
		filename,
		width,
		height,
		original_size,
		compressed_size,
		compression_ratio,
		pixel_perfect,
		error_message: None,
	})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("╔═══════════════════════════════════════════════════════════╗");
	println!("║     KG Encoder/Decoder Round-trip Validation Test        ║");
	println!("╚═══════════════════════════════════════════════════════════╝");

	// Parse command line arguments
	let args: Vec<String> = env::args().collect();
	let mut max_files: Option<usize> = None;
	let mut save_output = true;

	let mut i = 1;
	while i < args.len() {
		match args[i].as_str() {
			"--limit" | "-l" => {
				if i + 1 < args.len() {
					max_files = Some(args[i + 1].parse()?);
					i += 2;
				} else {
					eprintln!("Error: --limit requires a number");
					return Ok(());
				}
			}
			"--no-save" => {
				save_output = false;
				i += 1;
			}
			"--help" | "-h" => {
				println!("\nUsage: cargo run --example kg_encoder_test [OPTIONS]");
				println!("\nOptions:");
				println!("  --limit, -l <N>   Test only the first N files");
				println!("  --no-save         Don't save output files (faster testing)");
				println!("  --help, -h        Show this help message");
				println!("\nExamples:");
				println!("  cargo run --example kg_encoder_test");
				println!("  cargo run --example kg_encoder_test --limit 10");
				println!("  cargo run --example kg_encoder_test --no-save");
				return Ok(());
			}
			_ => {
				eprintln!("Unknown option: {}", args[i]);
				eprintln!("Use --help for usage information");
				return Ok(());
			}
		}
	}

	// Get paths
	let cargo_root = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
	let bin_root = PathBuf::from(&cargo_root).join("bin");
	let bmp_dir = bin_root.join("raw_bmp");
	let output_dir = bin_root.join("kg_test_output");

	// Check if BMP directory exists
	if !bmp_dir.exists() {
		eprintln!("✗ BMP directory not found: {}", bmp_dir.display());
		eprintln!("  Please ensure bin/raw_bmp directory exists with BMP files.");
		return Ok(());
	}

	// Create output directory only if saving
	if save_output && !output_dir.exists() {
		fs::create_dir_all(&output_dir)?;
		println!("\n✓ Created output directory: {}", output_dir.display());
	}

	if !save_output {
		println!("\n⚠ Output saving disabled (--no-save mode)");
	}

	// Find all BMP files
	let mut bmp_files: Vec<PathBuf> = fs::read_dir(&bmp_dir)?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let path = entry.path();
			if path.extension()? == "bmp" {
				Some(path)
			} else {
				None
			}
		})
		.collect();

	bmp_files.sort();

	if bmp_files.is_empty() {
		println!("\n✗ No BMP files found in: {}", bmp_dir.display());
		return Ok(());
	}

	// Limit files if requested
	if let Some(limit) = max_files {
		if limit < bmp_files.len() {
			bmp_files.truncate(limit);
			println!(
				"\n✓ Found {} BMP files, testing first {} (--limit)",
				bmp_files.len() + (limit - bmp_files.len()),
				limit
			);
		} else {
			println!("\n✓ Found {} BMP files to test", bmp_files.len());
		}
	} else {
		println!("\n✓ Found {} BMP files to test", bmp_files.len());
	}

	// Test each file
	let mut results = Vec::new();
	let mut success_count = 0;
	let mut skipped_count = 0;

	for bmp_path in &bmp_files {
		let output = if save_output {
			Some(output_dir.as_path())
		} else {
			None
		};
		match test_bmp_file(bmp_path, output) {
			Ok(stats) => {
				if stats.pixel_perfect && stats.error_message.is_none() {
					success_count += 1;
				} else if stats.error_message.is_some() {
					skipped_count += 1;
				}
				results.push(stats);
			}
			Err(e) => {
				eprintln!("\n✗ Error testing {}: {}", bmp_path.display(), e);
			}
		}
	}

	// Print summary
	println!("\n");
	println!("╔═══════════════════════════════════════════════════════════╗");
	println!("║                      Test Summary                         ║");
	println!("╚═══════════════════════════════════════════════════════════╝");

	for stats in &results {
		stats.print_summary();
	}

	// Print overall statistics
	println!("\n╔═══════════════════════════════════════════════════════════╗");
	println!("║                   Overall Statistics                     ║");
	println!("╚═══════════════════════════════════════════════════════════╝");
	println!("│ Total files tested: {}", results.len());
	println!("│ Pixel-perfect matches: {}", success_count);
	println!("│ Skipped (too many colors): {}", skipped_count);
	println!("│ Failed: {}", results.len() - success_count - skipped_count);

	if !results.is_empty() {
		let avg_ratio: f64 = results
			.iter()
			.filter(|r| r.error_message.is_none())
			.map(|r| r.compression_ratio)
			.sum::<f64>()
			/ (results.len() - skipped_count) as f64;
		println!("│ Average compression ratio: {:.2}%", avg_ratio * 100.0);
	}
	println!("└───────────────────────────────────────────────────────────");

	if success_count == results.len() - skipped_count {
		println!("\n✓ All tests passed! The KG encoder is working correctly.");
	} else {
		println!("\n⚠ Some tests failed. Please review the results above.");
	}

	if save_output {
		println!("\n✓ Output files saved to: {}", output_dir.display());
	}

	Ok(())
}
