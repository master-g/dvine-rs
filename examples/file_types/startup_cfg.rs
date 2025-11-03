use dvine_rs::prelude::file::startup_ini::{RenderMode, StartupIni};

#[allow(unused)]
pub(super) fn test() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");

	// Load startup.cfg file
	let startup_cfg_path = bin_root.join("startup.ini");
	let mut startup_cfg = StartupIni::open(&startup_cfg_path).unwrap();

	startup_cfg.set_render_mode(RenderMode::VsyncOff);

	// Print loaded configuration
	println!("Loaded startup.cfg:");
	println!("{}", startup_cfg);
}
