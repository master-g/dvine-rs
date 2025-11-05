//! Test file types for `dvine-rs`

mod efc;
mod efc_builder;
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
			"efc" => efc::test(),
			"efc_builder" => efc_builder::run().unwrap(),
			"font" => font::test_fonts(),
			"item" => item::test().unwrap(),
			"startup" => startup_cfg::test(),
			"kg" => kg::test(true),
			_ => {
				println!("Unknown example: {}", args[1]);
				println!("Available examples: efc, efc_builder, font, item, startup, kg, extract");
			}
		}
	} else {
		println!("Available examples: efc, efc_builder, font, item, startup, kg, extract");
		println!("Usage: cargo run --example file-types -- <example_name>");
	}
}
