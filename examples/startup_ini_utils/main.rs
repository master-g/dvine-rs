//! Startup.ini Configuration Management Utility
//!
//! A command-line tool for managing, validating, and modifying startup.ini configuration files.
//!
//! # Features
//!
//! - **show**: Display current configuration in a human-readable format
//! - **validate**: Validate configuration file integrity
//! - **set**: Modify configuration values (opening mode, VGA mode, render mode, window rect)
//! - **export**: Export configuration to JSON format
//! - **import**: Import configuration from JSON format
//! - **reset**: Reset configuration to default values
//!
//! # Configuration Fields
//!
//! - **Opening Mode**: Controls game opening behavior (Normal, Loop, Skip)
//! - **VGA Mode**: Display compatibility mode (Default, VGA Compatible)
//! - **Render Mode**: Rendering settings (VSYNC ON, VSYNC OFF)
//! - **Window Rect**: Dialog window position [left, top, right, bottom]
//!
//! # Usage
//!
//! ```bash
//! # Show current configuration
//! cargo run --example startup_ini_utils -- show startup.ini
//!
//! # Validate configuration
//! cargo run --example startup_ini_utils -- validate startup.ini
//!
//! # Modify settings
//! cargo run --example startup_ini_utils -- set startup.ini --opening-mode skip
//! cargo run --example startup_ini_utils -- set startup.ini --render-mode vsync-off
//!
//! # Export to JSON
//! cargo run --example startup_ini_utils -- export startup.ini config.json
//!
//! # Import from JSON
//! cargo run --example startup_ini_utils -- import config.json startup.ini
//!
//! # Reset to defaults
//! cargo run --example startup_ini_utils -- reset startup.ini
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use dvine_rs::prelude::file::{StartupIni, StartupOpeningMode, StartupRenderMode, StartupVgaMode};
use inquire::{Confirm, Select, Text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "startup_ini_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "Startup.ini configuration utility - manage and validate startup.ini files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Display current configuration
	Show {
		/// Input startup.ini file path
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Show raw hex dump
		#[arg(short = 'x', long)]
		hex: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Validate configuration file
	Validate {
		/// Input startup.ini file path
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Modify configuration values
	Set {
		/// Input startup.ini file path
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Output file path (optional, defaults to overwriting input)
		#[arg(short, long, value_name = "OUTPUT")]
		output: Option<PathBuf>,

		/// Opening mode
		#[arg(long, value_name = "MODE")]
		opening_mode: Option<OpeningModeArg>,

		/// VGA mode
		#[arg(long, value_name = "MODE")]
		vga_mode: Option<VgaModeArg>,

		/// Render mode
		#[arg(long, value_name = "MODE")]
		render_mode: Option<RenderModeArg>,

		/// Window rectangle [left,top,right,bottom]
		/// Note: This is for the configuration DIALOG, not the game window.
		/// Standard dialog size is 366×196. Only modify left/top position.
		#[arg(long, value_name = "RECT", num_args = 4)]
		window_rect: Option<Vec<u32>>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Export configuration to JSON
	Export {
		/// Input startup.ini file path
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Output JSON file path
		#[arg(value_name = "OUTPUT")]
		output: PathBuf,

		/// Pretty print JSON
		#[arg(short, long)]
		pretty: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Import configuration from JSON
	Import {
		/// Input JSON file path
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Output startup.ini file path
		#[arg(value_name = "OUTPUT")]
		output: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Reset configuration to default values
	Reset {
		/// Output startup.ini file path
		#[arg(value_name = "OUTPUT")]
		output: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Create a new configuration interactively
	Create {
		/// Output startup.ini file path
		#[arg(value_name = "OUTPUT")]
		output: PathBuf,
	},
}

/// Opening mode argument
#[derive(Debug, Clone, Copy, ValueEnum)]
enum OpeningModeArg {
	/// Normal opening mode
	Normal,
	/// Loop startup
	Loop,
	/// Skip opening
	Skip,
}

impl From<OpeningModeArg> for StartupOpeningMode {
	fn from(arg: OpeningModeArg) -> Self {
		match arg {
			OpeningModeArg::Normal => StartupOpeningMode::Normal,
			OpeningModeArg::Loop => StartupOpeningMode::Loop,
			OpeningModeArg::Skip => StartupOpeningMode::Skip,
		}
	}
}

/// VGA mode argument
#[derive(Debug, Clone, Copy, ValueEnum)]
enum VgaModeArg {
	/// Default resolution
	Default,
	/// VGA compatible mode
	VgaCompatible,
}

impl From<VgaModeArg> for StartupVgaMode {
	fn from(arg: VgaModeArg) -> Self {
		match arg {
			VgaModeArg::Default => StartupVgaMode::Default,
			VgaModeArg::VgaCompatible => StartupVgaMode::VgaCompatible,
		}
	}
}

/// Render mode argument
#[derive(Debug, Clone, Copy, ValueEnum)]
enum RenderModeArg {
	/// VSYNC ON
	VsyncOn,
	/// VSYNC OFF
	VsyncOff,
}

impl From<RenderModeArg> for StartupRenderMode {
	fn from(arg: RenderModeArg) -> Self {
		match arg {
			RenderModeArg::VsyncOn => StartupRenderMode::VsyncOn,
			RenderModeArg::VsyncOff => StartupRenderMode::VsyncOff,
		}
	}
}

/// JSON serialization structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StartupIniJson {
	/// Opening mode
	opening_mode: String,
	/// VGA mode
	vga_mode: String,
	/// Render mode
	render_mode: String,
	/// Window rectangle [left, top, right, bottom]
	window_rect: [u32; 4],
}

impl From<&StartupIni> for StartupIniJson {
	fn from(ini: &StartupIni) -> Self {
		Self {
			opening_mode: match ini.opening_mode() {
				StartupOpeningMode::Normal => "Normal".to_string(),
				StartupOpeningMode::Loop => "Loop".to_string(),
				StartupOpeningMode::Skip => "Skip".to_string(),
			},
			vga_mode: match ini.vga_mode() {
				StartupVgaMode::Default => "Default".to_string(),
				StartupVgaMode::VgaCompatible => "VGA Compatible".to_string(),
			},
			render_mode: match ini.render_mode() {
				StartupRenderMode::VsyncOn => "VSYNC ON".to_string(),
				StartupRenderMode::VsyncOff => "VSYNC OFF".to_string(),
			},
			window_rect: ini.window_rect(),
		}
	}
}

impl TryFrom<&StartupIniJson> for StartupIni {
	type Error = Box<dyn std::error::Error>;

	fn try_from(json: &StartupIniJson) -> Result<Self, Self::Error> {
		let opening_mode = match json.opening_mode.as_str() {
			"Normal" => StartupOpeningMode::Normal,
			"Loop" => StartupOpeningMode::Loop,
			"Skip" => StartupOpeningMode::Skip,
			_ => return Err(format!("Invalid opening mode: {}", json.opening_mode).into()),
		};

		let vga_mode = match json.vga_mode.as_str() {
			"Default" => StartupVgaMode::Default,
			"VGA Compatible" => StartupVgaMode::VgaCompatible,
			_ => return Err(format!("Invalid VGA mode: {}", json.vga_mode).into()),
		};

		let render_mode = match json.render_mode.as_str() {
			"VSYNC ON" => StartupRenderMode::VsyncOn,
			"VSYNC OFF" => StartupRenderMode::VsyncOff,
			_ => return Err(format!("Invalid render mode: {}", json.render_mode).into()),
		};

		let mut ini = StartupIni::default();
		ini.set_opening_mode(opening_mode);
		ini.set_vga_mode(vga_mode);
		ini.set_render_mode(render_mode);
		ini.set_window_rect(json.window_rect);

		Ok(ini)
	}
}

/// Print hex dump of data
fn print_hex_dump(data: &[u8]) {
	println!("\nHex dump ({} bytes):", data.len());
	for (i, chunk) in data.chunks(8).enumerate() {
		print!("  {:04X}: ", i * 8);

		// Hex bytes
		for byte in chunk {
			print!("{:02X} ", byte);
		}

		// Padding
		for _ in chunk.len()..8 {
			print!("   ");
		}

		// ASCII representation
		print!(" |");
		for byte in chunk {
			if *byte >= 32 && *byte <= 126 {
				print!("{}", *byte as char);
			} else {
				print!(".");
			}
		}
		println!("|");
	}
}

/// Handle show command
fn handle_show(input: PathBuf, hex: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("📄 Reading configuration");
		println!("   Input: {}", input.display());
	}

	// Load configuration
	let ini = StartupIni::open(&input)?;

	if verbose {
		println!("   ✓ Loaded successfully\n");
	}

	// Display configuration
	println!("╔════════════════════════════════════════════╗");
	println!("║         Startup.ini Configuration          ║");
	println!("╠════════════════════════════════════════════╣");
	println!("║ Opening Mode:  {:<27} ║", format!("{}", ini.opening_mode()));
	println!("║ VGA Mode:      {:<27} ║", format!("{}", ini.vga_mode()));
	println!("║ Render Mode:   {:<27} ║", format!("{}", ini.render_mode()));
	println!(
		"║ Window Rect:   [{:>4}, {:>4}, {:>4}, {:>4}]    ║",
		ini.window_rect()[0],
		ini.window_rect()[1],
		ini.window_rect()[2],
		ini.window_rect()[3]
	);
	println!("╚════════════════════════════════════════════╝");

	// Show hex dump if requested
	if hex {
		let bytes = ini.to_bytes();
		print_hex_dump(&bytes);
	}

	Ok(())
}

/// Handle validate command
fn handle_validate(input: PathBuf, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔍 Validating configuration");
		println!("   Input: {}", input.display());
	}

	// Check file size
	let metadata = fs::metadata(&input)?;
	let file_size = metadata.len();

	if verbose {
		println!("\n📊 File Information:");
		println!("   Size: {} bytes", file_size);
	}

	if file_size != StartupIni::size() as u64 {
		println!(
			"\n❌ Validation FAILED: Invalid file size\n   Expected: {} bytes\n   Found: {} bytes",
			StartupIni::size(),
			file_size
		);
		return Err("Invalid file size".into());
	}

	// Try to parse
	let ini = match StartupIni::open(&input) {
		Ok(ini) => ini,
		Err(e) => {
			println!("\n❌ Validation FAILED: {}", e);
			return Err(e.into());
		}
	};

	if verbose {
		println!("\n✓ File Format:");
		println!("   Opening Mode: {} (valid)", ini.opening_mode());
		println!("   VGA Mode: {} (valid)", ini.vga_mode());
		println!("   Render Mode: {} (valid)", ini.render_mode());
		println!(
			"   Window Rect: [{}, {}, {}, {}]",
			ini.window_rect()[0],
			ini.window_rect()[1],
			ini.window_rect()[2],
			ini.window_rect()[3]
		);
	}

	println!("\n✅ Validation PASSED: Configuration is valid");

	Ok(())
}

/// Handle set command
fn handle_set(
	input: PathBuf,
	output: Option<PathBuf>,
	opening_mode: Option<OpeningModeArg>,
	vga_mode: Option<VgaModeArg>,
	render_mode: Option<RenderModeArg>,
	window_rect: Option<Vec<u32>>,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let output = output.unwrap_or_else(|| input.clone());

	if verbose {
		println!("🔧 Modifying configuration");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
	}

	// Load current configuration
	let mut ini = StartupIni::open(&input)?;

	if verbose {
		println!("\n📖 Current values:");
		println!("   Opening Mode: {}", ini.opening_mode());
		println!("   VGA Mode: {}", ini.vga_mode());
		println!("   Render Mode: {}", ini.render_mode());
		println!(
			"   Window Rect: [{}, {}, {}, {}]",
			ini.window_rect()[0],
			ini.window_rect()[1],
			ini.window_rect()[2],
			ini.window_rect()[3]
		);
	}

	let mut changes = Vec::new();

	// Apply changes
	if let Some(mode) = opening_mode {
		let new_mode: StartupOpeningMode = mode.into();
		ini.set_opening_mode(new_mode);
		changes.push(format!("Opening Mode → {}", new_mode));
	}

	if let Some(mode) = vga_mode {
		let new_mode: StartupVgaMode = mode.into();
		ini.set_vga_mode(new_mode);
		changes.push(format!("VGA Mode → {}", new_mode));
	}

	if let Some(mode) = render_mode {
		let new_mode: StartupRenderMode = mode.into();
		ini.set_render_mode(new_mode);
		changes.push(format!("Render Mode → {}", new_mode));
	}

	if let Some(rect) = window_rect {
		if rect.len() != 4 {
			return Err("Window rect must have exactly 4 values".into());
		}
		let rect_array = [rect[0], rect[1], rect[2], rect[3]];
		ini.set_window_rect(rect_array);
		changes.push(format!("Window Rect → [{}, {}, {}, {}]", rect[0], rect[1], rect[2], rect[3]));
	}

	if changes.is_empty() {
		println!("\n⚠ No changes specified");
		println!("   Use --opening-mode, --vga-mode, --render-mode, or --window-rect");
		return Ok(());
	}

	// Save modified configuration
	let bytes = ini.to_bytes();
	fs::write(&output, bytes)?;

	if verbose {
		println!("\n✓ Applied changes:");
		for change in &changes {
			println!("   • {}", change);
		}
		println!("\n💾 Saved to {}", output.display());
		println!("\n✅ Configuration updated successfully!");
	} else {
		println!("✓ Updated {} field(s) -> {}", changes.len(), output.display());
		for change in &changes {
			println!("  • {}", change);
		}
	}

	Ok(())
}

/// Handle export command
fn handle_export(
	input: PathBuf,
	output: PathBuf,
	pretty: bool,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("📤 Exporting to JSON");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
	}

	// Load configuration
	let ini = StartupIni::open(&input)?;

	if verbose {
		println!("\n📖 Configuration loaded:");
		println!("   Opening Mode: {}", ini.opening_mode());
		println!("   VGA Mode: {}", ini.vga_mode());
		println!("   Render Mode: {}", ini.render_mode());
		println!(
			"   Window Rect: [{}, {}, {}, {}]",
			ini.window_rect()[0],
			ini.window_rect()[1],
			ini.window_rect()[2],
			ini.window_rect()[3]
		);
	}

	// Convert to JSON
	let json_data = StartupIniJson::from(&ini);

	let json_string = if pretty {
		serde_json::to_string_pretty(&json_data)?
	} else {
		serde_json::to_string(&json_data)?
	};

	// Save JSON
	fs::write(&output, json_string)?;

	if verbose {
		println!("\n💾 Saved JSON to {}", output.display());
		println!("\n✅ Export completed successfully!");
	} else {
		println!("✓ Exported {} -> {}", input.display(), output.display());
	}

	Ok(())
}

/// Handle import command
fn handle_import(
	input: PathBuf,
	output: PathBuf,
	verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("📥 Importing from JSON");
		println!("   Input:  {}", input.display());
		println!("   Output: {}", output.display());
	}

	// Load JSON
	let json_string = fs::read_to_string(&input)?;
	let json_data: StartupIniJson = serde_json::from_str(&json_string)?;

	if verbose {
		println!("\n📖 JSON loaded:");
		println!("   Opening Mode: {}", json_data.opening_mode);
		println!("   VGA Mode: {}", json_data.vga_mode);
		println!("   Render Mode: {}", json_data.render_mode);
		println!(
			"   Window Rect: [{}, {}, {}, {}]",
			json_data.window_rect[0],
			json_data.window_rect[1],
			json_data.window_rect[2],
			json_data.window_rect[3]
		);
	}

	// Convert to StartupIni
	let ini = StartupIni::try_from(&json_data)?;

	// Save binary file
	let bytes = ini.to_bytes();
	fs::write(&output, bytes)?;

	if verbose {
		println!("\n💾 Saved to {}", output.display());
		println!("   Size: {} bytes", bytes.len());
		println!("\n✅ Import completed successfully!");
	} else {
		println!("✓ Imported {} -> {}", input.display(), output.display());
	}

	Ok(())
}

/// Handle reset command
fn handle_reset(output: PathBuf, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
	if verbose {
		println!("🔄 Resetting to default configuration");
		println!("   Output: {}", output.display());
	}

	// Create default configuration
	let ini = StartupIni::default();

	if verbose {
		println!("\n📋 Default values:");
		println!("   Opening Mode: {}", ini.opening_mode());
		println!("   VGA Mode: {}", ini.vga_mode());
		println!("   Render Mode: {}", ini.render_mode());
		println!(
			"   Window Rect: [{}, {}, {}, {}]",
			ini.window_rect()[0],
			ini.window_rect()[1],
			ini.window_rect()[2],
			ini.window_rect()[3]
		);
	}

	// Save default configuration
	let bytes = ini.to_bytes();
	fs::write(&output, bytes)?;

	if verbose {
		println!("\n💾 Saved to {}", output.display());
		println!("   Size: {} bytes", bytes.len());
		println!("\n✅ Reset completed successfully!");
	} else {
		println!("✓ Reset to defaults -> {}", output.display());
	}

	Ok(())
}

/// Handle create command (interactive)
fn handle_create(output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
	println!("🎨 Interactive Configuration Creator");
	println!("   Creating: {}\n", output.display());

	// Opening Mode
	let opening_mode_options = vec!["Normal", "Loop", "Skip"];
	let opening_mode_choice = Select::new("Select opening mode:", opening_mode_options)
		.with_help_message("Controls game opening behavior")
		.prompt()?;

	let opening_mode = match opening_mode_choice {
		"Loop" => StartupOpeningMode::Loop,
		"Skip" => StartupOpeningMode::Skip,
		_ => StartupOpeningMode::Normal, // "Normal" or any other value
	};

	// VGA Mode
	let vga_mode_options = vec!["Default", "VGA Compatible"];
	let vga_mode_choice = Select::new("Select VGA mode:", vga_mode_options)
		.with_help_message("Display compatibility mode")
		.prompt()?;

	let vga_mode = match vga_mode_choice {
		"VGA Compatible" => StartupVgaMode::VgaCompatible,
		_ => StartupVgaMode::Default, // "Default" or any other value
	};

	// Render Mode
	let render_mode_options = vec!["VSYNC ON", "VSYNC OFF"];
	let render_mode_choice = Select::new("Select render mode:", render_mode_options)
		.with_help_message("Vertical sync setting")
		.prompt()?;

	let render_mode = match render_mode_choice {
		"VSYNC OFF" => StartupRenderMode::VsyncOff,
		_ => StartupRenderMode::VsyncOn, // "VSYNC ON" or any other value
	};

	// Window position (only top and left are customizable)
	// Note: This window rect is for the game's CONFIGURATION DIALOG, not the game window itself.
	// The dialog has a fixed size of 366×196 pixels.
	// Typical position from bin/startup.ini: left=1158, top=571
	// right and bottom are auto-calculated: right = left + 366, bottom = top + 196
	const DIALOG_WIDTH: u32 = 366;
	const DIALOG_HEIGHT: u32 = 196;

	let customize_position = Confirm::new("Customize dialog window position?")
		.with_default(false)
		.with_help_message("The configuration dialog position on screen (not the game window)")
		.prompt()?;

	let window_rect = if customize_position {
		let left_str = Text::new("Enter left position (X coordinate):")
			.with_default("1158")
			.with_help_message("Horizontal position of the dialog (default: 1158)")
			.prompt()?;

		let top_str = Text::new("Enter top position (Y coordinate):")
			.with_default("571")
			.with_help_message("Vertical position of the dialog (default: 571)")
			.prompt()?;

		let left: u32 = left_str.parse().unwrap_or(1158);
		let top: u32 = top_str.parse().unwrap_or(571);
		let right = left + DIALOG_WIDTH;
		let bottom = top + DIALOG_HEIGHT;

		[left, top, right, bottom]
	} else {
		// Use typical values from bin/startup.ini
		[1158, 571, 1524, 767]
	};

	// Create configuration
	let mut ini = StartupIni::default();
	ini.set_opening_mode(opening_mode);
	ini.set_vga_mode(vga_mode);
	ini.set_render_mode(render_mode);
	ini.set_window_rect(window_rect);

	// Display summary
	println!("\n📋 Configuration Summary:");
	println!("   Opening Mode: {}", opening_mode);
	println!("   VGA Mode: {}", vga_mode);
	println!("   Render Mode: {}", render_mode);
	println!(
		"   Dialog Position: [{}, {}, {}, {}]",
		window_rect[0], window_rect[1], window_rect[2], window_rect[3]
	);

	// Confirm before saving
	let confirm = Confirm::new("Save this configuration?").with_default(true).prompt()?;

	if !confirm {
		println!("\n⚠ Configuration creation cancelled");
		return Ok(());
	}

	// Save configuration
	let bytes = ini.to_bytes();
	fs::write(&output, bytes)?;

	println!("\n💾 Configuration saved to {}", output.display());
	println!("   Size: 24 bytes");
	println!("\n✅ Configuration created successfully!");

	Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	match cli.command {
		Commands::Show {
			input,
			hex,
			verbose,
		} => handle_show(input, hex, verbose),

		Commands::Validate {
			input,
			verbose,
		} => handle_validate(input, verbose),

		Commands::Set {
			input,
			output,
			opening_mode,
			vga_mode,
			render_mode,
			window_rect,
			verbose,
		} => handle_set(input, output, opening_mode, vga_mode, render_mode, window_rect, verbose),

		Commands::Export {
			input,
			output,
			pretty,
			verbose,
		} => handle_export(input, output, pretty, verbose),

		Commands::Import {
			input,
			output,
			verbose,
		} => handle_import(input, output, verbose),

		Commands::Reset {
			output,
			verbose,
		} => handle_reset(output, verbose),

		Commands::Create {
			output,
		} => handle_create(output),
	}
}
