//! `.dsk` file format support.
//!
//! DSK files are block-based container files that store multiple files.
//! Each block is 2048 bytes (0x0800). The DSK file works in conjunction with
//! a PFT file which contains the metadata (file names, indices, sizes).
//!
//! # Design Philosophy
//!
//! This implementation treats DSK as an abstract concept - a wrapper around any
//! `Read + Seek` implementation. This allows for maximum flexibility:
//! - Memory-backed: `Cursor<Vec<u8>>`
//! - File-backed: `BufReader<File>`
//! - Network-backed: Custom network readers
//! - Any other seekable data source
//!
//! # Examples
//!
//! ## Reading from a file
//!
//! ```no_run
//! use dvine_types::file::{pft, dsk};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let pft = pft::File::open("data.pft").unwrap();
//! let file = File::open("data.dsk").unwrap();
//! let mut dsk = dsk::File::new(BufReader::new(file), pft);
//!
//! // Extract a file by name
//! let data = dsk.extract_by_name("SPRITE.DAT").unwrap();
//! ```
//!
//! ## Reading from memory
//!
//! ```no_run
//! use dvine_types::file::{pft, dsk};
//! use std::io::Cursor;
//!
//! let pft = pft::File::open("data.pft").unwrap();
//! let data = std::fs::read("data.dsk").unwrap();
//! let mut dsk = dsk::File::new(Cursor::new(data), pft);
//!
//! // Same API works for memory-backed version
//! let file_data = dsk.extract_by_index(0).unwrap();
//! ```
//!
//! ## Using convenience functions
//!
//! ```no_run
//! use dvine_types::file::{pft, dsk};
//!
//! let pft = pft::File::open("data.pft").unwrap();
//!
//! // File-backed (automatically uses BufReader)
//! let mut dsk = dsk::File::open("data.dsk", pft.clone()).unwrap();
//!
//! // Memory-backed (loads entire file)
//! let mut dsk_mem = dsk::File::from_bytes(std::fs::read("data.dsk").unwrap(), pft).unwrap();
//! ```

use std::{
	fs,
	io::{self, BufReader, Cursor, Read, Seek, SeekFrom},
	path::Path,
};

use super::{DSK_BLOCK_SIZE, error::DskError, pft};

/// DSK File abstraction over any seekable reader
///
/// This structure wraps any type implementing `Read + Seek` and provides
/// block-based access to files stored in DSK format. File metadata is
/// provided by the associated PFT file.
///
/// # Type Parameters
///
/// * `R` - Any type implementing `Read + Seek`
///
/// # Examples
///
/// ```no_run
/// use dvine_types::file::{pft, dsk};
/// use std::io::Cursor;
///
/// let pft = pft::File::empty();
/// let data = vec![0u8; 2048 * 10]; // 10 blocks
/// let mut dsk = dsk::File::new(Cursor::new(data), pft);
/// ```
pub struct File<R> {
	/// The underlying reader
	reader: R,

	/// Associated PFT file containing metadata
	pft: pft::File,

	/// Optional name/description for debugging
	name: Option<String>,
}

impl<R: Read + Seek> File<R> {
	/// Creates a new DSK file from a reader and PFT metadata
	///
	/// # Arguments
	///
	/// * `reader` - Any type implementing `Read + Seek`
	/// * `pft` - Associated PFT file containing metadata
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::{pft, dsk};
	/// use std::io::Cursor;
	///
	/// let pft = pft::File::empty();
	/// let data = vec![0u8; 2048];
	/// let dsk = dsk::File::new(Cursor::new(data), pft);
	/// ```
	pub fn new(reader: R, pft: pft::File) -> Self {
		Self {
			reader,
			pft,
			name: None,
		}
	}

	/// Creates a new DSK file with a name for debugging
	///
	/// # Arguments
	///
	/// * `reader` - Any type implementing `Read + Seek`
	/// * `pft` - Associated PFT file containing metadata
	/// * `name` - Name or description for this DSK file
	pub fn with_name(reader: R, pft: pft::File, name: impl Into<String>) -> Self {
		Self {
			reader,
			pft,
			name: Some(name.into()),
		}
	}

	/// Returns a reference to the associated PFT file
	pub fn pft(&self) -> &pft::File {
		&self.pft
	}

	/// Returns the name/description if set
	pub fn name(&self) -> Option<&str> {
		self.name.as_deref()
	}

	/// Returns the total size of the DSK data in bytes
	///
	/// # Errors
	///
	/// Returns an error if seeking fails
	pub fn size(&mut self) -> io::Result<u64> {
		let current = self.reader.stream_position()?;
		let size = self.reader.seek(SeekFrom::End(0))?;
		self.reader.seek(SeekFrom::Start(current))?;
		Ok(size)
	}

	/// Returns the total number of blocks in the DSK file
	///
	/// # Errors
	///
	/// Returns an error if determining the size fails
	pub fn num_blocks(&mut self) -> io::Result<usize> {
		Ok((self.size()? / DSK_BLOCK_SIZE as u64) as usize)
	}

	/// Returns the number of valid files in the container
	/// (entries with non-zero size or non-empty name)
	pub fn num_files(&self) -> usize {
		self.pft.entries().iter().filter(|e| e.is_valid()).count()
	}

	/// Checks if a file with the given name exists
	pub fn contains(&self, name: &str) -> bool {
		self.pft.find_entry(name).is_some()
	}

	/// Returns an iterator over entries only (without extracting data)
	///
	/// This is efficient if you only need to inspect metadata.
	/// Only returns valid entries (entries with non-zero size or non-empty name).
	pub fn entries(&self) -> impl Iterator<Item = &pft::Entry> {
		self.pft.entries().iter().filter(|e| e.is_valid())
	}

	/// Reads a single block by index
	///
	/// # Arguments
	///
	/// * `index` - Zero-based block index
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The block index is out of range
	/// - An I/O error occurs
	pub fn read_block(&mut self, index: usize) -> Result<Vec<u8>, DskError> {
		let offset = (index * DSK_BLOCK_SIZE) as u64;
		let total_size = self.size()?;

		if offset >= total_size {
			return Err(DskError::BlockOutOfRange {
				index: index as u32,
				total: (total_size / DSK_BLOCK_SIZE as u64) as usize,
			});
		}

		self.reader.seek(SeekFrom::Start(offset))?;

		let mut buffer = vec![0u8; DSK_BLOCK_SIZE];
		self.reader.read_exact(&mut buffer)?;

		Ok(buffer)
	}

	/// Reads multiple consecutive blocks starting from the given index
	///
	/// # Arguments
	///
	/// * `start_index` - Starting block index
	/// * `count` - Number of blocks to read
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Any block index is out of range
	/// - An I/O error occurs
	pub fn read_blocks(&mut self, start_index: usize, count: usize) -> Result<Vec<u8>, DskError> {
		let offset = (start_index * DSK_BLOCK_SIZE) as u64;
		let bytes_to_read = count * DSK_BLOCK_SIZE;
		let end_offset = offset + bytes_to_read as u64;
		let total_size = self.size()?;

		if end_offset > total_size {
			return Err(DskError::BlockOutOfRange {
				index: (start_index + count - 1) as u32,
				total: (total_size / DSK_BLOCK_SIZE as u64) as usize,
			});
		}

		self.reader.seek(SeekFrom::Start(offset))?;

		let mut buffer = vec![0u8; bytes_to_read];
		self.reader.read_exact(&mut buffer)?;

		Ok(buffer)
	}

	/// Extracts file data for a given PFT entry
	///
	/// This reads the blocks specified by the entry's index and returns
	/// only the actual file data (trimming any padding in the last block).
	///
	/// # Arguments
	///
	/// * `entry` - PFT entry describing the file
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The required blocks are out of range
	/// - The actual size exceeds available data
	/// - An I/O error occurs
	pub fn extract(&mut self, entry: &pft::Entry) -> Result<Vec<u8>, DskError> {
		let start_index = entry.index as usize;
		let blocks_needed = entry.blocks_needed() as usize;
		let actual_size = entry.actual_size as usize;

		// Read all blocks for this entry
		let block_data = self.read_blocks(start_index, blocks_needed)?;

		// Validate actual size
		if actual_size > block_data.len() {
			return Err(DskError::InvalidExtraction {
				required: actual_size,
				available: block_data.len(),
			});
		}

		// Return only the actual file data (trim padding)
		Ok(block_data[..actual_size].to_vec())
	}

	/// Extracts file data by entry index
	///
	/// # Arguments
	///
	/// * `index` - Index of the entry in the PFT file
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The index is out of range
	/// - Extraction fails
	/// - An I/O error occurs
	pub fn extract_by_index(&mut self, index: usize) -> Result<Vec<u8>, DskError> {
		// Get entry information before borrowing self mutably
		let entry = *self.pft.get_entry(index).ok_or_else(|| DskError::InvalidExtraction {
			required: index,
			available: self.pft.num_entries(),
		})?;

		self.extract(&entry)
	}

	/// Extracts file data by file name
	///
	/// # Arguments
	///
	/// * `name` - Name of the file to extract
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The file is not found
	/// - Extraction fails
	/// - An I/O error occurs
	pub fn extract_by_name(&mut self, name: &str) -> Result<Vec<u8>, DskError> {
		// Get entry information before borrowing self mutably
		let entry = *self.pft.find_entry(name).ok_or_else(|| {
			DskError::IOError(io::Error::new(
				io::ErrorKind::NotFound,
				format!("File '{}' not found", name),
			))
		})?;

		self.extract(&entry)
	}

	/// Returns an iterator over all entries and their extracted data
	///
	/// This is useful for processing all files in the DSK container.
	///
	/// # Note
	///
	/// The iterator will read files on-demand, so it requires mutable access.
	/// Each file is loaded into memory as it's iterated.
	pub fn iter(&mut self) -> DskIterator<'_, R> {
		DskIterator {
			dsk: self,
			index: 0,
		}
	}

	/// Validates the DSK file structure
	///
	/// Checks that:
	/// - All entries reference valid blocks
	/// - No entries exceed the available space
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - Any entry references out-of-range blocks
	/// - An I/O error occurs while determining file size
	pub fn validate(&mut self) -> Result<(), DskError> {
		let total_blocks = self.num_blocks()?;

		for entry in self.pft.entries() {
			let start_index = entry.index as usize;
			let blocks_needed = entry.blocks_needed() as usize;
			let end_index = start_index + blocks_needed;

			if end_index > total_blocks {
				return Err(DskError::FileTooLarge {
					size: entry.actual_size as usize,
					blocks_needed,
					blocks_available: total_blocks.saturating_sub(start_index),
				});
			}
		}

		Ok(())
	}

	/// Consumes this DSK file and returns the underlying reader
	pub fn into_inner(self) -> R {
		self.reader
	}

	/// Returns a reference to the underlying reader
	pub fn get_ref(&self) -> &R {
		&self.reader
	}

	/// Returns a mutable reference to the underlying reader
	pub fn get_mut(&mut self) -> &mut R {
		&mut self.reader
	}
}

// Convenience constructors for common types
impl File<BufReader<fs::File>> {
	/// Opens a DSK file from a path with buffered reading
	///
	/// # Arguments
	///
	/// * `path` - Path to the DSK file
	/// * `pft` - Associated PFT file containing metadata
	///
	/// # Errors
	///
	/// Returns an error if the file cannot be opened
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::{pft, dsk};
	///
	/// let pft = pft::File::open("data.pft").unwrap();
	/// let mut dsk = dsk::File::open_with_pft("data.dsk", pft).unwrap();
	/// ```
	pub fn open_with_pft(path: impl AsRef<Path>, pft: pft::File) -> io::Result<Self> {
		let path_ref = path.as_ref();
		let file = fs::File::open(path_ref)?;
		let reader = BufReader::new(file);
		Ok(Self::with_name(reader, pft, path_ref.display().to_string()))
	}

	/// Opens a DSK file and its associated PFT file from a directory and base name
	///
	/// # Arguments
	///
	/// * `dir` - Directory containing the DSK and PFT files
	/// * `name` - Base name of the files (without extension)
	///
	/// # Errors
	///
	/// Returns an error if either file cannot be opened
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::{pft, dsk};
	///
	/// let mut dsk = dsk::File::open("data", "ANM").unwrap();
	/// ```
	///
	/// This will open "data/ANM.DSK" and "data/ANM.PFT"
	pub fn open(dir: impl AsRef<Path>, name: &str) -> Result<Self, DskError> {
		let pft_name = format!("{}.PFT", name);
		let pft_path = dir.as_ref().join(pft_name);
		let pft = pft::File::open(&pft_path)?;

		let dsk_name = format!("{}.DSK", name);
		let dsk_path = dir.as_ref().join(dsk_name);
		let dsk = Self::open_with_pft(&dsk_path, pft)?;

		Ok(dsk)
	}
}

impl File<Cursor<Vec<u8>>> {
	/// Creates a DSK file from a byte vector (memory-backed)
	///
	/// # Arguments
	///
	/// * `data` - Raw block data
	/// * `pft` - Associated PFT file containing metadata
	///
	/// # Examples
	///
	/// ```no_run
	/// use dvine_types::file::{pft, dsk};
	///
	/// let pft = pft::File::empty();
	/// let data = vec![0u8; 2048 * 5]; // 5 blocks
	/// let mut dsk = dsk::File::from_bytes(data, pft).unwrap();
	/// ```
	pub fn from_bytes(data: Vec<u8>, pft: pft::File) -> io::Result<Self> {
		Ok(Self::new(Cursor::new(data), pft))
	}
}

impl<R> std::fmt::Display for File<R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(name) = &self.name {
			writeln!(f, "DSK File: {}", name)?;
		} else {
			writeln!(f, "DSK File")?;
		}
		writeln!(f, "  Contains {} files:", self.pft.num_entries())?;
		for entry in self.pft.entries() {
			writeln!(f, "    {}", entry)?;
		}
		Ok(())
	}
}

impl<R: std::fmt::Debug> std::fmt::Debug for File<R> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("File")
			.field("reader", &self.reader)
			.field("pft", &self.pft)
			.field("name", &self.name)
			.finish()
	}
}

/// Iterator over DSK file entries and their data
///
/// This iterator yields `Result<(pft::Entry, Vec<u8>), DskError>` for each file
/// in the DSK container. Files are loaded on-demand as the iterator progresses.
///
/// Note: This returns owned `Entry` values to avoid lifetime issues.
///
/// # Type Parameters
///
/// * `R` - The reader type implementing `Read + Seek`
pub struct DskIterator<'a, R> {
	dsk: &'a mut File<R>,
	index: usize,
}

impl<'a, R: Read + Seek> Iterator for DskIterator<'a, R> {
	type Item = Result<(pft::Entry, Vec<u8>), DskError>;

	fn next(&mut self) -> Option<Self::Item> {
		// Skip invalid entries
		loop {
			if self.index >= self.dsk.pft.num_entries() {
				return None;
			}

			// Copy the entry (it's a small Copy type)
			let entry = *self.dsk.pft.get_entry(self.index)?;
			self.index += 1;

			// Skip invalid entries (zero size and empty name)
			if !entry.is_valid() {
				continue;
			}

			let result = self.dsk.extract(&entry);
			return Some(result.map(|data| (entry, data)));
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		// We can't know exactly how many valid entries remain without iterating
		// So we provide a conservative estimate
		let remaining = self.dsk.pft.num_entries().saturating_sub(self.index);
		(0, Some(remaining))
	}
}

// Note: We can't implement ExactSizeIterator anymore because we filter invalid entries
// and can't know the exact count without consuming the iterator

/// Type alias for file-backed DSK files with buffering
pub type DskFile = File<BufReader<fs::File>>;

/// Type alias for memory-backed DSK files
pub type DskMemory = File<Cursor<Vec<u8>>>;
