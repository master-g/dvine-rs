//! Item data structures and functions for handling ITEM files.

/// An entry in the ITEM.dat file, representing a single item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEntry {
	/// Item ID
	pub id: u16,

	/// raw name in Shift-JIS encoding
	raw_name: [u8; 23],

	/// Unknown field (4 bytes)
	unknown1: [u8; 4],

	/// Item type
	pub item_type: u8,
}

impl ItemEntry {
	/// Returns the item name as a UTF-8 string.
	pub fn name(&self) -> String {
		// Convert raw_name from Shift-JIS to UTF-8
		String::from_utf8_lossy(&self.raw_name).trim_end_matches('\0').trim().to_string()
	}
}

impl From<&[u8]> for ItemEntry {
	fn from(data: &[u8]) -> Self {
		let id = u16::from_le_bytes([data[0], data[1]]);
		let mut raw_name = [0u8; 23];
		raw_name.copy_from_slice(&data[2..25]);
		let mut unknown1 = [0u8; 4];
		unknown1.copy_from_slice(&data[25..29]);
		let item_type = data[29];

		Self {
			id,
			raw_name,
			unknown1,
			item_type,
		}
	}
}
