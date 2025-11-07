//! DSK/PFT (Disk/Pack File Table) CLI Utility
//!
//! A command-line tool for managing DSK container files and their PFT metadata.
//!
//! # Features
//!
//! - **info**: Display DSK/PFT information and statistics
//! - **list**: List all files in the container with detailed information
//! - **extract**: Extract files from DSK container (single, multiple, or all)
//! - **pack**: Create DSK/PFT from a directory or file list
//! - **verify**: Validate DSK/PFT integrity and consistency
//!
//! # File Format
//!
//! DSK files are block-based containers (2048 bytes per block) that store multiple files.
//! PFT files contain the metadata (file names, block indices, actual sizes).
//!
//! # Usage Examples
//!
//! ```bash
//! # Display information about a DSK/PFT pair
//! cargo run --example dsk_utils -- info bin/DATA
//!
//! # List all files in detailed table format
//! cargo run --example dsk_utils -- list bin/DATA --format table
//!
//! # Extract a specific file by name
//! cargo run --example dsk_utils -- extract bin/DATA FILE1 -o output/
//!
//! # Extract all files
//! cargo run --example dsk_utils -- extract bin/DATA --all -o output/
//!
//! # Pack files into DSK/PFT
//! cargo run --example dsk_utils -- pack input/ -o bin/DATA
//!
//! # Verify integrity
//! cargo run --example dsk_utils -- verify bin/DATA
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use dvine_rs::prelude::file::{dsk, pft};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "dsk_utils")]
#[command(author = "dvine-rs project")]
#[command(version = "1.0")]
#[command(about = "DSK/PFT container utility - inspect, extract, pack, and verify", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Display DSK/PFT information and statistics
	Info {
		/// Input path (directory with NAME.DSK/NAME.PFT or direct .DSK/.PFT file)
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Base name (e.g., "DATA" for DATA.DSK/DATA.PFT)
		#[arg(short, long, value_name = "NAME")]
		name: Option<String>,

		/// Show detailed information
		#[arg(short, long)]
		detailed: bool,

		/// Show block usage map
		#[arg(short, long)]
		blocks: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// List all files in the container
	List {
		/// Input path (directory with NAME.DSK/NAME.PFT or direct .DSK/.PFT file)
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Base name (e.g., "DATA" for DATA.DSK/DATA.PFT)
		#[arg(short, long, value_name = "NAME")]
		name: Option<String>,

		/// Sort by field
		#[arg(short, long, value_enum, default_value = "index")]
		sort: SortOrder,

		/// Output format
		#[arg(short, long, value_enum, default_value = "table")]
		format: OutputFormat,

		/// Filter by name pattern (case-insensitive)
		#[arg(short = 'p', long, value_name = "PATTERN")]
		filter: Option<String>,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Extract files from DSK container
	Extract {
		/// Input path (directory with NAME.DSK/NAME.PFT or direct .DSK/.PFT file)
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Base name (e.g., "DATA" for DATA.DSK/DATA.PFT)
		#[arg(short, long, value_name = "NAME")]
		name: Option<String>,

		/// Files to extract (names or indices)
		#[arg(value_name = "FILES")]
		files: Vec<String>,

		/// Extract all files
		#[arg(short, long)]
		all: bool,

		/// Output directory
		#[arg(short, long, value_name = "OUTPUT_DIR", default_value = "extracted")]
		output: PathBuf,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Pack files into DSK/PFT container
	Pack {
		/// Input directory or metadata JSON file
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Output path (directory or base name)
		#[arg(short, long, value_name = "OUTPUT")]
		output: PathBuf,

		/// Base name for output files (e.g., "DATA" for DATA.DSK/DATA.PFT)
		#[arg(short, long, value_name = "NAME")]
		name: Option<String>,

		/// Optimize block allocation
		#[arg(long)]
		optimize: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Verify DSK/PFT integrity
	Verify {
		/// Input path (directory with NAME.DSK/NAME.PFT or direct .DSK/.PFT file)
		#[arg(value_name = "INPUT")]
		input: PathBuf,

		/// Base name (e.g., "DATA" for DATA.DSK/DATA.PFT)
		#[arg(short, long, value_name = "NAME")]
		name: Option<String>,

		/// Strict validation mode
		#[arg(short, long)]
		strict: bool,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SortOrder {
	/// Sort by entry index
	Index,
	/// Sort by file name
	Name,
	/// Sort by file size
	Size,
	/// Sort by block count
	Blocks,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
	/// Table format
	Table,
	/// JSON format
	Json,
	/// CSV format
	Csv,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackMetadata {
	name: String,
	files: Vec<FileMetadata>,
	total_blocks: usize,
	block_size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileMetadata {
	name: String,
	path: PathBuf,
	size: u32,
	#[serde(skip_serializing_if = "Option::is_none")]
	index: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	blocks: Option<u32>,
}

/// Opens a DSK file from input path
fn open_dsk(input: &PathBuf, name: Option<String>) -> Result<dsk::DskFile, String> {
	if input.is_dir() {
		// Directory mode: need base name
		let base_name = name.ok_or("Base name required when input is a directory")?;
		dsk::DskFile::open(input, &base_name).map_err(|e| format!("Failed to open DSK/PFT: {}", e))
	} else if input.extension().and_then(|s| s.to_str()) == Some("DSK")
		|| input.extension().and_then(|s| s.to_str()) == Some("dsk")
	{
		// DSK file mode: derive PFT path
		let pft_path = input.with_extension("PFT");
		if !pft_path.exists() {
			let pft_path = input.with_extension("pft");
			if !pft_path.exists() {
				return Err(format!("PFT file not found for {}", input.display()));
			}
		}

		let pft_file =
			pft::File::open(&pft_path).map_err(|e| format!("Failed to open PFT file: {}", e))?;

		dsk::DskFile::open_with_pft(input, pft_file)
			.map_err(|e| format!("Failed to open DSK file: {}", e))
	} else {
		Err("Input must be a directory or .DSK file".to_string())
	}
}

/// Handles the 'info' command
fn handle_info(
	input: &PathBuf,
	name: Option<String>,
	detailed: bool,
	blocks: bool,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading DSK/PFT from: {}", input.display());
	}

	let mut dsk = open_dsk(input, name)?;

	println!("\n=== DSK Container Information ===");
	if let Some(name) = dsk.name() {
		println!("Name: {}", name);
	}

	let total_size = dsk.size().map_err(|e| format!("Failed to get size: {}", e))?;
	let total_blocks = dsk.num_blocks().map_err(|e| format!("Failed to get block count: {}", e))?;
	let num_files = dsk.num_files();

	println!("Total Size: {} bytes ({:.2} MB)", total_size, total_size as f64 / 1024.0 / 1024.0);
	println!("Total Blocks: {}", total_blocks);
	println!("Block Size: {} bytes", pft::Entry::block_size());
	println!("Files: {}", num_files);

	if detailed {
		println!("\n=== File List ===");
		let mut total_file_size = 0u64;
		let mut used_blocks = 0usize;

		for (idx, entry) in dsk.entries().enumerate() {
			let blocks_needed = entry.blocks_needed();
			total_file_size += entry.actual_size as u64;
			used_blocks += blocks_needed as usize;

			println!(
				"  [{:3}] {:8} - {:7} bytes ({:2} blocks) @ block {}",
				idx,
				entry.name(),
				entry.actual_size,
				blocks_needed,
				entry.index
			);
		}

		println!("\n=== Storage Statistics ===");
		println!(
			"Total File Size: {} bytes ({:.2} MB)",
			total_file_size,
			total_file_size as f64 / 1024.0 / 1024.0
		);
		println!("Used Blocks: {} / {}", used_blocks, total_blocks);
		println!("Free Blocks: {}", total_blocks.saturating_sub(used_blocks));
		println!("Utilization: {:.1}%", (used_blocks as f64 / total_blocks as f64) * 100.0);

		let waste = used_blocks as u64 * pft::Entry::block_size() as u64 - total_file_size;
		println!(
			"Wasted Space: {} bytes ({:.1}%)",
			waste,
			(waste as f64 / (used_blocks as u64 * pft::Entry::block_size() as u64) as f64) * 100.0
		);
	}

	if blocks {
		println!("\n=== Block Usage Map ===");
		let mut block_map = vec![false; total_blocks];

		for entry in dsk.entries() {
			let start = entry.index as usize;
			let count = entry.blocks_needed() as usize;
			let end = (start + count).min(total_blocks);
			block_map[start..end].fill(true);
		}

		// Print block map in rows of 64
		for (idx, chunk) in block_map.chunks(64).enumerate() {
			print!("{:4}: ", idx * 64);
			for &used in chunk {
				print!(
					"{}",
					if used {
						'█'
					} else {
						'░'
					}
				);
			}
			println!();
		}

		println!("\nLegend: █ = used, ░ = free");
	}

	Ok(())
}

/// Handles the 'list' command
fn handle_list(
	input: &PathBuf,
	name: Option<String>,
	sort: SortOrder,
	format: OutputFormat,
	filter: Option<String>,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading DSK/PFT from: {}", input.display());
	}

	let dsk = open_dsk(input, name)?;

	// Collect entries
	let mut entries: Vec<_> = dsk.entries().collect();

	// Apply filter
	if let Some(ref pattern) = filter {
		let pattern_lower = pattern.to_lowercase();
		entries.retain(|e| e.name().to_lowercase().contains(&pattern_lower));

		if verbose {
			println!("Filtered to {} entries matching '{}'", entries.len(), pattern);
		}
	}

	// Sort entries
	match sort {
		SortOrder::Index => entries.sort_by_key(|e| e.index),
		SortOrder::Name => entries.sort_by_key(|a| a.name()),
		SortOrder::Size => entries.sort_by_key(|e| e.actual_size),
		SortOrder::Blocks => entries.sort_by_key(|e| e.blocks_needed()),
	}

	// Output
	match format {
		OutputFormat::Table => {
			println!(
				"\n{:>5} | {:8} | {:>10} | {:>6} | {:>10}",
				"Index", "Name", "Size", "Blocks", "Block Idx"
			);
			println!("{:-<5}-+-{:-<8}-+-{:-<10}-+-{:-<6}-+-{:-<10}", "", "", "", "", "");

			for entry in &entries {
				println!(
					"{:>5} | {:8} | {:>10} | {:>6} | {:>10}",
					entry.index,
					entry.name(),
					entry.actual_size,
					entry.blocks_needed(),
					entry.index
				);
			}

			println!("\nTotal: {} files", entries.len());
		}
		OutputFormat::Json => {
			let json_entries: Vec<_> = entries
				.iter()
				.map(|e| {
					serde_json::json!({
						"index": e.index,
						"name": e.name(),
						"size": e.actual_size,
						"blocks": e.blocks_needed(),
					})
				})
				.collect();

			println!("{}", serde_json::to_string_pretty(&json_entries).unwrap());
		}
		OutputFormat::Csv => {
			println!("index,name,size,blocks,block_index");
			for entry in entries {
				println!(
					"{},{},{},{},{}",
					entry.index,
					entry.name(),
					entry.actual_size,
					entry.blocks_needed(),
					entry.index
				);
			}
		}
	}

	Ok(())
}

/// Handles the 'extract' command
fn handle_extract(
	input: &PathBuf,
	name: Option<String>,
	files: Vec<String>,
	all: bool,
	output: &PathBuf,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Loading DSK/PFT from: {}", input.display());
	}

	let mut dsk = open_dsk(input, name)?;

	// Create output directory
	fs::create_dir_all(output).map_err(|e| format!("Failed to create output directory: {}", e))?;

	if all {
		// Extract all files
		if verbose {
			println!("Extracting all files to: {}", output.display());
		}

		let mut count = 0;
		let entries_vec: Vec<_> = dsk.entries().copied().collect();
		for entry in entries_vec {
			let filename = entry.name();
			let output_path = output.join(&filename);

			if verbose {
				println!("  Extracting: {} ({} bytes)", filename, entry.actual_size);
			}

			let data = dsk
				.extract(&entry)
				.map_err(|e| format!("Failed to extract '{}': {}", filename, e))?;

			fs::write(&output_path, data)
				.map_err(|e| format!("Failed to write '{}': {}", output_path.display(), e))?;

			count += 1;
		}

		println!("✓ Extracted {} files to {}", count, output.display());
	} else if !files.is_empty() {
		// Extract specific files
		for file_spec in &files {
			// Try as index first
			if let Ok(index) = file_spec.parse::<usize>() {
				let entry =
					*dsk.pft().get_entry(index).ok_or(format!("Index {} not found", index))?;

				let filename = entry.name();
				let output_path = output.join(&filename);

				if verbose {
					println!(
						"  Extracting [{}]: {} ({} bytes)",
						index, filename, entry.actual_size
					);
				}

				let data = dsk
					.extract(&entry)
					.map_err(|e| format!("Failed to extract index {}: {}", index, e))?;

				fs::write(&output_path, data)
					.map_err(|e| format!("Failed to write '{}': {}", output_path.display(), e))?;
			} else {
				// Try as name
				let entry = *dsk
					.pft()
					.find_entry(file_spec)
					.ok_or(format!("File '{}' not found", file_spec))?;

				let output_path = output.join(file_spec);

				if verbose {
					println!("  Extracting: {} ({} bytes)", file_spec, entry.actual_size);
				}

				let data = dsk
					.extract(&entry)
					.map_err(|e| format!("Failed to extract '{}': {}", file_spec, e))?;

				fs::write(&output_path, data)
					.map_err(|e| format!("Failed to write '{}': {}", output_path.display(), e))?;
			}
		}

		println!("✓ Extracted {} files to {}", files.len(), output.display());
	} else {
		return Err("Either --all or file names/indices must be specified".to_string());
	}

	Ok(())
}

/// Handles the 'pack' command
fn handle_pack(
	input: &PathBuf,
	output: &Path,
	name: Option<String>,
	optimize: bool,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Packing from: {}", input.display());
	}

	// Determine output paths
	let (dsk_path, pft_path, base_name) = if output.is_dir() {
		let base = name.ok_or("Base name required when output is a directory")?;
		let dsk = output.join(format!("{}.DSK", base));
		let pft = output.join(format!("{}.PFT", base));
		(dsk, pft, base)
	} else {
		let base =
			output.file_stem().and_then(|s| s.to_str()).ok_or("Invalid output path")?.to_string();
		let dsk = output.with_extension("DSK");
		let pft = output.with_extension("PFT");
		(dsk, pft, base)
	};

	if verbose {
		println!("Output DSK: {}", dsk_path.display());
		println!("Output PFT: {}", pft_path.display());
	}

	// Check if input is metadata JSON
	let files_to_pack = if input.extension().and_then(|s| s.to_str()) == Some("json") {
		// Load from metadata
		let json_str =
			fs::read_to_string(input).map_err(|e| format!("Failed to read metadata: {}", e))?;
		let metadata: PackMetadata = serde_json::from_str(&json_str)
			.map_err(|e| format!("Failed to parse metadata: {}", e))?;

		metadata.files
	} else if input.is_dir() {
		// Scan directory
		let mut files = Vec::new();
		for entry in fs::read_dir(input).map_err(|e| format!("Failed to read directory: {}", e))? {
			let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
			let path = entry.path();

			if path.is_file() {
				let size = fs::metadata(&path)
					.map_err(|e| format!("Failed to get file size: {}", e))?
					.len() as u32;

				let name = path
					.file_name()
					.and_then(|s| s.to_str())
					.ok_or("Invalid filename")?
					.to_string();

				// Validate name length (max 8 chars)
				let truncated_name = if name.len() > 8 {
					if verbose {
						println!("Warning: Truncating '{}' to 8 characters", name);
					}
					name[..8].to_string()
				} else {
					name
				};

				files.push(FileMetadata {
					name: truncated_name,
					path: path.clone(),
					size,
					index: None,
					blocks: None,
				});
			}
		}

		files
	} else {
		return Err("Input must be a directory or JSON metadata file".to_string());
	};

	if files_to_pack.is_empty() {
		return Err("No files to pack".to_string());
	}

	if verbose {
		println!("Found {} files to pack", files_to_pack.len());
	}

	// Sort files if optimizing
	let mut files = files_to_pack;
	if optimize {
		// Sort by size descending (larger files first)
		files.sort_by(|a, b| b.size.cmp(&a.size));
		if verbose {
			println!("Optimizing block allocation (largest files first)");
		}
	}

	// Allocate blocks and create PFT entries
	let mut pft_entries = Vec::new();
	let mut current_block = 0u32;
	let mut dsk_data = Vec::new();

	for file in &files {
		let data = fs::read(&file.path)
			.map_err(|e| format!("Failed to read '{}': {}", file.path.display(), e))?;

		if data.len() as u32 != file.size {
			return Err(format!(
				"File size mismatch for '{}': expected {}, got {}",
				file.name,
				file.size,
				data.len()
			));
		}

		let blocks_needed = (file.size as usize).div_ceil(pft::Entry::block_size());

		if verbose {
			println!(
				"  {} - {} bytes ({} blocks) @ block {}",
				file.name, file.size, blocks_needed, current_block
			);
		}

		// Create PFT entry
		let entry = pft::Entry::new(&file.name, current_block, file.size);
		pft_entries.push(entry);

		// Write data to DSK with padding
		dsk_data.extend_from_slice(&data);

		// Pad to block boundary
		let padding_needed = blocks_needed * pft::Entry::block_size() - data.len();
		dsk_data.extend(vec![0u8; padding_needed]);

		current_block += blocks_needed as u32;
	}

	// Create and save PFT file
	let pft_file = pft::File::new(pft_entries);
	let pft_bytes = pft_file.to_bytes();
	fs::write(&pft_path, pft_bytes).map_err(|e| format!("Failed to write PFT file: {}", e))?;

	// Save DSK file
	fs::write(&dsk_path, dsk_data).map_err(|e| format!("Failed to write DSK file: {}", e))?;

	println!("✓ Created {}.DSK ({} files, {} blocks)", base_name, files.len(), current_block);
	println!("  DSK: {}", dsk_path.display());
	println!("  PFT: {}", pft_path.display());

	// Save metadata for future reference
	let metadata_path =
		output.parent().unwrap_or(output).join(format!("{}_metadata.json", base_name));
	let metadata = PackMetadata {
		name: base_name,
		files: files
			.iter()
			.zip(pft_file.entries().iter())
			.map(|(f, e)| FileMetadata {
				name: f.name.clone(),
				path: f.path.clone(),
				size: e.actual_size,
				index: Some(e.index),
				blocks: Some(e.blocks_needed()),
			})
			.collect(),
		total_blocks: current_block as usize,
		block_size: pft::Entry::block_size(),
	};

	let metadata_json = serde_json::to_string_pretty(&metadata).unwrap();
	fs::write(&metadata_path, metadata_json)
		.map_err(|e| format!("Failed to write metadata: {}", e))?;

	if verbose {
		println!("  Metadata: {}", metadata_path.display());
	}

	Ok(())
}

/// Handles the 'verify' command
fn handle_verify(
	input: &PathBuf,
	name: Option<String>,
	strict: bool,
	verbose: bool,
) -> Result<(), String> {
	if verbose {
		println!("Verifying DSK/PFT: {}", input.display());
	}

	let mut dsk = open_dsk(input, name)?;

	println!("\n=== Verification Results ===");

	// Basic validation
	dsk.pft().validate().map_err(|e| format!("PFT validation failed: {}", e))?;

	println!("✓ PFT header is valid");

	// DSK validation
	dsk.validate().map_err(|e| format!("DSK validation failed: {}", e))?;

	println!("✓ DSK structure is valid");

	// Check each entry
	let total_blocks = dsk.num_blocks().map_err(|e| format!("Failed to get block count: {}", e))?;
	let mut errors = Vec::new();
	let mut warnings = Vec::new();

	let entries_vec: Vec<_> = dsk.pft().entries().to_vec();
	for (idx, entry) in entries_vec.iter().enumerate() {
		// Check if entry is valid
		if !entry.is_valid() {
			warnings.push(format!("Entry {} has zero size and empty name", idx));
			continue;
		}

		// Check name
		if !entry.raw_name.iter().all(|&b| b.is_ascii() || b == 0) {
			errors.push(format!("Entry {}: Name contains non-ASCII bytes", idx));
		}

		// Check block range
		let blocks_needed = entry.blocks_needed() as usize;
		let end_block = entry.index as usize + blocks_needed;

		if end_block > total_blocks {
			errors.push(format!(
				"Entry {} ('{}'): Requires blocks {}-{}, but only {} blocks available",
				idx,
				entry.name(),
				entry.index,
				end_block - 1,
				total_blocks
			));
		}

		// Strict mode: additional checks
		if strict {
			// Check for inconsistencies
			if entry.actual_size == 0 && entry.index != 0 {
				warnings.push(format!(
					"Entry {} ('{}'): Zero size but non-zero index {}",
					idx,
					entry.name(),
					entry.index
				));
			}

			// Try to extract (validates data integrity)
			if entry.is_valid() {
				match dsk.extract(entry) {
					Ok(data) => {
						if data.len() != entry.actual_size as usize {
							errors.push(format!(
								"Entry {} ('{}'): Extracted size {} doesn't match expected {}",
								idx,
								entry.name(),
								data.len(),
								entry.actual_size
							));
						}
					}
					Err(e) => {
						errors.push(format!(
							"Entry {} ('{}'): Extraction failed: {}",
							idx,
							entry.name(),
							e
						));
					}
				}
			}
		}
	}

	// Report results
	if !warnings.is_empty() {
		println!("\n⚠ Warnings:");
		for warning in &warnings {
			println!("  {}", warning);
		}
	}

	if !errors.is_empty() {
		println!("\n✗ Errors:");
		for error in &errors {
			println!("  {}", error);
		}
		return Err(format!("Verification failed with {} error(s)", errors.len()));
	}

	println!("\n✓ Verification passed");
	if strict {
		println!("  (Strict mode - all files extracted successfully)");
	}

	Ok(())
}

fn main() {
	let cli = Cli::parse();

	let result = match cli.command {
		Commands::Info {
			input,
			name,
			detailed,
			blocks,
			verbose,
		} => handle_info(&input, name, detailed, blocks, verbose),
		Commands::List {
			input,
			name,
			sort,
			format,
			filter,
			verbose,
		} => handle_list(&input, name, sort, format, filter, verbose),
		Commands::Extract {
			input,
			name,
			files,
			all,
			output,
			verbose,
		} => handle_extract(&input, name, files, all, &output, verbose),
		Commands::Pack {
			input,
			output,
			name,
			optimize,
			verbose,
		} => handle_pack(&input, output.as_path(), name, optimize, verbose),
		Commands::Verify {
			input,
			name,
			strict,
			verbose,
		} => handle_verify(&input, name, strict, verbose),
	};

	if let Err(e) = result {
		eprintln!("Error: {}", e);
		std::process::exit(1);
	}
}
