//! FNT (Font) CLI Utility
//!
//! A command-line tool for inspecting, exporting, and rendering font files from FNT format.
//!
//! # Features
//!
//! - **info**: Display font file information (size, glyph count, encoding)
//! - **dump**: Export all glyphs to a PNG image grid
//! - **render**: Render UTF-8 text to PNG using the font
//! - **extract**: Extract specific glyphs by character code or text
//!
//! # Font Format
//!
//! FNT files use Shift-JIS encoding for Japanese text:
//! - Single-byte characters (0x00-0x7F): ASCII
//! - Single-byte characters (0xA1-0xDF): Half-width katakana
//! - Double-byte characters: Japanese hiragana, katakana, kanji
//!
//! # Usage Examples
//!
//! ```bash
//! # Display font information
//! cargo run --example fnt_utils -- info SYSTEM.FNT
//!
//! # Dump all glyphs to a grid image
//! cargo run --example fnt_utils -- dump SYSTEM.FNT -o system_glyphs.png
//!
//! # Render single line text
//! cargo run --example fnt_utils -- render SYSTEM.FNT "Hello World" -o hello.png
//!
//! # Render multi-line text from file
//! cargo run --example fnt_utils -- render SYSTEM.FNT -f text.txt -o output.png
//!
//! # Extract specific glyph by character
//! cargo run --example fnt_utils -- extract SYSTEM.FNT "A" -o glyph_a.png
//!
//! # Extract glyph by hex code
//! cargo run --example fnt_utils -- extract SYSTEM.FNT --code 0x82A0 -o glyph_hiragana.png
//! ```

use clap::{Parser, Subcommand};
use dvine_rs::prelude::file::{FntFile, Glyph, GlyphBitmap};
use image::{ImageBuffer, Rgb, RgbImage};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fnt_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "FNT font utility - inspect, export, and render font files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Display font file information
	Info {
		/// Input FNT file path
		#[arg(value_name = "INPUT_FNT")]
		input: PathBuf,

		/// Show detailed glyph information
		#[arg(short, long)]
		detailed: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Dump all glyphs to a PNG grid image
	Dump {
		/// Input FNT file path
		#[arg(value_name = "INPUT_FNT")]
		input: PathBuf,

		/// Output PNG file path (defaults to `input_glyphs.png`)
		#[arg(short, long, value_name = "OUTPUT_PNG")]
		output: Option<PathBuf>,

		/// Grid size (auto-calculated if not specified)
		#[arg(short, long, value_name = "GRID_SIZE")]
		grid_size: Option<usize>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Render text to PNG image
	Render {
		/// Input FNT file path
		#[arg(value_name = "INPUT_FNT")]
		input: PathBuf,

		/// Text to render (UTF-8 string)
		#[arg(value_name = "TEXT")]
		text: Option<String>,

		/// Read text from file (one line per image line)
		#[arg(short, long, value_name = "TEXT_FILE")]
		file: Option<PathBuf>,

		/// Output PNG file path (defaults to `text_render.png`)
		#[arg(short, long, value_name = "OUTPUT_PNG")]
		output: Option<PathBuf>,

		/// Character spacing in pixels
		#[arg(long, default_value = "1")]
		char_spacing: u32,

		/// Line spacing in pixels
		#[arg(long, default_value = "2")]
		line_spacing: u32,

		/// Padding around text in pixels
		#[arg(long, default_value = "4")]
		padding: u32,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Extract specific glyph(s) to PNG
	Extract {
		/// Input FNT file path
		#[arg(value_name = "INPUT_FNT")]
		input: PathBuf,

		/// Character(s) to extract
		#[arg(value_name = "TEXT")]
		text: Option<String>,

		/// Character code in hex (e.g., 0x82A0)
		#[arg(long, value_name = "HEX_CODE")]
		code: Option<String>,

		/// Output PNG file path (defaults to `glyph_XXXX.png`)
		#[arg(short, long, value_name = "OUTPUT_PNG")]
		output: Option<PathBuf>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},
}

/// Encodes UTF-8 text to Shift-JIS bytes
fn encode_text_to_shift_jis(text: &str) -> Result<Vec<u8>, String> {
	let (encoded, _encoding, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
	if had_errors {
		Err("Failed to encode text to Shift-JIS: some characters are not supported".to_string())
	} else {
		Ok(encoded.to_vec())
	}
}

/// Handles the 'info' command
fn handle_info(input: &PathBuf, detailed: bool, verbose: bool) -> Result<(), String> {
	if verbose {
		println!("Loading font file: {}", input.display());
	}

	let font = FntFile::open(input).map_err(|e| format!("Failed to load font file: {}", e))?;

	println!("\n=== Font Information ===");
	println!("File: {}", input.display());
	println!("Font Size: {}", font.font_size());
	println!("Bytes per Glyph: {}", font.bytes_per_glyph());
	println!("Number of Glyphs: {}", font.num_of_glyphs());
	println!("Encoding: Shift-JIS");

	if detailed {
		println!("\n=== Sample Glyphs ===");
		let mut count = 0;
		for glyph in font.iter().take(10) {
			println!("\nGlyph 0x{:04X}:", glyph.code());
			let bitmap: GlyphBitmap = (&glyph).into();
			let art = bitmap.to_ascii_art();
			println!("{}", art);
			count += 1;
		}
		println!("\n(Showing first {} glyphs)", count);
	}

	Ok(())
}

/// Handles the 'dump' command
fn handle_dump(
	input: &PathBuf,
	output: Option<PathBuf>,
	grid_size: Option<usize>,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading font file: {}", input.display());
	}

	let font = FntFile::open(input).map_err(|e| format!("Failed to load font file: {}", e))?;

	// Determine output path
	let output_path = output.unwrap_or_else(|| {
		let mut path = input.clone();
		path.set_extension("png");
		let filename = path.file_stem().unwrap().to_string_lossy().to_string();
		PathBuf::from(format!("{}_glyphs.png", filename))
	});

	if verbose {
		println!("Dumping glyphs to: {}", output_path.display());
	}

	// Collect all glyphs
	let glyphs: Vec<Glyph> = font.iter().collect();
	let num_glyphs = glyphs.len();

	if num_glyphs == 0 {
		return Err("No glyphs found in font".to_string());
	}

	// Calculate grid dimensions
	let grid = grid_size.unwrap_or_else(|| (num_glyphs as f64).sqrt().ceil() as usize);

	if verbose {
		println!("Total glyphs: {}", num_glyphs);
		println!("Grid layout: {}x{} cells", grid, grid);
	}

	// Get glyph size and separator width
	let glyph_size = font.font_size() as u32;
	let separator_width = 1u32;

	// Calculate image dimensions
	let img_width = grid as u32 * (glyph_size + separator_width) + separator_width;
	let img_height = grid as u32 * (glyph_size + separator_width) + separator_width;

	if verbose {
		println!("Image dimensions: {}x{} pixels", img_width, img_height);
	}

	// Create image with white background
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, white);

	// Draw green separators (1px grid lines)
	let green = Rgb([0, 255, 0]);

	// Vertical separators
	for col in 0..=grid {
		let x = col as u32 * (glyph_size + separator_width);
		for y in 0..img_height {
			img.put_pixel(x, y, green);
		}
	}

	// Horizontal separators
	for row in 0..=grid {
		let y = row as u32 * (glyph_size + separator_width);
		for x in 0..img_width {
			img.put_pixel(x, y, green);
		}
	}

	// Draw each glyph in black
	let black = Rgb([0, 0, 0]);

	for (idx, glyph) in glyphs.iter().enumerate() {
		let grid_row = idx / grid;
		let grid_col = idx % grid;

		// Calculate starting position for this glyph
		let start_x = grid_col as u32 * (glyph_size + separator_width) + separator_width;
		let start_y = grid_row as u32 * (glyph_size + separator_width) + separator_width;

		// Convert glyph to bitmap and draw
		let bitmap: GlyphBitmap = glyph.into();

		for (y_offset, line) in bitmap.line_iterator().enumerate() {
			for (x_offset, &pixel) in line.iter().enumerate() {
				if pixel {
					img.put_pixel(start_x + x_offset as u32, start_y + y_offset as u32, black);
				}
			}
		}
	}

	// Save the image
	img.save(&output_path).map_err(|e| format!("Failed to save image: {}", e))?;

	println!("✓ Image saved: {}", output_path.display());

	Ok(())
}

/// Handles the 'render' command
#[allow(clippy::too_many_arguments)]
fn handle_render(
	input: &PathBuf,
	text: Option<String>,
	file: Option<PathBuf>,
	output: Option<PathBuf>,
	char_spacing: u32,
	line_spacing: u32,
	padding: u32,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading font file: {}", input.display());
	}

	let font = FntFile::open(input).map_err(|e| format!("Failed to load font file: {}", e))?;

	// Get text to render
	let lines: Vec<String> = if let Some(text_file) = file {
		if verbose {
			println!("Reading text from: {}", text_file.display());
		}
		fs::read_to_string(&text_file)
			.map_err(|e| format!("Failed to read text file: {}", e))?
			.lines()
			.map(str::to_string)
			.collect()
	} else if let Some(text_str) = text {
		vec![text_str]
	} else {
		return Err("Either TEXT or --file must be provided".to_string());
	};

	if lines.is_empty() {
		return Err("No text to render".to_string());
	}

	// Determine output path
	let output_path = output.unwrap_or_else(|| PathBuf::from("text_render.png"));

	if verbose {
		println!("Rendering {} line(s) to: {}", lines.len(), output_path.display());
	}

	// Encode all lines and collect glyphs
	let mut all_line_glyphs = Vec::new();
	let mut max_line_width = 0usize;

	for (idx, line) in lines.iter().enumerate() {
		if verbose {
			println!("Line {}: \"{}\"", idx + 1, line);
		}

		// Encode UTF-8 text to Shift-JIS bytes
		let encoded = encode_text_to_shift_jis(line)?;

		// Get all glyphs from the byte stream
		let glyphs = font.lookup_from_stream(&encoded);

		if verbose {
			println!("  Found {} glyphs", glyphs.len());
		}

		max_line_width = max_line_width.max(glyphs.len());
		all_line_glyphs.push(glyphs);
	}

	if all_line_glyphs.is_empty() || all_line_glyphs.iter().all(std::vec::Vec::is_empty) {
		return Err("No glyphs found for any line".to_string());
	}

	// Get glyph dimensions
	let glyph_size = font.font_size() as u32;

	// Calculate image dimensions
	let img_width = if max_line_width > 0 {
		padding * 2 + max_line_width as u32 * (glyph_size + char_spacing) - char_spacing
	} else {
		padding * 2 + glyph_size
	};
	let img_height =
		padding * 2 + all_line_glyphs.len() as u32 * (glyph_size + line_spacing) - line_spacing;

	if verbose {
		println!("Image dimensions: {}x{} pixels", img_width, img_height);
	}

	// Create image with white background
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(img_width, img_height, white);

	// Draw each line
	let black = Rgb([0, 0, 0]);
	let mut current_y = padding;

	for line_glyphs in all_line_glyphs.iter() {
		let mut current_x = padding;

		for glyph in line_glyphs.iter() {
			// Convert glyph to bitmap
			let bitmap: GlyphBitmap = glyph.into();

			// Draw glyph using line iterator
			for (y_offset, line) in bitmap.line_iterator().enumerate() {
				for (x_offset, &pixel) in line.iter().enumerate() {
					if pixel {
						let px = current_x + x_offset as u32;
						let py = current_y + y_offset as u32;
						img.put_pixel(px, py, black);
					}
				}
			}

			// Move to next character position
			current_x += glyph_size + char_spacing;
		}

		// Move to next line
		current_y += glyph_size + line_spacing;
	}

	// Save the image
	img.save(&output_path).map_err(|e| format!("Failed to save image: {}", e))?;

	println!("✓ Image saved: {}", output_path.display());

	Ok(())
}

/// Handles the 'extract' command
fn handle_extract(
	input: &PathBuf,
	text: Option<String>,
	code: Option<String>,
	output: Option<PathBuf>,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading font file: {}", input.display());
	}

	let font = FntFile::open(input).map_err(|e| format!("Failed to load font file: {}", e))?;

	// Get glyph to extract
	let glyph = if let Some(hex_code) = code {
		// Parse hex code
		let code_value = if hex_code.starts_with("0x") || hex_code.starts_with("0X") {
			u16::from_str_radix(&hex_code[2..], 16)
		} else {
			u16::from_str_radix(&hex_code, 16)
		}
		.map_err(|e| format!("Invalid hex code '{}': {}", hex_code, e))?;

		if verbose {
			println!("Looking up glyph by code: 0x{:04X}", code_value);
		}

		font.lookup(code_value)
			.ok_or_else(|| format!("Glyph not found for code 0x{:04X}", code_value))?
	} else if let Some(text_str) = text {
		if text_str.is_empty() {
			return Err("Text cannot be empty".to_string());
		}

		// Get first character
		let first_char = text_str.chars().next().unwrap();
		if verbose {
			println!("Extracting glyph for character: '{}'", first_char);
		}

		// Encode to Shift-JIS
		let encoded = encode_text_to_shift_jis(&first_char.to_string())?;

		// Lookup glyph
		let glyphs = font.lookup_from_stream(&encoded);
		if glyphs.is_empty() {
			return Err(format!("Glyph not found for character '{}'", first_char));
		}

		glyphs.into_iter().next().unwrap()
	} else {
		return Err("Either TEXT or --code must be provided".to_string());
	};

	// Determine output path
	let output_path =
		output.unwrap_or_else(|| PathBuf::from(format!("glyph_{:04X}.png", glyph.code())));

	if verbose {
		println!("Extracting glyph 0x{:04X} to: {}", glyph.code(), output_path.display());
	}

	// Create bitmap
	let bitmap: GlyphBitmap = (&glyph).into();
	let glyph_size = font.font_size() as u32;

	// Create image with white background
	let white = Rgb([255, 255, 255]);
	let mut img: RgbImage = ImageBuffer::from_pixel(glyph_size, glyph_size, white);

	// Draw glyph in black
	let black = Rgb([0, 0, 0]);

	for (y_offset, line) in bitmap.line_iterator().enumerate() {
		for (x_offset, &pixel) in line.iter().enumerate() {
			if pixel {
				img.put_pixel(x_offset as u32, y_offset as u32, black);
			}
		}
	}

	// Save the image
	img.save(&output_path).map_err(|e| format!("Failed to save image: {}", e))?;

	println!("✓ Image saved: {}", output_path.display());

	// Print ASCII art if verbose
	if verbose {
		println!("\nASCII Art:");
		println!("{}", bitmap.to_ascii_art());
	}

	Ok(())
}

fn main() {
	let cli = Cli::parse();

	let result = match cli.command {
		Commands::Info {
			input,
			detailed,
			verbose,
		} => handle_info(&input, detailed, verbose),
		Commands::Dump {
			input,
			output,
			grid_size,
			verbose,
		} => handle_dump(&input, output, grid_size, verbose),
		Commands::Render {
			input,
			text,
			file,
			output,
			char_spacing,
			line_spacing,
			padding,
			verbose,
		} => handle_render(&input, text, file, output, char_spacing, line_spacing, padding, verbose),
		Commands::Extract {
			input,
			text,
			code,
			output,
			verbose,
		} => handle_extract(&input, text, code, output, verbose),
	};

	if let Err(e) = result {
		eprintln!("Error: {}", e);
		std::process::exit(1);
	}
}
