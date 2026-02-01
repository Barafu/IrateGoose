use anyhow::Result;
use rayon::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::{
    fs,
    path::{Path, PathBuf},
    collections::HashMap,
};

use crate::descriptions::HRTFMetadata;
use crate::settings::AppSettings;
use xxhash_rust::xxh3::xxh3_64;

pub struct FileManager {
    settings: Rc<RefCell<AppSettings>>,
    pub wave_data: Vec<WaveFileData>,
    /// Wavefile dir that was scanned last time.
    current_wavefile_dir: Option<PathBuf>,
    descriptions: crate::descriptions::Descriptions,
}

// All about Wav file
#[derive(Debug, Default)]
pub struct WaveFileData {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub sample_rate: WaveSampleRate,
    pub metadata: Option<Rc<HRTFMetadata>>,
    pub checksum: u64,
}

// Detected sample rate of Wav file
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum WaveSampleRate {
    F48000,
    F44100,
    F96000,
    #[default]
    Unknown,
    Damaged,
}

impl FileManager {
    pub fn new(settings: Rc<RefCell<AppSettings>>, descriptions: crate::descriptions::Descriptions) -> FileManager {
        FileManager {
            settings,
            wave_data: Vec::new(),
            current_wavefile_dir: None,
            descriptions,
        }
    }

    /// Searches for WAV files inside the wavefile_dir, and read info from the files it found.
    pub fn rescan_configured_directory(&mut self) -> Result<()> {
        // Detect WAV files
        self.wave_data.clear();
        self.current_wavefile_dir = self.settings.borrow().get_wav_directory();
        let working_path = match self.current_wavefile_dir.clone() {
            Some(dir) => dir,
            None => return Ok(()), // No directory configured, nothing to scan
        };
        self.scan_directory(&working_path)?;

        // Detect sample rates and compute checksums
        // This will store intermediate results
        struct FileMetadataRecord {
            samplerate: WaveSampleRate,
            checksum: u64,
        }
        // Copy all file paths, keeping the order
        let paths: Vec<PathBuf> = self.wave_data.iter().map(|w|w.path.clone()).collect();
        // Multithreaded scan of files to collect metadata
        let metarecords: Vec<FileMetadataRecord> = paths.par_iter().map(|path| {
            let (samplerate, checksum) = Self::detect_sample_rate_and_checksum(&path);
            FileMetadataRecord {
                samplerate,
                checksum,
            }
        }).collect();
        // Copy collected metadta back to wave data
        self.wave_data.iter_mut().zip(metarecords.iter()).for_each(|d|{
            d.0.sample_rate = d.1.samplerate;
            d.0.checksum = d.1.checksum;
        });

        // Sort entries: HeSuVi entries first, then alphabetically by path
        self.wave_data.sort_by(|a, b| {
            let a_is_hesuvi = a.path.to_string_lossy().contains("HeSuVi/");
            let b_is_hesuvi = b.path.to_string_lossy().contains("HeSuVi/");

            match (a_is_hesuvi, b_is_hesuvi) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.path.cmp(&b.path), // both HeSuVi or both non-HeSuVi
            }
        });

        // Populate metadata from descriptions
        for wave in &mut self.wave_data {
            let stem = wave.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            wave.metadata = self.descriptions.get_rc(stem);
        }

        self.wave_data.shrink_to_fit();
        Ok(())
    }


    fn detect_sample_rate_and_checksum(path: &Path) -> (WaveSampleRate, u64) {
        // Read entire file
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(_) => return (WaveSampleRate::Damaged, 0),
        };

        // Check length
        if data.len() < 28 {
            return (WaveSampleRate::Damaged, 0);
        }

        // Verify WAV header
        if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return (WaveSampleRate::Damaged, 0);
        }

        // Extract sample rate
        let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
        let wave_sample_rate = match sample_rate {
            44100 => WaveSampleRate::F44100,
            48000 => WaveSampleRate::F48000,
            96000 => WaveSampleRate::F96000,
            _ => WaveSampleRate::Unknown,
        };

        // Compute xxh3 hash
        let hash = xxh3_64(&data);

        (wave_sample_rate, hash)
    }

    fn scan_directory(&mut self, path: &Path) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory(&path)?;
            } else {
                // Only store files that end with .wav (case-insensitive)
                let ext = match path.extension() {
                    Some(ext) => ext,
                    None => continue,
                };
                let ext_str = match ext.to_str() {
                    Some(s) => s,
                    None => continue,
                };
                if !ext_str.eq_ignore_ascii_case("wav") {
                    continue;
                }
                // Compute relative path relative to current_wavefile_dir
                let relative_path = match &self.current_wavefile_dir {
                    Some(base_dir) => path
                        .strip_prefix(base_dir)
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|_| path.clone()),
                    None => path.clone(),
                };
                // Store absolute path with detected sample rate
                self.wave_data.push(WaveFileData {
                    path,
                    relative_path,
                    ..Default::default()
                });
            }
        }
        Ok(())
    }
}
