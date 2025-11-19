//! ANM validation utility.
//!
//! Provides two subcommands:
//! - `validate`: scan a directory (defaults to `bin/anm_extract`) and check every
//!   `.ANM` file with the simulated parser and raw reader.
//! - `inspect`: deep-dive into a single file and optionally focus on one slot.

use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use dvine_rs::prelude::file::anm::{
	AnimationSequence, File as AnmFile, ParseConfig, compute_slot_windows, constants,
	sequence::SequenceParseStats,
};
use walkdir::WalkDir;

fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Command::Validate(opts) => run_validate(opts),
		Command::Inspect(opts) => run_inspect(opts),
	}
}

#[derive(Parser)]
#[command(name = "anm_utils")]
#[command(author = "dvine-rs project")]
#[command(version)]
#[command(about = "Validate and inspect animation (.ANM) files", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand)]
enum Command {
	/// Validate every .ANM file under a directory
	Validate(ValidateArgs),
	/// Inspect a single .ANM file and optionally focus on one slot
	Inspect(InspectArgs),
}

#[derive(Args)]
struct ValidateArgs {
	/// Directory containing extracted .ANM files
	#[arg(short = 'd', long, value_name = "DIR", default_value = "bin/anm_extract")]
	root: PathBuf,

	/// Recurse into sub-directories while scanning
	#[arg(short, long, default_value_t = false)]
	recursive: bool,

	/// Print per-slot diagnostics even when clean
	#[arg(short, long, default_value_t = false)]
	verbose: bool,

	/// Exit with an error when warnings are encountered
	#[arg(long, default_value_t = false)]
	fail_on_warning: bool,

	/// Maximum number of simulated parser iterations before assuming a loop
	#[arg(long, value_name = "COUNT", default_value_t = 5000)]
	max_iterations: usize,

	/// Maximum visits per frame index before stopping
	#[arg(long, value_name = "COUNT", default_value_t = 128)]
	max_visits_per_index: usize,
}

#[derive(Args)]
struct InspectArgs {
	/// Path to a single .ANM file
	#[arg(value_name = "FILE")]
	file: PathBuf,

	/// Only show diagnostics for the specified slot (0-255)
	#[arg(short, long, value_name = "SLOT")]
	slot: Option<usize>,

	/// Show clean slots in addition to warnings/errors
	#[arg(short, long, default_value_t = false)]
	verbose: bool,

	/// Maximum number of simulated parser iterations before assuming a loop
	#[arg(long, value_name = "COUNT", default_value_t = 5000)]
	max_iterations: usize,

	/// Maximum visits per frame index before stopping
	#[arg(long, value_name = "COUNT", default_value_t = 128)]
	max_visits_per_index: usize,
}

fn run_validate(args: ValidateArgs) -> Result<()> {
	if !args.root.exists() {
		bail!("Root directory {} does not exist", args.root.display());
	}
	if !args.root.is_dir() {
		bail!("{} is not a directory", args.root.display());
	}

	let config = build_config(args.max_iterations, args.max_visits_per_index)?;
	let files = collect_anm_files(&args.root, args.recursive)?;
	if files.is_empty() {
		println!("No .ANM files found under {}", args.root.display());
		return Ok(());
	}

	let root_display = args.root.canonicalize().unwrap_or(args.root.clone());
	let mut totals = ScanTotals::default();

	for path in files {
		match validate_file(&path, &config) {
			Ok(report) => {
				totals.update(&report);
				print_file_report(&report, &path, Some(&root_display), args.verbose);
			}
			Err(err) => {
				totals.record_failure();
				println!("{} {} - {}", Severity::Error.icon(), path.display(), err);
			}
		}
	}

	print_summary(&totals);

	if totals.files_error > 0 {
		bail!("Validation finished with errors (see summary)");
	}
	if args.fail_on_warning && totals.files_warning > 0 {
		bail!("Validation finished with warnings (see summary)");
	}

	Ok(())
}

fn run_inspect(args: InspectArgs) -> Result<()> {
	if let Some(slot) = args.slot {
		if slot >= constants::ANIMATION_SLOT_COUNT {
			bail!("Slot {} out of range (max {})", slot, constants::ANIMATION_SLOT_COUNT - 1);
		}
	}

	let config = build_config(args.max_iterations, args.max_visits_per_index)?;
	let report = validate_file(&args.file, &config)?;

	println!("File: {} (size: {} bytes)", args.file.display(), report.file_size);
	println!(
		"SPR filename: {}",
		if report.spr_filename.is_empty() {
			"<unset>"
		} else {
			report.spr_filename.as_str()
		}
	);
	println!(
		"Slots analyzed: {} | warnings: {} | errors: {}",
		report.active_slots(),
		report.warning_slots(),
		report.error_slots()
	);
	println!(
		"Simulated frames: {} | bytes≈{} | slots exhausted: {}",
		report.frames_total, report.bytes_total, report.slots_exhausted
	);

	let mut slots: Vec<&SlotSummary> = match args.slot {
		Some(idx) => report.slot_reports.iter().filter(|summary| summary.slot == idx).collect(),
		None => report.slot_reports.iter().collect(),
	};

	if slots.is_empty() {
		println!("No matching slots found.");
		return Ok(());
	}

	slots.sort_by_key(|summary| summary.slot);

	for slot in slots {
		if !args.verbose && args.slot.is_none() && slot.severity == Severity::Ok {
			continue;
		}
		print_slot_details(slot, "  ", true);
	}

	Ok(())
}

fn build_config(max_iterations: usize, max_visits: usize) -> Result<ParseConfig> {
	if max_iterations == 0 {
		bail!("max-iterations must be greater than zero");
	}
	if max_visits == 0 {
		bail!("max-visits-per-index must be greater than zero");
	}
	Ok(ParseConfig::new(max_iterations, max_visits))
}

fn collect_anm_files(root: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
	let max_depth = if recursive {
		usize::MAX
	} else {
		1
	};
	let mut files = Vec::new();

	for entry in WalkDir::new(root).max_depth(max_depth).follow_links(false).into_iter() {
		let entry = match entry {
			Ok(entry) => entry,
			Err(err) => {
				println!("{}", err);
				continue;
			}
		};

		if entry.file_type().is_file() {
			files.push(entry.into_path());
		}
	}

	files.sort();
	Ok(files)
}

fn validate_file(path: &Path, config: &ParseConfig) -> Result<FileReport> {
	let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
	let anm = AnmFile::from_bytes(&bytes)
		.with_context(|| format!("Failed to parse header/index for {}", path.display()))?;

	let mut slot_reports = Vec::new();
	let mut severity = Severity::Ok;
	let mut frames_total = 0usize;
	let mut bytes_total = 0usize;
	let mut slots_exhausted = 0usize;
	let slot_windows = compute_slot_windows(anm.index_table(), bytes.len());

	for (slot, &word_offset) in anm.index_table().iter().enumerate() {
		if word_offset == constants::NO_ANIMATION {
			continue;
		}

		let byte_offset = constants::ANIMATION_DATA_OFFSET + (word_offset as usize * 2);
		let mut summary = SlotSummary::new(slot, word_offset, byte_offset);

		let Some(window) = slot_windows[slot] else {
			summary.add_error(format!(
				"Slot offset 0x{byte_offset:05X} exceeds file length ({} bytes)",
				bytes.len()
			));
			severity.escalate(summary.severity);
			slot_reports.push(summary);
			continue;
		};

		if window.len() < constants::FRAME_DESCRIPTOR_SIZE {
			summary.add_error(format!("Slot data window is too small ({} bytes)", window.len()));
			severity.escalate(summary.severity);
			slot_reports.push(summary);
			continue;
		}

		let slice = &bytes[window.start..window.end];

		match AnimationSequence::from_bytes_with_config(slice, config) {
			Ok((sequence, stats)) => {
				summary.frames = sequence.len();
				summary.stats = Some(stats);
				frames_total += sequence.len();
				bytes_total += stats.bytes_consumed;
				if stats.exhausted_data_window {
					slots_exhausted += 1;
				}

				if stats.loop_detected {
					summary.add_warning(format!(
						"Loop guard triggered after visiting {} unique positions",
						stats.unique_frame_positions
					));
				}
			}
			Err(err) => {
				summary.add_error(format!("Simulated parser failed: {err}"));
			}
		}

		match AnimationSequence::from_bytes_raw(slice) {
			Ok((raw_sequence, raw_bytes)) => {
				if summary.frames == 0 {
					summary.frames = raw_sequence.len();
				}
				summary.raw_bytes = Some(raw_bytes);
			}
			Err(err) => {
				summary.add_error(format!("Raw reader failed: {err}"));
			}
		}

		severity.escalate(summary.severity);
		slot_reports.push(summary);
	}

	Ok(FileReport {
		severity,
		slot_reports,
		spr_filename: anm.spr_filename().to_string(),
		file_size: bytes.len(),
		frames_total,
		bytes_total,
		slots_exhausted,
	})
}

fn print_file_report(report: &FileReport, path: &Path, root: Option<&Path>, verbose: bool) {
	let rel = root
		.and_then(|root| path.strip_prefix(root).ok())
		.map(|p| p.display().to_string())
		.unwrap_or_else(|| path.display().to_string());

	println!(
		"{} {:<50} | slots {:3} warn {:2} err {:2} | frames {:5} bytes {:7} exh {:3} | spr {}",
		report.severity.icon(),
		rel,
		report.active_slots(),
		report.warning_slots(),
		report.error_slots(),
		report.frames_total,
		report.bytes_total,
		report.slots_exhausted,
		if report.spr_filename.is_empty() {
			"<unset>"
		} else {
			report.spr_filename.as_str()
		}
	);

	let mut printed_details = false;
	if verbose || report.severity != Severity::Ok {
		for slot in &report.slot_reports {
			if !verbose && slot.severity == Severity::Ok {
				continue;
			}
			printed_details = true;
			print_slot_details(slot, "    ", verbose);
		}
	}

	if printed_details {
		println!();
	}
}

fn print_slot_details(slot: &SlotSummary, indent: &str, include_clean_note: bool) {
	println!(
		"{indent}Slot {:03} (word 0x{:04X}, byte 0x{:05X}) - {:<4} - {} frames",
		slot.slot,
		slot.word_offset,
		slot.byte_offset,
		slot.severity.short_label(),
		slot.frames
	);

	if let Some(stats) = slot.stats {
		println!(
			"{indent}    stats: unique_positions={} bytes={} window_exhausted={} loop={}",
			stats.unique_frame_positions,
			stats.bytes_consumed,
			stats.exhausted_data_window,
			stats.loop_detected
		);
	}

	if let Some(raw_bytes) = slot.raw_bytes {
		println!("{indent}    raw bytes consumed: {}", raw_bytes);
	}

	if slot.issues.is_empty() {
		if include_clean_note {
			println!("{indent}    [OK] sequence ended cleanly");
		}
		return;
	}

	for issue in &slot.issues {
		println!("{indent}    [{}] {}", issue.severity.short_label(), issue.message);
	}
}

fn print_summary(totals: &ScanTotals) {
	println!(
		"\nSummary: files={} | ok={} warn={} err={} | slots={} warn={} err={} exhausted={} | frames={} bytes≈{}",
		totals.files_total,
		totals.files_ok,
		totals.files_warning,
		totals.files_error,
		totals.slots,
		totals.warning_slots,
		totals.error_slots,
		totals.slots_exhausted,
		totals.frames,
		totals.bytes
	);
}

#[derive(Default)]
struct ScanTotals {
	files_total: usize,
	files_ok: usize,
	files_warning: usize,
	files_error: usize,
	slots: usize,
	warning_slots: usize,
	error_slots: usize,
	frames: usize,
	bytes: usize,
	slots_exhausted: usize,
}

impl ScanTotals {
	fn update(&mut self, report: &FileReport) {
		self.files_total += 1;
		self.slots += report.active_slots();
		self.warning_slots += report.warning_slots();
		self.error_slots += report.error_slots();
		self.frames += report.frames_total;
		self.bytes += report.bytes_total;
		self.slots_exhausted += report.slots_exhausted;

		match report.severity {
			Severity::Ok => self.files_ok += 1,
			Severity::Warning => self.files_warning += 1,
			Severity::Error => self.files_error += 1,
		}
	}

	fn record_failure(&mut self) {
		self.files_total += 1;
		self.files_error += 1;
	}
}

struct FileReport {
	severity: Severity,
	slot_reports: Vec<SlotSummary>,
	spr_filename: String,
	file_size: usize,
	frames_total: usize,
	bytes_total: usize,
	slots_exhausted: usize,
}

impl FileReport {
	fn active_slots(&self) -> usize {
		self.slot_reports.len()
	}

	fn warning_slots(&self) -> usize {
		self.slot_reports.iter().filter(|slot| slot.severity == Severity::Warning).count()
	}

	fn error_slots(&self) -> usize {
		self.slot_reports.iter().filter(|slot| slot.severity == Severity::Error).count()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
	Ok,
	Warning,
	Error,
}

impl Severity {
	fn escalate(&mut self, other: Severity) {
		if other > *self {
			*self = other;
		}
	}

	fn icon(&self) -> &'static str {
		match self {
			Severity::Ok => "✅",
			Severity::Warning => "⚠️ ",
			Severity::Error => "❌",
		}
	}

	fn short_label(&self) -> &'static str {
		match self {
			Severity::Ok => "OK",
			Severity::Warning => "WARN",
			Severity::Error => "ERR",
		}
	}
}

struct SlotSummary {
	slot: usize,
	word_offset: u16,
	byte_offset: usize,
	frames: usize,
	stats: Option<SequenceParseStats>,
	raw_bytes: Option<usize>,
	severity: Severity,
	issues: Vec<SlotIssue>,
}

impl SlotSummary {
	fn new(slot: usize, word_offset: u16, byte_offset: usize) -> Self {
		Self {
			slot,
			word_offset,
			byte_offset,
			frames: 0,
			stats: None,
			raw_bytes: None,
			severity: Severity::Ok,
			issues: Vec::new(),
		}
	}

	fn add_warning(&mut self, message: String) {
		self.severity.escalate(Severity::Warning);
		self.issues.push(SlotIssue {
			severity: Severity::Warning,
			message,
		});
	}

	fn add_error(&mut self, message: String) {
		self.severity = Severity::Error;
		self.issues.push(SlotIssue {
			severity: Severity::Error,
			message,
		});
	}
}

struct SlotIssue {
	severity: Severity,
	message: String,
}
