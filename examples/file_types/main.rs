//! Test file types for `dvine-rs`

mod item;

fn main() {
	// Initialize logger with default level set to info if RUST_LOG is not set
	env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

	// Parse command line arguments
	let args: Vec<String> = std::env::args().collect();
	if args.len() > 1 {
		match args[1].as_str() {
			"item" => item::test().unwrap(),
			_ => {
				println!("Unknown example: {}", args[1]);
			}
		}
	} else {
		println!(
			"Available examples: efc, efc_builder, font, item, mfd, startup, kg, extract, spr"
		);
		println!("Usage: cargo run --example file-types -- <example_name>");
	}
}
