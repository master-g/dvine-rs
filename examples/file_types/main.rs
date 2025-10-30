//! Test file types for `dvine-rs`

mod extract;
mod font;
mod startup_cfg;

fn main() {
	// Initialize logger with default level set to info if RUST_LOG is not set
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

	// extract_pft_dsk();
	// check_startup_cfg();
	// font::test_fonts();
	font::test_render_to_bitmap();
}
