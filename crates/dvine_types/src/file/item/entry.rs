//! Item data structures and functions for handling ITEM files.

use encoding_rs::SHIFT_JIS;

/// An entry in the ITEM.dat file, representing a single item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEntry {
	/// Item ID
	/// first 2 bytes, 0x00 - 0x01
	pub id: u16,

	/// raw name in Shift-JIS encoding
	/// 0x02 - 0x15 (20 bytes)
	raw_name: [u8; 20],

	/// rest
	extra: [u8; 16],
}

impl ItemEntry {
	/// Returns the raw name bytes in Shift-JIS encoding.
	pub fn raw_name(&self) -> &[u8; 20] {
		&self.raw_name
	}

	/// Returns the item name as a UTF-8 string.
	pub fn name(&self) -> Option<String> {
		// collect bytes until the first 0x00 byte
		let null_pos = self.raw_name.iter().position(|&b| b == 0x00).unwrap_or(self.raw_name.len());

		let raw_bytes = &self.raw_name[..null_pos];

		// Convert raw_name from Shift-JIS to UTF-8
		let (cow, _encoding_used, had_error) = SHIFT_JIS.decode(raw_bytes);
		if had_error {
			return None;
		}

		let str = cow.trim().to_string();
		Some(str)
	}

	/// Returns the extra data bytes.
	pub fn extra(&self) -> &[u8; 16] {
		&self.extra
	}
}

impl From<&[u8]> for ItemEntry {
	fn from(data: &[u8]) -> Self {
		let id = u16::from_le_bytes([data[0], data[1]]);
		let mut raw_name = [0u8; 20];
		raw_name.copy_from_slice(&data[2..22]);

		let mut extra = [0u8; 16];
		extra.copy_from_slice(&data[22..38]);

		Self {
			id,
			raw_name,
			extra,
		}
	}
}
