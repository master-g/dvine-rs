//! Example demonstrating item data file parsing.
//!
//! This example shows how to load and work with ITEM.dat files.

use dvine_rs::prelude::*;

pub fn test() -> Result<(), Box<dyn std::error::Error>> {
	// Example 1: Create a new item file
	println!("=== Example 1: Creating a new item file ===");
	let mut items = ItemFile::new();
	println!("Created empty item file with {} items", items.item_count());

	// Example 2: Add some items
	println!("\n=== Example 2: Adding items ===");
	let item1 = [0x01; 208]; // First item (all bytes set to 0x01)
	let item2 = [0x02; 208]; // Second item (all bytes set to 0x02)
	items.add_item(item1);
	items.add_item(item2);
	println!("Added 2 items, now have {} items", items.item_count());

	// Example 3: Access items
	println!("\n=== Example 3: Accessing items ===");
	if let Some(item) = items.get_item(0) {
		println!("Item 0: First byte = 0x{:02X}", item[0]);
		println!("Item 0: Size = {} bytes", item.len());
	}

	// Example 4: Iterate over items
	println!("\n=== Example 4: Iterating over items ===");
	for (index, item) in items.iter().enumerate() {
		println!("Item {}: First byte = 0x{:02X}", index, item[0]);
	}

	// Example 5: Serialize to bytes
	println!("\n=== Example 5: Serialization ===");
	let bytes = items.to_bytes();
	println!("Serialized to {} bytes", bytes.len());
	println!("  - Item count: 2 bytes");
	println!("  - Item data: {} bytes", items.item_count() as usize * 208);
	println!("  - Checksum: 128 bytes");

	// Example 6: Deserialize from bytes
	println!("\n=== Example 6: Deserialization ===");
	let loaded = ItemFile::from_bytes(&bytes)?;
	println!("Loaded {} items from bytes", loaded.item_count());

	// Verify the data matches
	assert_eq!(loaded.item_count(), items.item_count());
	println!("✓ Verification passed!");

	// Example 7: Working with real file (if available)
	println!("\n=== Example 7: Loading from file ===");
	match ItemFile::open("bin/ITEM.dat") {
		Ok(file) => {
			println!("✓ Successfully loaded ITEM.dat");
			println!("  - Item count: {}", file.item_count());
			println!("  - Total size: {} bytes", file.item_count() as usize * 208);

			// Show first few items
			println!("\nFirst 5 items:");
			for (index, item) in file.iter().take(5).enumerate() {
				let entry = ItemEntry::from(item.as_slice());
				let name = entry.name().unwrap_or_else(|| {
					let n = hex::encode(entry.raw_name());
					format!("0x{n}")
				});
				println!("  Item {index}: {name}");
			}
		}
		Err(e) => {
			println!("✗ Could not load ITEM.dat: {}", e);
			println!("  (This is expected if the file doesn't exist)");
		}
	}

	Ok(())
}
