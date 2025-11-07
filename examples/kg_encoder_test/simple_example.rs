//! Simple KG Encoder API Usage Example
//!
//! This file demonstrates the basic API usage for encoding and decoding KG images.
//! For comprehensive validation testing, see main.rs.

use dvine_rs::prelude::file::kg::{File as KgFile, compress};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== Simple KG Encoder API Example ===\n");

	// Example 1: Create a simple gradient image
	println!("1. Creating a 16x16 gradient image...");
	let width = 16u16;
	let height = 16u16;
	let mut rgb_data = Vec::with_capacity((width as usize) * (height as usize) * 3);

	for y in 0..height {
		for x in 0..width {
			let r = ((x * 255) / width) as u8;
			let g = ((y * 255) / height) as u8;
			let b = 128u8;
			rgb_data.push(r);
			rgb_data.push(g);
			rgb_data.push(b);
		}
	}

	println!("   ✓ Created {}x{} image ({} bytes)", width, height, rgb_data.len());

	// Example 2: Compress to KG format
	println!("\n2. Compressing to KG format...");
	let compressed = compress(&rgb_data, width, height)?;
	let compression_ratio = (compressed.len() as f64) / (rgb_data.len() as f64) * 100.0;

	println!("   ✓ Compressed to {} bytes", compressed.len());
	println!("   ✓ Compression ratio: {:.2}%", compression_ratio);

	// Example 3: Save KG file
	println!("\n3. Saving KG file...");
	std::fs::write("example_gradient.kg", &compressed)?;
	println!("   ✓ Saved to example_gradient.kg");

	// Example 4: Load and decode KG file
	println!("\n4. Loading and decoding KG file...");
	let kg_file = KgFile::open("example_gradient.kg")?;
	println!("   ✓ Loaded KG file");
	println!("   - Width: {}", kg_file.header().width());
	println!("   - Height: {}", kg_file.header().height());
	println!("   - Compression type: {}", kg_file.header().compression_type());

	// Example 5: Verify round-trip accuracy
	println!("\n5. Verifying round-trip accuracy...");
	let decoded_rgb = kg_file.pixels();
	let is_identical = rgb_data == decoded_rgb;

	if is_identical {
		println!("   ✓ Pixel-perfect match! Encoder is working correctly.");
	} else {
		println!("   ✗ Pixels don't match!");
		let diff_count = rgb_data.iter().zip(decoded_rgb.iter()).filter(|(a, b)| a != b).count();
		println!("   - Differing bytes: {}", diff_count);
	}

	// Example 6: Alternative API - Using File methods
	println!("\n6. Using File API for round-trip...");
	let kg_file2 = KgFile::open("example_gradient.kg")?;
	kg_file2.save("example_gradient_copy.kg")?;
	println!("   ✓ Saved copy using File::save()");

	// Cleanup
	println!("\n7. Cleaning up...");
	std::fs::remove_file("example_gradient.kg")?;
	std::fs::remove_file("example_gradient_copy.kg")?;
	println!("   ✓ Removed temporary files");

	println!("\n=== All examples completed successfully! ===");

	Ok(())
}
