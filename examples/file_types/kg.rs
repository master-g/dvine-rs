use dvine_rs::prelude::file::{KgError, KgFile};
use image::{ImageBuffer, Rgb};

// use dvine_rs::prelude::file::KgHeader;

pub(super) fn test() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let kc_extract_path = bin_root.join("kg_extract");

	// for every file in kc_extract directory
	for entry in std::fs::read_dir(&kc_extract_path).unwrap() {
		let entry = entry.unwrap();
		let path = entry.path();
		if path.is_file() {
			let f = match KgFile::open(&path) {
				Ok(f) => f,
				Err(e) => {
					if matches!(
						e,
						KgError::InvalidMagic {
							expected: _,
							actual: _
						}
					) {
						continue;
					}

					panic!("Failed to open KG file {}: {}", path.display(), e);
				}
			};

			println!("{}", f.header());

			// Get image dimensions and pixel data
			let width = f.header().width() as u32;
			let height = f.header().height() as u32;
			let pixels = f.pixels();

			// Create an RGB image buffer
			let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
				ImageBuffer::from_raw(width, height, pixels.to_vec())
					.expect("Failed to create image buffer");

			// Generate output BMP filename
			let output_filename = path.file_stem().unwrap().to_str().unwrap().to_owned() + ".bmp";
			let output_path = kc_extract_path.join(output_filename);

			// Save as BMP
			img.save(&output_path).unwrap();
			println!("Saved BMP to: {}", output_path.display());
		}
	}
}
