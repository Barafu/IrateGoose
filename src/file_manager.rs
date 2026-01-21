use std::{fs, io, path::{Path, PathBuf}};
use anyhow::Result;

pub struct FileManager {
    wavefile_dir: PathBuf,
    pub wave_data: Vec<WaveFileData>,
}

#[derive(Debug, Default)]
pub struct WaveFileData {
    pub path: PathBuf,
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

    pub fn rescan_folder(&mut self) -> Result<()> {
        self.wave_data.clear();
        let w = self.wavefile_dir.clone();
        self.scan_folder(&w)?;
        
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

    fn scan_folder(&mut self, path: &Path) -> Result<()>
    {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_folder(&path)?;
            } else {
                // Only store files that end with .wav
                if let Some(ext) = path.extension() {
                    if ext == "wav" {
                        // Store absolute path
                        self.wave_data.push(WaveFileData { path: path.clone() });
                    }
                }
            }
        };
        Ok(())

    }
}