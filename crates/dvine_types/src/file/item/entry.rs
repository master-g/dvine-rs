//! Item data structures and functions for handling ITEM files.

use encoding_rs::SHIFT_JIS;

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
	/// Returns the raw name bytes in Shift-JIS encoding.
	pub fn raw_name(&self) -> &[u8; 23] {
		&self.raw_name
	}

	/// Returns the item name as a UTF-8 string.
	pub fn name(&self) -> Option<String> {
		// Convert raw_name from Shift-JIS to UTF-8
		let (cow, _encoding_used, had_error) = SHIFT_JIS.decode(&self.raw_name);
		if had_error {
			return None;
		}

		let str = cow.trim().to_string();
		Some(str)
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
