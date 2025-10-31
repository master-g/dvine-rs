use std::{collections::HashSet, io::Read};

pub(super) fn test() {
	let cargo_root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let bin_root = std::path::Path::new(&cargo_root).join("bin");
	let kc_extract_path = bin_root.join("kg_extract");

	// buffer
	let mut buf: [u8; 48] = [0; 48];

	let mut fourth_byte_set: HashSet<u8> = std::collections::HashSet::new();

	// for every file in kc_extract directory
	for entry in std::fs::read_dir(&kc_extract_path).unwrap() {
		let entry = entry.unwrap();
		let path = entry.path();
		if path.is_file() {
			// read first 32 bytes
			let mut file = std::fs::File::open(&path).unwrap();
			file.read_exact(&mut buf).unwrap();

			// collect fourth byte
			fourth_byte_set.insert(buf[3]);

			// print buf in hex-dump format
			// 32 bytes per line, 16 bytes per group

			println!("File: {}", path.file_name().unwrap().to_string_lossy());
			print!("      ");
			for i in 0..16 {
				if i % 8 == 0 && i != 0 {
					print!(" ");
				}
				print!("{:02X} ", i);
			}
			for (i, byte) in buf.iter().enumerate() {
				if i % 16 == 0 {
					print!("\n{:04X}: ", i);
				} else if i % 8 == 0 {
					print!(" ");
				}
				print!("{:02X} ", byte);
			}
			println!("\n");
		}
	}

	// print unique fourth bytes
	println!("\nUnique fourth bytes:");
	for byte in &fourth_byte_set {
		print!("{:02X} ", byte);
	}
	println!();
}
