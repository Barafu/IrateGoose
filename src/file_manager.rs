use std::{fs, path::{Path, PathBuf}};
use anyhow::Result;
use rayon::prelude::*;

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
        rayon::ThreadPoolBuilder::new().num_threads(2).build_global().unwrap();
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

    pub fn relative_paths(&self) -> Vec<PathBuf> {
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
        let output = match std::process::Command::new("file")
            .arg(path)
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => return WaveSampleRate::Unknown,
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("44100") {
            WaveSampleRate::F44100
        } else if stdout.contains("48000") {
            WaveSampleRate::F48000
        } else {
            WaveSampleRate::Unknown
        }
    }

    fn scan_folder(&mut self, path: &Path) -> Result<()>
    {
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
        };
        Ok(())

    }
}