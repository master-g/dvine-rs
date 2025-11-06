//! SPR frame structures and utilities.
//!
//! This module provides types for working with individual frames in SPR files,
//! including both sprite (color) data and mask (transparency) data.

use std::fmt;

use super::{Color, Palette};

/// SPR frame descriptor entry (24 bytes).
///
/// This structure describes a single frame's metadata, including offsets to both
/// the color sprite data and the transparency mask data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameEntry {
	/// Offset to color sprite data (relative to data area start)
	pub color_offset: u32,

	/// Offset to mask data (relative to data area start)
	pub mask_offset: u32,

	/// Frame width in pixels
	pub width: u32,

	/// Frame height in pixels
	pub height: u32,

	/// Hotspot X coordinate (registration point)
	pub hotspot_x: u32,

	/// Hotspot Y coordinate (registration point)
	pub hotspot_y: u32,
}

impl FrameEntry {
	/// Creates a new frame entry.
	///
	/// # Arguments
	///
	/// * `color_offset` - Offset to color data (relative to data area)
	/// * `mask_offset` - Offset to mask data (relative to data area)
	/// * `width` - Frame width in pixels
	/// * `height` - Frame height in pixels
	/// * `hotspot_x` - Hotspot X coordinate
	/// * `hotspot_y` - Hotspot Y coordinate
	pub fn new(
		color_offset: u32,
		mask_offset: u32,
		width: u32,
		height: u32,
		hotspot_x: u32,
		hotspot_y: u32,
	) -> Self {
		Self {
			color_offset,
			mask_offset,
			width,
			height,
			hotspot_x,
			hotspot_y,
		}
	}

	/// Returns the total number of pixels in this frame.
	#[inline]
	pub fn pixel_count(&self) -> usize {
		(self.width as usize) * (self.height as usize)
	}

	/// Returns the frame's width.
	#[inline]
	pub fn width(&self) -> u32 {
		self.width
	}

	/// Returns the frame's height.
	#[inline]
	pub fn height(&self) -> u32 {
		self.height
	}

	/// Returns the hotspot X coordinate.
	#[inline]
	pub fn hotspot_x(&self) -> u32 {
		self.hotspot_x
	}

	/// Returns the hotspot Y coordinate.
	#[inline]
	pub fn hotspot_y(&self) -> u32 {
		self.hotspot_y
	}

	/// Returns the color data offset.
	#[inline]
	pub fn color_offset(&self) -> u32 {
		self.color_offset
	}

	/// Returns the mask data offset.
	#[inline]
	pub fn mask_offset(&self) -> u32 {
		self.mask_offset
	}
}

impl fmt::Display for FrameEntry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}×{} (hotspot: {}, {})",
			self.width, self.height, self.hotspot_x, self.hotspot_y
		)
	}
}

/// Complete SPR frame with both sprite and mask data.
///
/// This structure combines a frame entry with its corresponding pixel data.
/// SPR frames contain two types of data:
/// - **Sprite data**: Indexed color values (typically 176-255 range mapping to palette 0-79)
/// - **Mask data**: Binary transparency values (0x00 = transparent, 0xFF = opaque)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
	/// Frame metadata
	entry: FrameEntry,

	/// Sprite (color) pixel data
	sprite_pixels: Vec<u8>,

	/// Mask (transparency) pixel data
	mask_pixels: Vec<u8>,
}

impl Frame {
	/// Creates a new frame with sprite and mask data.
	///
	/// # Arguments
	///
	/// * `entry` - Frame metadata
	/// * `sprite_pixels` - Sprite color data
	/// * `mask_pixels` - Mask transparency data
	///
	/// # Panics
	///
	/// Panics if the pixel data lengths don't match the frame dimensions.
	pub fn new(entry: FrameEntry, sprite_pixels: Vec<u8>, mask_pixels: Vec<u8>) -> Self {
		let expected_size = entry.pixel_count();
		assert_eq!(sprite_pixels.len(), expected_size, "Sprite pixel data size mismatch");
		assert_eq!(mask_pixels.len(), expected_size, "Mask pixel data size mismatch");

		Self {
			entry,
			sprite_pixels,
			mask_pixels,
		}
	}

	/// Creates a new frame with empty pixel data.
	///
	/// Both sprite and mask data are initialized to zero.
	pub fn new_empty(entry: FrameEntry) -> Self {
		let pixel_count = entry.pixel_count();
		Self {
			entry,
			sprite_pixels: vec![0; pixel_count],
			mask_pixels: vec![0; pixel_count],
		}
	}

	/// Returns a reference to the frame entry.
	#[inline]
	pub fn entry(&self) -> &FrameEntry {
		&self.entry
	}

	/// Returns a mutable reference to the frame entry.
	#[inline]
	pub fn entry_mut(&mut self) -> &mut FrameEntry {
		&mut self.entry
	}

	/// Returns a reference to the sprite pixel data.
	#[inline]
	pub fn sprite_pixels(&self) -> &[u8] {
		&self.sprite_pixels
	}

	/// Returns a mutable reference to the sprite pixel data.
	#[inline]
	pub fn sprite_pixels_mut(&mut self) -> &mut [u8] {
		&mut self.sprite_pixels
	}

	/// Returns a reference to the mask pixel data.
	#[inline]
	pub fn mask_pixels(&self) -> &[u8] {
		&self.mask_pixels
	}

	/// Returns a mutable reference to the mask pixel data.
	#[inline]
	pub fn mask_pixels_mut(&mut self) -> &mut [u8] {
		&mut self.mask_pixels
	}

	/// Returns the frame width.
	#[inline]
	pub fn width(&self) -> u32 {
		self.entry.width
	}

	/// Returns the frame height.
	#[inline]
	pub fn height(&self) -> u32 {
		self.entry.height
	}

	/// Returns the hotspot X coordinate.
	#[inline]
	pub fn hotspot_x(&self) -> u32 {
		self.entry.hotspot_x
	}

	/// Returns the hotspot Y coordinate.
	#[inline]
	pub fn hotspot_y(&self) -> u32 {
		self.entry.hotspot_y
	}

	/// Gets a sprite pixel value at the specified coordinates.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	///
	/// # Returns
	///
	/// The pixel value, or None if coordinates are out of bounds.
	pub fn get_sprite_pixel(&self, x: u32, y: u32) -> Option<u8> {
		if x >= self.entry.width || y >= self.entry.height {
			return None;
		}
		let index = (y * self.entry.width + x) as usize;
		self.sprite_pixels.get(index).copied()
	}

	/// Gets a mask pixel value at the specified coordinates.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	///
	/// # Returns
	///
	/// The mask value, or None if coordinates are out of bounds.
	pub fn get_mask_pixel(&self, x: u32, y: u32) -> Option<u8> {
		if x >= self.entry.width || y >= self.entry.height {
			return None;
		}
		let index = (y * self.entry.width + x) as usize;
		self.mask_pixels.get(index).copied()
	}

	/// Sets a sprite pixel value at the specified coordinates.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	/// * `value` - New pixel value
	///
	/// # Returns
	///
	/// `true` if the pixel was set, `false` if coordinates are out of bounds.
	pub fn set_sprite_pixel(&mut self, x: u32, y: u32, value: u8) -> bool {
		if x >= self.entry.width || y >= self.entry.height {
			return false;
		}
		let index = (y * self.entry.width + x) as usize;
		if let Some(pixel) = self.sprite_pixels.get_mut(index) {
			*pixel = value;
			true
		} else {
			false
		}
	}

	/// Sets a mask pixel value at the specified coordinates.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	/// * `value` - New mask value
	///
	/// # Returns
	///
	/// `true` if the mask was set, `false` if coordinates are out of bounds.
	pub fn set_mask_pixel(&mut self, x: u32, y: u32, value: u8) -> bool {
		if x >= self.entry.width || y >= self.entry.height {
			return false;
		}
		let index = (y * self.entry.width + x) as usize;
		if let Some(pixel) = self.mask_pixels.get_mut(index) {
			*pixel = value;
			true
		} else {
			false
		}
	}

	/// Returns an iterator over the rows of sprite pixel data.
	pub fn sprite_rows(&self) -> FrameRowIterator<'_> {
		FrameRowIterator::new(&self.sprite_pixels, self.entry.width as usize)
	}

	/// Returns an iterator over the rows of mask pixel data.
	pub fn mask_rows(&self) -> FrameRowIterator<'_> {
		FrameRowIterator::new(&self.mask_pixels, self.entry.width as usize)
	}

	/// Decodes a sprite pixel value from the raw format to palette index.
	///
	/// SPR files store palette indices in the range 176-255, which maps to
	/// palette indices 0-79.
	///
	/// # Arguments
	///
	/// * `raw_value` - Raw pixel value from the file
	///
	/// # Returns
	///
	/// The actual palette index (0-79), or 0 if the value is out of range.
	#[inline]
	pub fn decode_sprite_pixel(raw_value: u8) -> u8 {
		raw_value.saturating_sub(176)
	}

	/// Encodes a palette index to the raw sprite pixel format.
	///
	/// Converts palette indices 0-79 to the 176-255 range used in SPR files.
	///
	/// # Arguments
	///
	/// * `palette_index` - Palette index (0-79)
	///
	/// # Returns
	///
	/// The encoded pixel value for the file format.
	#[inline]
	pub fn encode_sprite_pixel(palette_index: u8) -> u8 {
		176_u8.saturating_add(palette_index.min(79))
	}

	/// Exports the sprite data as a PGM (Portable `GrayMap`) image.
	///
	/// This creates a grayscale image showing the raw sprite pixel values.
	pub fn sprite_to_pgm(&self) -> Vec<u8> {
		let mut output = Vec::new();

		// PGM header
		output.extend_from_slice(b"P5\n");
		output
			.extend_from_slice(format!("{} {}\n", self.entry.width, self.entry.height).as_bytes());
		output.extend_from_slice(b"255\n");

		// Pixel data
		output.extend_from_slice(&self.sprite_pixels);

		output
	}

	/// Exports the mask data as a PGM (Portable `GrayMap`) image.
	///
	/// This creates a grayscale image showing the transparency mask.
	pub fn mask_to_pgm(&self) -> Vec<u8> {
		let mut output = Vec::new();

		// PGM header
		output.extend_from_slice(b"P5\n");
		output
			.extend_from_slice(format!("{} {}\n", self.entry.width, self.entry.height).as_bytes());
		output.extend_from_slice(b"255\n");

		// Pixel data
		output.extend_from_slice(&self.mask_pixels);

		output
	}

	/// Converts the sprite data to ASCII art representation.
	///
	/// # Arguments
	///
	/// * `char_map` - Function to map pixel values to characters
	pub fn sprite_to_ascii_art<F>(&self, char_map: F) -> String
	where
		F: Fn(u8) -> char,
	{
		let mut result = String::new();

		for row in self.sprite_rows() {
			for &pixel in row {
				result.push(char_map(pixel));
			}
			result.push('\n');
		}

		result
	}

	/// Converts the mask data to ASCII art representation.
	///
	/// # Arguments
	///
	/// * `char_map` - Function to map mask values to characters
	pub fn mask_to_ascii_art<F>(&self, char_map: F) -> String
	where
		F: Fn(u8) -> char,
	{
		let mut result = String::new();

		for row in self.mask_rows() {
			for &pixel in row {
				result.push(char_map(pixel));
			}
			result.push('\n');
		}

		result
	}

	/// Converts the sprite data to ASCII art with a default character mapping.
	///
	/// Uses ' ' for low values and '#' for high values.
	pub fn sprite_to_ascii_art_default(&self) -> String {
		self.sprite_to_ascii_art(|pixel| {
			if pixel < 128 {
				' '
			} else {
				'#'
			}
		})
	}

	/// Converts the mask data to ASCII art with a default character mapping.
	///
	/// Uses ' ' for transparent (0x00) and '#' for opaque (0xFF).
	pub fn mask_to_ascii_art_default(&self) -> String {
		self.mask_to_ascii_art(|pixel| {
			if pixel < 128 {
				' '
			} else {
				'#'
			}
		})
	}

	/// Applies a palette to the sprite data, returning RGB pixel data.
	///
	/// This method decodes the sprite pixel indices and maps them to RGB colors
	/// using the provided palette.
	///
	/// # Arguments
	///
	/// * `palette` - The color palette to use
	///
	/// # Returns
	///
	/// A vector of RGB bytes (width × height × 3 bytes).
	/// Pixels are in row-major order, with each pixel as [R, G, B].
	pub fn apply_palette_rgb(&self, palette: &Palette) -> Vec<u8> {
		let pixel_count = self.entry.pixel_count();
		let mut rgb_data = Vec::with_capacity(pixel_count * 3);

		for &raw_pixel in &self.sprite_pixels {
			let palette_index = Self::decode_sprite_pixel(raw_pixel);
			let color = palette.get(palette_index);

			rgb_data.push(color.r);
			rgb_data.push(color.g);
			rgb_data.push(color.b);
		}

		rgb_data
	}

	/// Applies a palette to the sprite data, returning RGBA pixel data.
	///
	/// This method decodes the sprite pixel indices and maps them to RGBA colors
	/// using the provided palette. The alpha channel from the palette is used.
	///
	/// # Arguments
	///
	/// * `palette` - The color palette to use
	///
	/// # Returns
	///
	/// A vector of RGBA bytes (width × height × 4 bytes).
	/// Pixels are in row-major order, with each pixel as [R, G, B, A].
	pub fn apply_palette_rgba(&self, palette: &Palette) -> Vec<u8> {
		let pixel_count = self.entry.pixel_count();
		let mut rgba_data = Vec::with_capacity(pixel_count * 4);

		for &raw_pixel in &self.sprite_pixels {
			let palette_index = Self::decode_sprite_pixel(raw_pixel);
			let color = palette.get(palette_index);

			rgba_data.push(color.r);
			rgba_data.push(color.g);
			rgba_data.push(color.b);
			rgba_data.push(color.a);
		}

		rgba_data
	}

	/// Applies both palette and mask, returning RGBA pixel data with transparency.
	///
	/// This method combines the sprite color data with the mask transparency data,
	/// producing RGBA pixels where the alpha channel is determined by the mask.
	///
	/// # Arguments
	///
	/// * `palette` - The color palette to use for sprite colors
	///
	/// # Returns
	///
	/// A vector of RGBA bytes (width × height × 4 bytes).
	/// Pixels are in row-major order, with each pixel as [R, G, B, A].
	/// Alpha is taken from the mask data (0x00 = transparent, 0xFF = opaque).
	pub fn apply_palette_with_mask(&self, palette: &Palette) -> Vec<u8> {
		let pixel_count = self.entry.pixel_count();
		let mut rgba_data = Vec::with_capacity(pixel_count * 4);

		for i in 0..pixel_count {
			let raw_pixel = self.sprite_pixels[i];
			let mask_value = self.mask_pixels[i];

			let palette_index = Self::decode_sprite_pixel(raw_pixel);
			let color = palette.get(palette_index);

			rgba_data.push(color.r);
			rgba_data.push(color.g);
			rgba_data.push(color.b);
			rgba_data.push(mask_value); // Use mask as alpha
		}

		rgba_data
	}

	/// Gets the color at a specific pixel position using a palette.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	/// * `palette` - The color palette to use
	///
	/// # Returns
	///
	/// The color at the specified position, or None if coordinates are out of bounds.
	pub fn get_color_at(&self, x: u32, y: u32, palette: &Palette) -> Option<Color> {
		let raw_pixel = self.get_sprite_pixel(x, y)?;
		let palette_index = Self::decode_sprite_pixel(raw_pixel);
		Some(palette.get(palette_index))
	}

	/// Gets the RGBA color at a specific pixel position, including mask transparency.
	///
	/// # Arguments
	///
	/// * `x` - X coordinate (0-based)
	/// * `y` - Y coordinate (0-based)
	/// * `palette` - The color palette to use
	///
	/// # Returns
	///
	/// The RGBA color at the specified position with mask-based alpha,
	/// or None if coordinates are out of bounds.
	pub fn get_color_with_mask_at(&self, x: u32, y: u32, palette: &Palette) -> Option<Color> {
		let raw_pixel = self.get_sprite_pixel(x, y)?;
		let mask_value = self.get_mask_pixel(x, y)?;

		let palette_index = Self::decode_sprite_pixel(raw_pixel);
		let mut color = palette.get(palette_index);
		color.a = mask_value; // Override alpha with mask

		Some(color)
	}

	/// Returns an iterator over sprite rows with palette colors applied.
	///
	/// # Arguments
	///
	/// * `palette` - The color palette to use
	///
	/// # Returns
	///
	/// An iterator yielding rows of Color values.
	pub fn color_rows<'a>(&'a self, palette: &'a Palette) -> ColorRowIterator<'a> {
		ColorRowIterator {
			frame: self,
			palette,
			current_row: 0,
		}
	}
}

impl fmt::Display for Frame {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}×{} (hotspot: {}, {}) - {} pixels",
			self.entry.width,
			self.entry.height,
			self.entry.hotspot_x,
			self.entry.hotspot_y,
			self.entry.pixel_count()
		)
	}
}

/// Iterator over rows of pixel data in a frame.
#[derive(Debug, Clone)]
pub struct FrameRowIterator<'a> {
	pixels: &'a [u8],
	width: usize,
	current_row: usize,
	total_rows: usize,
}

impl<'a> FrameRowIterator<'a> {
	/// Creates a new row iterator.
	///
	/// # Arguments
	///
	/// * `pixels` - Pixel data to iterate over
	/// * `width` - Width of each row
	pub fn new(pixels: &'a [u8], width: usize) -> Self {
		let total_rows = if width > 0 {
			pixels.len() / width
		} else {
			0
		};

		Self {
			pixels,
			width,
			current_row: 0,
			total_rows,
		}
	}
}

impl<'a> Iterator for FrameRowIterator<'a> {
	type Item = &'a [u8];

	fn next(&mut self) -> Option<Self::Item> {
		if self.current_row >= self.total_rows {
			return None;
		}

		let start = self.current_row * self.width;
		let end = start + self.width;
		self.current_row += 1;

		Some(&self.pixels[start..end])
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.total_rows - self.current_row;
		(remaining, Some(remaining))
	}
}

impl<'a> ExactSizeIterator for FrameRowIterator<'a> {
	fn len(&self) -> usize {
		self.total_rows - self.current_row
	}
}

/// Iterator over rows of color data in a frame.
#[derive(Debug, Clone)]
pub struct ColorRowIterator<'a> {
	frame: &'a Frame,
	palette: &'a Palette,
	current_row: usize,
}

impl<'a> Iterator for ColorRowIterator<'a> {
	type Item = Vec<Color>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.current_row >= self.frame.height() as usize {
			return None;
		}

		let width = self.frame.width() as usize;
		let start = self.current_row * width;
		let end = start + width;

		let mut row = Vec::with_capacity(width);
		for &raw_pixel in &self.frame.sprite_pixels[start..end] {
			let palette_index = Frame::decode_sprite_pixel(raw_pixel);
			row.push(self.palette.get(palette_index));
		}

		self.current_row += 1;
		Some(row)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.frame.height() as usize - self.current_row;
		(remaining, Some(remaining))
	}
}

impl<'a> ExactSizeIterator for ColorRowIterator<'a> {
	fn len(&self) -> usize {
		self.frame.height() as usize - self.current_row
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_frame_entry() {
		let entry = FrameEntry::new(0, 100, 10, 20, 5, 10);
		assert_eq!(entry.width(), 10);
		assert_eq!(entry.height(), 20);
		assert_eq!(entry.pixel_count(), 200);
		assert_eq!(entry.hotspot_x(), 5);
		assert_eq!(entry.hotspot_y(), 10);
	}

	#[test]
	fn test_frame_creation() {
		let entry = FrameEntry::new(0, 100, 2, 2, 1, 1);
		let sprite = vec![176, 177, 178, 179];
		let mask = vec![0, 0xFF, 0, 0xFF];
		let frame = Frame::new(entry, sprite.clone(), mask.clone());

		assert_eq!(frame.width(), 2);
		assert_eq!(frame.height(), 2);
		assert_eq!(frame.sprite_pixels(), &sprite);
		assert_eq!(frame.mask_pixels(), &mask);
	}

	#[test]
	fn test_pixel_encoding() {
		assert_eq!(Frame::decode_sprite_pixel(176), 0);
		assert_eq!(Frame::decode_sprite_pixel(255), 79);
		assert_eq!(Frame::decode_sprite_pixel(200), 24);

		assert_eq!(Frame::encode_sprite_pixel(0), 176);
		assert_eq!(Frame::encode_sprite_pixel(79), 255);
		assert_eq!(Frame::encode_sprite_pixel(24), 200);
	}

	#[test]
	fn test_pixel_access() {
		let entry = FrameEntry::new(0, 100, 2, 2, 0, 0);
		let sprite = vec![176, 177, 178, 179];
		let mask = vec![0, 0xFF, 0, 0xFF];
		let mut frame = Frame::new(entry, sprite, mask);

		assert_eq!(frame.get_sprite_pixel(0, 0), Some(176));
		assert_eq!(frame.get_sprite_pixel(1, 1), Some(179));
		assert_eq!(frame.get_mask_pixel(1, 0), Some(0xFF));

		assert!(frame.set_sprite_pixel(0, 0, 200));
		assert_eq!(frame.get_sprite_pixel(0, 0), Some(200));
	}

	#[test]
	fn test_row_iterator() {
		let entry = FrameEntry::new(0, 100, 3, 2, 0, 0);
		let sprite = vec![1, 2, 3, 4, 5, 6];
		let mask = vec![0; 6];
		let frame = Frame::new(entry, sprite, mask);

		let rows: Vec<_> = frame.sprite_rows().collect();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0], &[1, 2, 3]);
		assert_eq!(rows[1], &[4, 5, 6]);
	}

	#[test]
	fn test_apply_palette_rgb() {
		let mut palette = Palette::new();
		palette.set(0, Color::rgb(255, 0, 0)); // Red
		palette.set(1, Color::rgb(0, 255, 0)); // Green

		let entry = FrameEntry::new(0, 100, 2, 1, 0, 0);
		let sprite = vec![Frame::encode_sprite_pixel(0), Frame::encode_sprite_pixel(1)];
		let mask = vec![0xFF; 2];
		let frame = Frame::new(entry, sprite, mask);

		let rgb = frame.apply_palette_rgb(&palette);
		assert_eq!(rgb.len(), 6); // 2 pixels × 3 bytes
		assert_eq!(&rgb[0..3], &[255, 0, 0]); // First pixel: red
		assert_eq!(&rgb[3..6], &[0, 255, 0]); // Second pixel: green
	}

	#[test]
	fn test_apply_palette_with_mask() {
		let mut palette = Palette::new();
		palette.set(0, Color::rgb(255, 0, 0));

		let entry = FrameEntry::new(0, 100, 2, 1, 0, 0);
		let sprite = vec![Frame::encode_sprite_pixel(0); 2];
		let mask = vec![0xFF, 0x00]; // First opaque, second transparent
		let frame = Frame::new(entry, sprite, mask);

		let rgba = frame.apply_palette_with_mask(&palette);
		assert_eq!(rgba.len(), 8); // 2 pixels × 4 bytes
		assert_eq!(&rgba[0..4], &[255, 0, 0, 0xFF]); // First pixel: opaque
		assert_eq!(&rgba[4..8], &[255, 0, 0, 0x00]); // Second pixel: transparent
	}

	#[test]
	fn test_get_color_at() {
		let mut palette = Palette::new();
		palette.set(0, Color::rgb(255, 0, 0));
		palette.set(1, Color::rgb(0, 255, 0));

		let entry = FrameEntry::new(0, 100, 2, 2, 0, 0);
		let sprite = vec![
			Frame::encode_sprite_pixel(0),
			Frame::encode_sprite_pixel(1),
			Frame::encode_sprite_pixel(1),
			Frame::encode_sprite_pixel(0),
		];
		let mask = vec![0xFF; 4];
		let frame = Frame::new(entry, sprite, mask);

		assert_eq!(frame.get_color_at(0, 0, &palette), Some(Color::rgb(255, 0, 0)));
		assert_eq!(frame.get_color_at(1, 0, &palette), Some(Color::rgb(0, 255, 0)));
		assert_eq!(frame.get_color_at(0, 1, &palette), Some(Color::rgb(0, 255, 0)));
		assert_eq!(frame.get_color_at(1, 1, &palette), Some(Color::rgb(255, 0, 0)));
	}

	#[test]
	fn test_color_rows() {
		let mut palette = Palette::new();
		palette.set(0, Color::rgb(255, 0, 0));
		palette.set(1, Color::rgb(0, 255, 0));

		let entry = FrameEntry::new(0, 100, 2, 2, 0, 0);
		let sprite = vec![
			Frame::encode_sprite_pixel(0),
			Frame::encode_sprite_pixel(1),
			Frame::encode_sprite_pixel(1),
			Frame::encode_sprite_pixel(0),
		];
		let mask = vec![0xFF; 4];
		let frame = Frame::new(entry, sprite, mask);

		let rows: Vec<_> = frame.color_rows(&palette).collect();
		assert_eq!(rows.len(), 2);
		assert_eq!(rows[0], vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)]);
		assert_eq!(rows[1], vec![Color::rgb(0, 255, 0), Color::rgb(255, 0, 0)]);
	}
}
