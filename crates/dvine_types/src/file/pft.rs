//! PFT (Pack File Table) file format support.
//!
//! PFT files contain metadata for files stored in DSK containers,
//! including file names, block indices, and sizes.

use std::{
	fmt::Formatter,
	io::{self, Read},
};

use super::{DSK_BLOCK_SIZE, DvFileError, FileType};

mod constants {
	/// Magic number for PFT files
	pub const MAGIC: [u8; 4] = [0x10, 0x00, 0x00, 0x08];

	/// Size of the PFT header in bytes
	pub const HEADER_SIZE: usize = 16;
}

/// PFT File Header
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Header {
	/// Magic Number
	pub magic: [u8; 4],
	/// Number of entries
	pub num_entries: u32,
	/// Padding (reserved)
	pub padding: u64,
}

impl Header {
	/// Creates a new header with the specified number of entries
	pub fn new(num_entries: u32) -> Self {
		Self {
			magic: constants::MAGIC,
			num_entries,
			padding: 0,
		}
	}

	/// Loads header from a byte slice
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		if data.len() < constants::HEADER_SIZE {
			return Err(DvFileError::insufficient_data(
				FileType::Pft,
				constants::HEADER_SIZE,
				data.len(),
			));
		}

		let magic: [u8; 4] = data[0..4].try_into()?;
		if magic != constants::MAGIC {
			return Err(DvFileError::invalid_magic(FileType::Pft, &constants::MAGIC, &magic));
		}

		let num_entries = u32::from_le_bytes(data[4..8].try_into()?);
		let padding = u64::from_le_bytes(data[8..16].try_into()?);

		Ok(Self {
			magic,
			num_entries,
			padding,
		})
	}

	/// Loads header from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut buffer = [0u8; constants::HEADER_SIZE];
		reader.read_exact(&mut buffer)?;
		Self::from_bytes(&buffer)
	}

	/// Serializes header to bytes
	pub fn to_bytes(self) -> [u8; constants::HEADER_SIZE] {
		let mut buffer = [0u8; constants::HEADER_SIZE];
		buffer[0..4].copy_from_slice(&self.magic);
		buffer[4..8].copy_from_slice(&self.num_entries.to_le_bytes());
		buffer[8..16].copy_from_slice(&self.padding.to_le_bytes());
		buffer
	}

	/// Returns the size of the header in bytes
	pub const fn size() -> usize {
		constants::HEADER_SIZE
	}
}

impl Default for Header {
	fn default() -> Self {
		Self {
			magic: constants::MAGIC,
			num_entries: 0,
			padding: 0,
		}
	}
}

impl std::fmt::Display for Header {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "PFT {{ magic: {:02X?}, num_entries: {} }}", self.magic, self.num_entries)
	}
}

impl TryFrom<&[u8]> for Header {
	type Error = DvFileError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for Header {
	type Error = DvFileError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&Vec<u8>> for Header {
	type Error = DvFileError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<[u8; constants::HEADER_SIZE]> for Header {
	type Error = DvFileError;

	fn try_from(value: [u8; constants::HEADER_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&[u8; constants::HEADER_SIZE]> for Header {
	type Error = DvFileError;

	fn try_from(value: &[u8; constants::HEADER_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<Header> for [u8; constants::HEADER_SIZE] {
	fn from(header: Header) -> Self {
		header.to_bytes()
	}
}

impl From<&Header> for [u8; constants::HEADER_SIZE] {
	fn from(header: &Header) -> Self {
		header.to_bytes()
	}
}

impl From<Header> for Vec<u8> {
	fn from(header: Header) -> Self {
		header.to_bytes().to_vec()
	}
}

impl From<&Header> for Vec<u8> {
	fn from(header: &Header) -> Self {
		header.to_bytes().to_vec()
	}
}

/// PFT Entry
/// Every entry in DSK files has a corresponding PFT entry
/// with its name, index, and actual size.
/// every block in the DSK file is 0x0800 bytes, e.g 2048 bytes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entry {
	/// ASCII name for the entry
	pub raw_name: [u8; 8],

	/// Index of the entry, in little-endian format
	pub index: u32,

	/// Actual size in bytes, in little-endian format
	pub actual_size: u32,
}

const ENTRY_SIZE: usize = 16;

impl Entry {
	/// Creates a new entry with the given name, index, and actual size
	pub fn new(name: &str, index: u32, actual_size: u32) -> Self {
		let mut raw_name = [0u8; 8];
		let bytes = name.as_bytes();
		let len = bytes.len().min(8);
		raw_name[..len].copy_from_slice(&bytes[..len]);

		Self {
			raw_name,
			index,
			actual_size,
		}
	}

	/// Returns the name as a string, trimming null bytes and whitespace
	pub fn name(&self) -> String {
		String::from_utf8_lossy(&self.raw_name).trim_end_matches('\0').trim().to_string()
	}

	/// Loads entry from a byte slice
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		let mut cursor = io::Cursor::new(data);
		Self::from_reader(&mut cursor)
	}

	/// Loads entry from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		let mut raw_name = [0u8; 8];
		reader.read_exact(&mut raw_name)?;

		let mut index_bytes = [0u8; 4];
		reader.read_exact(&mut index_bytes)?;
		let index = u32::from_le_bytes(index_bytes);

		let mut size_bytes = [0u8; 4];
		reader.read_exact(&mut size_bytes)?;
		let actual_size = u32::from_le_bytes(size_bytes);

		Ok(Self {
			raw_name,
			index,
			actual_size,
		})
	}

	/// Serializes entry to bytes
	pub fn to_bytes(self) -> [u8; ENTRY_SIZE] {
		let mut buffer = [0u8; ENTRY_SIZE];
		buffer[0..8].copy_from_slice(&self.raw_name);
		buffer[8..12].copy_from_slice(&self.index.to_le_bytes());
		buffer[12..16].copy_from_slice(&self.actual_size.to_le_bytes());
		buffer
	}

	/// Returns the size of an entry in bytes
	pub const fn size() -> usize {
		ENTRY_SIZE
	}

	/// Returns the block size for DSK files
	pub const fn block_size() -> usize {
		DSK_BLOCK_SIZE
	}

	/// Calculates the number of blocks needed for this entry
	pub fn blocks_needed(&self) -> u32 {
		self.actual_size.div_ceil(DSK_BLOCK_SIZE as u32)
	}

	/// Checks if this entry is valid (has non-zero size or non-empty name)
	///
	/// An entry is considered invalid if both:
	/// - The `actual_size` is 0
	/// - The `raw_name` is all zeros (empty)
	pub fn is_valid(&self) -> bool {
		// Entry is valid if it has a non-zero size
		if self.actual_size > 0 {
			return true;
		}

		// Or if it has a non-empty name
		self.raw_name.iter().any(|&b| b != 0)
	}
}

impl std::fmt::Display for Entry {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Entry {{ name: '{}', index: {}, actual_size: {} }}",
			self.name(),
			self.index,
			self.actual_size
		)
	}
}

impl TryFrom<&[u8]> for Entry {
	type Error = DvFileError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for Entry {
	type Error = DvFileError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&Vec<u8>> for Entry {
	type Error = DvFileError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<[u8; ENTRY_SIZE]> for Entry {
	type Error = DvFileError;

	fn try_from(value: [u8; ENTRY_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&[u8; ENTRY_SIZE]> for Entry {
	type Error = DvFileError;

	fn try_from(value: &[u8; ENTRY_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<Entry> for [u8; ENTRY_SIZE] {
	fn from(entry: Entry) -> Self {
		entry.to_bytes()
	}
}

impl From<&Entry> for [u8; ENTRY_SIZE] {
	fn from(entry: &Entry) -> Self {
		entry.to_bytes()
	}
}

impl From<Entry> for Vec<u8> {
	fn from(entry: Entry) -> Self {
		entry.to_bytes().to_vec()
	}
}

impl From<&Entry> for Vec<u8> {
	fn from(entry: &Entry) -> Self {
		entry.to_bytes().to_vec()
	}
}

/// PFT File
/// Contains a header and a list of entries
/// FIXME: Entry might have invalid data, e.g. zero size but non-zero index
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct File {
	header: Header,
	/// TODO: or use `BtreeMap` for better performance on lookups?
	entries: Vec<Entry>,
}

impl File {
	/// Creates a new PFT file with the given entries
	pub fn new(entries: Vec<Entry>) -> Self {
		let num_entries = entries.len() as u32;
		Self {
			header: Header::new(num_entries),
			entries,
		}
	}

	/// Creates an empty PFT file
	pub fn empty() -> Self {
		Self {
			header: Header::default(),
			entries: Vec::new(),
		}
	}

	/// Opens a PFT file from any reader
	pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DvFileError> {
		let mut file = std::fs::File::open(path)?;
		let pft = Self::from_reader(&mut file)?;
		pft.validate()?;
		Ok(pft)
	}

	/// Returns a reference to the header
	pub fn header(&self) -> &Header {
		&self.header
	}

	/// Returns a reference to the entries
	pub fn entries(&self) -> &[Entry] {
		&self.entries
	}

	/// Returns the number of entries
	pub fn num_entries(&self) -> usize {
		self.entries.len()
	}

	/// Gets an entry by index
	pub fn get_entry(&self, index: usize) -> Option<&Entry> {
		self.entries.get(index)
	}

	/// Finds an entry by name
	pub fn find_entry(&self, name: &str) -> Option<&Entry> {
		self.entries.iter().find(|e| e.name().eq_ignore_ascii_case(name))
	}

	/// Adds an entry to the file
	pub fn add_entry(&mut self, entry: Entry) {
		self.entries.push(entry);
		self.header.num_entries = self.entries.len() as u32;
	}

	/// Loads file from a byte slice
	pub fn from_bytes(data: &[u8]) -> Result<Self, DvFileError> {
		// Parse header
		let header = Header::from_bytes(data)?;

		let expected_entries = header.num_entries as usize;
		let header_size = Header::size();
		let required_size = header_size + expected_entries * Entry::size();

		if data.len() < required_size {
			return Err(DvFileError::insufficient_data(FileType::Pft, required_size, data.len()));
		}

		// Parse entries
		let mut entries = Vec::with_capacity(expected_entries);
		let mut offset = header_size;

		for _ in 0..expected_entries {
			let entry = Entry::from_bytes(&data[offset..])?;
			entries.push(entry);
			offset += Entry::size();
		}

		Ok(Self {
			header,
			entries,
		})
	}

	/// Loads file from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, DvFileError> {
		// Read header
		let header = Header::from_reader(reader)?;

		// Read entries
		let mut entries = Vec::with_capacity(header.num_entries as usize);
		for _ in 0..header.num_entries {
			let entry = Entry::from_reader(reader)?;
			entries.push(entry);
		}

		Ok(Self {
			header,
			entries,
		})
	}

	/// Serializes file to bytes
	pub fn to_bytes(&self) -> Vec<u8> {
		let total_size = Header::size() + self.entries.len() * Entry::size();
		let mut buffer = Vec::with_capacity(total_size);

		// Write header
		buffer.extend_from_slice(&self.header.to_bytes());

		// Write entries
		for entry in &self.entries {
			buffer.extend_from_slice(&entry.to_bytes());
		}

		buffer
	}

	/// Validates that the header's entry count matches the actual entries
	pub fn validate(&self) -> Result<(), DvFileError> {
		if self.header.num_entries as usize != self.entries.len() {
			return Err(DvFileError::EntryCountMismatch {
				file_type: FileType::Pft,
				expected: self.header.num_entries,
				actual: self.entries.len(),
			});
		}
		Ok(())
	}
}

impl Default for File {
	fn default() -> Self {
		Self::empty()
	}
}

impl std::fmt::Display for File {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "PFT File: {}\nEntries:\n", self.header)?;
		for entry in &self.entries {
			writeln!(f, "  {}", entry)?;
		}
		Ok(())
	}
}

impl TryFrom<&[u8]> for File {
	type Error = DvFileError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for File {
	type Error = DvFileError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&Vec<u8>> for File {
	type Error = DvFileError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<File> for Vec<u8> {
	fn from(file: File) -> Self {
		file.to_bytes()
	}
}

impl From<&File> for Vec<u8> {
	fn from(file: &File) -> Self {
		file.to_bytes()
	}
}

impl From<Vec<Entry>> for File {
	fn from(entries: Vec<Entry>) -> Self {
		Self::new(entries)
	}
}
