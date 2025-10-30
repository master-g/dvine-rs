//! font support test

use dvine_rs::FntFile;

#[allow(unused)]
pub(super) fn test_fonts() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	let sys_font_path = bin_root.join("system.fnt");
	let rubi_font_path = bin_root.join("rubi.fnt");

	let sys_font = FntFile::open(&sys_font_path).unwrap();
}
