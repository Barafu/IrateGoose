use anyhow::Result;
use rayon::prelude::*;
use std::io::Read;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct FileManager {
    wavefile_dir: PathBuf,
    pub wave_data: Vec<WaveFileData>,
}

// All about Wav file
#[derive(Debug, Default)]
pub struct WaveFileData {
    pub path: PathBuf,
    pub sample_rate: WaveSampleRate,
}

// Detected sample rate of Wav file
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum WaveSampleRate {
    F48000,
    F44100,
    #[default]
    Unknown,
    Damaged,
}

impl FileManager {
    pub fn new(path: PathBuf) -> Result<FileManager> {
        let mut f = FileManager {
            wavefile_dir: path,
            wave_data: Vec::new(),
        };
        f.rescan_folder()?;
        Ok(f)
    }

    /// Searches for WAV files inside the wavefile_dir, and read info from the files it found.
    pub fn rescan_folder(&mut self) -> Result<()> {
        // Detect WAV files
        self.wave_data.clear();
        let w = self.wavefile_dir.clone();
        self.scan_folder(&w)?;

        // Detect sample rates
        self.wave_data.par_iter_mut().for_each(|wave| {
            wave.sample_rate = Self::detect_sample_rate(&wave.path);
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

        self.wave_data.shrink_to_fit();
        Ok(())
    }

    /// Creates a list of relative paths to all detected WAV files
    pub fn list_relative_paths(&self) -> Vec<PathBuf> {
        self.wave_data
            .iter()
            .map(|wave| {
                wave.path
                    .strip_prefix(&self.wavefile_dir)
                    .map(|rel| rel.to_path_buf())
                    .unwrap_or_else(|_| wave.path.clone())
            })
            .collect()
    }

    pub fn absolute_path(&self, index: usize) -> &Path {
        &self.wave_data[index].path
    }

    fn detect_sample_rate(path: &Path) -> WaveSampleRate {
        // return WaveSampleRate::Damaged; // Uncomment for debug
        // Open the file
        let mut file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(_) => return WaveSampleRate::Damaged,
        };

        // Read at least 28 bytes to get the sample rate at offset 24-27
        let mut buffer = [0u8; 28];
        match file.read_exact(&mut buffer) {
            Ok(_) => (),
            Err(_) => return WaveSampleRate::Damaged,
        }

        // Verify WAV header
        // Bytes 0-3 should be "RIFF", bytes 8-11 should be "WAVE"
        if &buffer[0..4] != b"RIFF" || &buffer[8..12] != b"WAVE" {
            return WaveSampleRate::Damaged;
        }

        // Extract sample rate from bytes 24-27 (little-endian u32)
        let sample_rate = u32::from_le_bytes([buffer[24], buffer[25], buffer[26], buffer[27]]);

        // Match to known sample rates
        match sample_rate {
            44100 => WaveSampleRate::F44100,
            48000 => WaveSampleRate::F48000,
            _ => WaveSampleRate::Unknown,
        }
    }

    fn scan_folder(&mut self, path: &Path) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_folder(&path)?;
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
                // Store absolute path with detected sample rate
                self.wave_data.push(WaveFileData {
                    path: path.clone(),
                    sample_rate: WaveSampleRate::Unknown,
                });
            }
        }
        Ok(())
    }
}
