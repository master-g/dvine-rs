//! Statup configuration file parser and writer.

use std::{
	fmt::Formatter,
	io::{self, Read},
};

use super::error::StartupIniError;

/// Size of the startup.ini file in bytes
const STARTUP_INI_SIZE: usize = 24;

/// Game opening mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpeningMode {
	/// Normal mode
	Normal = 0,
	/// Loop startup
	Loop = 1,
	/// Skip opening
	Skip = 2,
}

impl OpeningMode {
	/// Converts a u8 value to `OpeningMode`
	pub fn from_u8(value: u8) -> Result<Self, StartupIniError> {
		match value {
			0 => Ok(Self::Normal),
			1 => Ok(Self::Loop),
			2 => Ok(Self::Skip),
			_ => Err(StartupIniError::InvalidOpeningMode(value)),
		}
	}

	/// Converts `OpeningMode` to u8
	pub fn to_u8(self) -> u8 {
		self as u8
	}
}

impl Default for OpeningMode {
	fn default() -> Self {
		Self::Normal
	}
}

impl std::fmt::Display for OpeningMode {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Normal => write!(f, "Normal"),
			Self::Loop => write!(f, "Loop"),
			Self::Skip => write!(f, "Skip"),
		}
	}
}

/// VGA mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VgaMode {
	/// Default resolution
	Default = 0,
	/// VGA compatible mode
	VgaCompatible = 1,
}

impl VgaMode {
	/// Converts a u8 value to `VgaMode`
	pub fn from_u8(value: u8) -> Result<Self, StartupIniError> {
		match value {
			0 => Ok(Self::Default),
			1 => Ok(Self::VgaCompatible),
			_ => Err(StartupIniError::InvalidVgaMode(value)),
		}
	}

	/// Converts `VgaMode` to u8
	pub fn to_u8(self) -> u8 {
		self as u8
	}
}

impl Default for VgaMode {
	fn default() -> Self {
		Self::Default
	}
}

impl std::fmt::Display for VgaMode {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Default => write!(f, "Default"),
			Self::VgaCompatible => write!(f, "VGA Compatible"),
		}
	}
}

/// Rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenderMode {
	/// VSYNC ON
	VsyncOn = 0,
	/// VSYNC OFF
	VsyncOff = 1,
}

impl RenderMode {
	/// Converts a u32 value to `RenderMode`
	pub fn from_u32(value: u32) -> Result<Self, StartupIniError> {
		match value {
			0 => Ok(Self::VsyncOn),
			1 => Ok(Self::VsyncOff),
			_ => Err(StartupIniError::InvalidRenderMode(value)),
		}
	}

	/// Converts `RenderMode` to u32
	pub fn to_u32(self) -> u32 {
		self as u32
	}
}

impl Default for RenderMode {
	fn default() -> Self {
		Self::VsyncOn
	}
}

impl std::fmt::Display for RenderMode {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::VsyncOn => write!(f, "VSYNC ON"),
			Self::VsyncOff => write!(f, "VSYNC OFF"),
		}
	}
}

/// Startup INI file structure, for file `startup.ini`
///
/// This structure maintains binary compatibility with the original file format.
/// Total size: 24 bytes (1 + 1 + 2 + 4 + 16)
///
/// Layout:
/// ```text
/// Offset  Size  Description
/// ------  ----  -----------------------------------------
/// 0       1     Game opening mode
/// 1       1     VGA mode
/// 2       2     Reserved (padding)
/// 4       4     Rendering mode
/// 8       16    Window rectangle (left, top, right, bottom)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartupIni {
	/// Game opening mode (1 byte)
	opening_mode: OpeningMode,

	/// VGA mode (1 byte)
	vga_mode: VgaMode,

	/// Reserved bytes, for padding (2 bytes)
	reserved: [u8; 2],

	/// Rendering mode (4 bytes)
	render_mode: RenderMode,

	/// Window rectangle (left, top, right, bottom) (16 bytes)
	/// This is for setting dialog itself, not the game window
	/// so we can just ignore it for now
	window_rect: [u32; 4],
}

impl StartupIni {
	/// Opens a startup.ini file from a path
	pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
		let mut file = std::fs::File::open(path)?;
		Self::from_reader(&mut file)
	}

	/// Creates a `StartupIni` from raw bytes
	///
	/// # Arguments
	/// * `data` - Byte slice containing the binary data
	///
	/// # Returns
	/// * `Ok(StartupIni)` if parsing succeeds
	/// * `Err(StartupIniError)` if the data is invalid or too short
	pub fn from_bytes(data: &[u8]) -> Result<Self, StartupIniError> {
		if data.len() < STARTUP_INI_SIZE {
			return Err(StartupIniError::InsufficientData {
				expected: STARTUP_INI_SIZE,
				actual: data.len(),
			});
		}

		let opening_mode = OpeningMode::from_u8(data[0])?;
		let vga_mode = VgaMode::from_u8(data[1])?;
		let reserved = [data[2], data[3]];

		let render_mode_value = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
		let render_mode = RenderMode::from_u32(render_mode_value)?;

		let window_rect = [
			u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
			u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
			u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
			u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
		];

		Ok(Self {
			opening_mode,
			vga_mode,
			reserved,
			render_mode,
			window_rect,
		})
	}

	/// Loads startup.ini from any reader
	pub fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
		let mut buffer = [0u8; STARTUP_INI_SIZE];
		reader.read_exact(&mut buffer)?;
		Self::from_bytes(&buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}

	/// Converts the `StartupIni` to bytes
	///
	/// # Returns
	/// A 24-byte array containing the binary representation
	pub fn to_bytes(&self) -> [u8; STARTUP_INI_SIZE] {
		let mut bytes = [0u8; STARTUP_INI_SIZE];

		bytes[0] = self.opening_mode.to_u8();
		bytes[1] = self.vga_mode.to_u8();
		bytes[2..4].copy_from_slice(&self.reserved);
		bytes[4..8].copy_from_slice(&self.render_mode.to_u32().to_le_bytes());

		for (i, &rect_value) in self.window_rect.iter().enumerate() {
			let offset = 8 + i * 4;
			bytes[offset..offset + 4].copy_from_slice(&rect_value.to_le_bytes());
		}

		bytes
	}

	/// Returns the size of the startup.ini file in bytes
	pub const fn size() -> usize {
		STARTUP_INI_SIZE
	}

	/// Gets the game opening mode
	pub fn opening_mode(&self) -> OpeningMode {
		self.opening_mode
	}

	/// Sets the game opening mode
	pub fn set_opening_mode(&mut self, mode: OpeningMode) {
		self.opening_mode = mode;
	}

	/// Gets the VGA mode
	pub fn vga_mode(&self) -> VgaMode {
		self.vga_mode
	}

	/// Sets the VGA mode
	pub fn set_vga_mode(&mut self, mode: VgaMode) {
		self.vga_mode = mode;
	}

	/// Gets the rendering mode
	pub fn render_mode(&self) -> RenderMode {
		self.render_mode
	}

	/// Sets the rendering mode
	pub fn set_render_mode(&mut self, mode: RenderMode) {
		self.render_mode = mode;
	}

	/// Gets the window rectangle (left, top, right, bottom)
	pub fn window_rect(&self) -> [u32; 4] {
		self.window_rect
	}

	/// Sets the window rectangle (left, top, right, bottom)
	pub fn set_window_rect(&mut self, rect: [u32; 4]) {
		self.window_rect = rect;
	}

	/// Gets a reference to the reserved bytes
	pub fn reserved(&self) -> &[u8; 2] {
		&self.reserved
	}
}

impl std::fmt::Display for StartupIni {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "Startup INI:")?;
		writeln!(f, "  Opening Mode: {}", self.opening_mode)?;
		writeln!(f, "  VGA Mode: {}", self.vga_mode)?;
		writeln!(f, "  Render Mode: {}", self.render_mode)?;
		writeln!(
			f,
			"  Window Rect: [{}, {}, {}, {}]",
			self.window_rect[0], self.window_rect[1], self.window_rect[2], self.window_rect[3]
		)?;
		Ok(())
	}
}

impl TryFrom<&[u8]> for StartupIni {
	type Error = StartupIniError;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<Vec<u8>> for StartupIni {
	type Error = StartupIniError;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&Vec<u8>> for StartupIni {
	type Error = StartupIniError;

	fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl TryFrom<[u8; STARTUP_INI_SIZE]> for StartupIni {
	type Error = StartupIniError;

	fn try_from(value: [u8; STARTUP_INI_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(&value)
	}
}

impl TryFrom<&[u8; STARTUP_INI_SIZE]> for StartupIni {
	type Error = StartupIniError;

	fn try_from(value: &[u8; STARTUP_INI_SIZE]) -> Result<Self, Self::Error> {
		Self::from_bytes(value)
	}
}

impl From<StartupIni> for [u8; STARTUP_INI_SIZE] {
	fn from(ini: StartupIni) -> Self {
		ini.to_bytes()
	}
}

impl From<&StartupIni> for [u8; STARTUP_INI_SIZE] {
	fn from(ini: &StartupIni) -> Self {
		ini.to_bytes()
	}
}

impl From<StartupIni> for Vec<u8> {
	fn from(ini: StartupIni) -> Self {
		ini.to_bytes().to_vec()
	}
}

impl From<&StartupIni> for Vec<u8> {
	fn from(ini: &StartupIni) -> Self {
		ini.to_bytes().to_vec()
	}
}
