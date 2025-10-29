//! Test file types for `dvine-rs`

use crate::startup_cfg::check_startup_cfg;

mod extract;
mod startup_cfg;

fn main() {
	// Initialize logger with default level set to info if RUST_LOG is not set
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

	// extract_pft_dsk();
	check_startup_cfg();
}
