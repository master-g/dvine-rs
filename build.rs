//! Build script for copying documentation assets

use std::env;
use std::fs;
use std::path::Path;

fn main() {
	// Tell Cargo to rerun this script if the assets change
	println!("cargo:rerun-if-changed=doc-assets/logo.jpg");
	println!("cargo:rerun-if-changed=doc-assets/3.ico");

	// Copy assets when building documentation
	copy_doc_assets();
}

fn copy_doc_assets() {
	// Get the manifest directory (project root)
	let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
		return;
	};

	// Get the target directory from OUT_DIR
	let Ok(out_dir) = env::var("OUT_DIR") else {
		return;
	};

	// Navigate from OUT_DIR to target directory
	// OUT_DIR is typically: target/{debug|release}/build/{crate-name}-{hash}/out
	let target_dir = Path::new(&out_dir)
		.ancestors()
		.find(|p| p.file_name().and_then(|s| s.to_str()) == Some("target"))
		.unwrap_or_else(|| Path::new("target"));

	// Copy to target/doc root directory (not the crate-specific directory)
	// because rustdoc cleans the crate directory before generating docs
	let doc_root = target_dir.join("doc");

	// Create doc directory if it doesn't exist
	let _ = fs::create_dir_all(&doc_root);

	// Copy logo
	let logo_src = Path::new(&manifest_dir).join("doc-assets/logo.jpg");
	let logo_dst = doc_root.join("logo.jpg");
	if logo_src.exists() {
		let _ = fs::copy(&logo_src, &logo_dst);
	}

	// Copy favicon
	let favicon_src = Path::new(&manifest_dir).join("doc-assets/3.ico");
	let favicon_dst = doc_root.join("3.ico");
	if favicon_src.exists() {
		let _ = fs::copy(&favicon_src, &favicon_dst);
	}
}
