//! Test file types for `dvine-rs`

mod extract;
mod font;
mod item;
mod kg;
mod startup_cfg;

fn main() {
	// Initialize logger with default level set to info if RUST_LOG is not set
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

	// Parse command line arguments
	let args: Vec<String> = std::env::args().collect();
	if args.len() > 1 {
		match args[1].as_str() {
			"extract" => {
				let filename = if args.len() > 2 {
					&args[2]
				} else {
					"anm"
				};
				extract::extract_pft_dsk(filename);
			}
			"font" => font::test_fonts(),
			"item" => item::test().unwrap(),
			"startup" => startup_cfg::test(),
			"kg" => kg::test(),
			_ => {
				println!("Unknown example: {}", args[1]);
				println!("Available examples: font, item");
			}
		}
	} else {
		println!("Available examples: font, item");
		println!("Usage: cargo run --example file-types -- <example_name>");
	}
}
