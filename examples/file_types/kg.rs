use std::path::Path;

use dvine_rs::prelude::file::{DvFileError, KgFile};
use image::{ImageBuffer, Rgb};

// use dvine_rs::prelude::file::KgHeader;

pub(super) fn test(path_to_kg_files: impl AsRef<Path>, flip: bool) {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let kg_path = bin_root.join(path_to_kg_files.as_ref());

	// for every file in kc_extract directory
	for entry in std::fs::read_dir(&kg_path).unwrap() {
		let entry = entry.unwrap();
		let path = entry.path();
		if path.is_file() {
			let f = match KgFile::open(&path) {
				Ok(f) => f,
				Err(e) => match e {
					DvFileError::InvalidMagic {
						..
					} => {
						println!("Skipping non-KG file: {}", path.display());
						continue;
					}
					DvFileError::UnderflowError {
						message,
						..
					} => {
						println!(
							"Skipping KG file cannot be decompress correctly:{}\n{}",
							message,
							path.display()
						);
						continue;
					}
					_ => {
						panic!("Failed to open KG file {}: {}", path.display(), e);
					}
				},
			};

			println!("{}", f.header());

			// Generate output BMP filename
			let output_filename = path.file_stem().unwrap().to_str().unwrap().to_owned() + ".bmp";
			let output_path = kg_path.join(output_filename);
			save_to_bmp(&f, &output_path, flip);
		}
	}
}

fn save_to_bmp(f: &KgFile, output_path: &std::path::Path, flip: bool) {
	// Get image dimensions and pixel data
	let width = f.header().width() as u32;
	let height = f.header().height() as u32;
	let pixels = f.pixels();

	// Create an RGB image buffer
	let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, pixels.to_vec())
		.expect("Failed to create image buffer");

	// flip XY axes
	let img = if flip {
		image::imageops::flip_vertical(&img)
		// image::imageops::flip_horizontal(&))
	} else {
		img
	};

	// Save as BMP
	img.save(&output_path).unwrap();
	println!("Saved BMP to: {}", output_path.display());
}
