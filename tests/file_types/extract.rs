use dvine_rs::DskFile;
use log::{error, info};

#[allow(unused)]
pub(super) fn extract_pft_dsk() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	// load pft/dsk files
	let mut cha = DskFile::open(&bin_root, "spr")
		.inspect_err(|e| {
			error!("Cannot open DSK file: {}", e);
		})
		.unwrap();

	// Iterate over all files
	info!("\nIterating over all files:");
	for (idx, result) in cha.iter().enumerate() {
		match result {
			Ok((entry, data)) => {
				info!("  [{}] {}: {} bytes", idx, entry.name(), data.len());
				let output_path = bin_root.join("extract").join(entry.name());
				let parent_dir = output_path.parent().unwrap();
				std::fs::create_dir_all(parent_dir).unwrap();
				std::fs::write(&output_path, &data).unwrap();
			}
			Err(e) => {
				error!("  [{}] Failed to extract: {}", idx, e);
			}
		}
	}

	info!("\nDone!");
}
