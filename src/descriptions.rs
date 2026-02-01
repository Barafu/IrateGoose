#![allow(dead_code)]

use anyhow::{Result, anyhow};
use csv::ReaderBuilder;
use log::warn;
use std::collections::BTreeMap;
use std::io::Read;
use std::rc::Rc;

/// Represents the configuration type for HRTF measurements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Configuration {
    Headphones,
    Speakers,
}

impl Configuration {
    /// Parse a string into a Configuration enum
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "Headphones" => Some(Configuration::Headphones),
            "Speakers" => Some(Configuration::Speakers),
            "" => None, // Empty string is treated as None
            _ => None,  // Invalid values are also treated as None
        }
    }
}

/// Represents a single entry from the HRTF descriptions CSV (excluding the HRIR filename)
#[derive(Debug, Clone, Default)]
pub struct HRTFMetadata {
    pub hrtf: String,
    pub configuration: Option<Configuration>,
    pub description: String,
    pub source: String,
    pub credits: String,
    pub points: Option<u32>,
}

/// Provides descriptions and credits for WAV files from the embedded database
pub struct Descriptions {
    /// Maps HRIR filename (without extension) to its description entry
    entries: BTreeMap<String, Rc<HRTFMetadata>>,
}

impl Descriptions {
    /// Creates a new Descriptions instance by loading and parsing the embedded CSV database
    pub fn new() -> Result<Self> {
        // Load the compressed CSV data embedded in the binary
        const COMPRESSED_DATA: &[u8] = include_bytes!("../data/HRTF_Descriptions.csv.zst");

        // Decompress the ZSTD compressed data
        let mut decoder = zstd::Decoder::new(COMPRESSED_DATA)?;
        let mut decompressed_data = Vec::new();
        decoder.read_to_end(&mut decompressed_data)?;

        // Parse the CSV data (semicolon-separated)
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_reader(decompressed_data.as_slice());

        let mut entries = BTreeMap::new();

        for result in rdr.records() {
            let record = result?;

            // Expected columns: HRIR;HRTF;Configuration;Description;Source;Credits;Points
            if record.len() != 7 {
                return Err(anyhow!(
                    "Invalid CSV record length: expected 7 columns, got {}",
                    record.len()
                ));
            }

            let hrir = record[0].to_string();

            // HRIR should be unique
            if entries.contains_key(&hrir) {
                warn!("Non-unique HRIR value '{}', skipping second entry", hrir);
                continue;
            }

            // Parse configuration field
            let config_str = record[2].trim();
            let configuration = Configuration::from_str(config_str);
            if !config_str.is_empty() && configuration.is_none() {
                warn!(
                    "Invalid configuration value '{}' for HRIR '{}', treating as None",
                    config_str, hrir
                );
            }

            // Parse points field
            let points_str = record[6].trim();
            let points = if points_str.is_empty() {
                None
            } else {
                match points_str.parse::<u32>() {
                    Ok(value) => Some(value),
                    Err(e) => {
                        warn!(
                            "Failed to parse points '{}' as u32 for HRIR '{}': {}, treating as None",
                            points_str, hrir, e
                        );
                        None
                    }
                }
            };

            let entry = HRTFMetadata {
                hrtf: record[1].to_string(),
                configuration,
                description: record[3].to_string(),
                source: record[4].to_string(),
                credits: record[5].to_string(),
                points,
            };

            entries.insert(hrir, Rc::new(entry));
        }

        Ok(Self { entries })
    }

    /// Get a shared reference-counted handle to the metadata.
    pub fn get_rc(&self, hrir_filename: &str) -> Option<Rc<HRTFMetadata>> {
        self.entries.get(hrir_filename).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptions_loading() {
        let descriptions = Descriptions::new();
        assert!(
            descriptions.is_ok(),
            "Failed to load descriptions: {:?}",
            descriptions.err()
        );

        let descriptions = descriptions.unwrap();
        assert!(
            !descriptions.entries.is_empty(),
            "Descriptions database should not be empty"
        );

        // Test that we can retrieve some entries
        // Note: We don't know specific HRIR filenames in the database,
        // but we can at least verify the iterator works
        let count = descriptions.entries.len();
        assert!(count > 0, "Expected at least one entry in the database");

        // Verify that iterating works (some fields may be empty in the CSV)
        let mut iter_count = 0;
        for (hrir, _entry) in descriptions.entries.iter() {
            assert!(!hrir.is_empty(), "HRIR filename should not be empty");
            iter_count += 1;
        }
        assert_eq!(iter_count, count, "Iterator should return all entries");

        println!("Successfully loaded {} HRTF descriptions", count);
    }

    #[test]
    fn test_configuration_parsing() {
        assert_eq!(
            Configuration::from_str("Headphones"),
            Some(Configuration::Headphones)
        );
        assert_eq!(
            Configuration::from_str("Speakers"),
            Some(Configuration::Speakers)
        );
        assert_eq!(Configuration::from_str(""), None);
        assert_eq!(Configuration::from_str("Invalid"), None);
        assert_eq!(Configuration::from_str("headphones"), None); // case sensitive
        assert_eq!(
            Configuration::from_str("  Headphones  "),
            Some(Configuration::Headphones)
        ); // trimmed
    }

    #[test]
    fn test_sadie_019_entry() {
        // Test that the database contains the SADIE_019 entry with expected values
        let descriptions = Descriptions::new().expect("Failed to load descriptions database");

        // Check that SADIE_019 exists in the database
        let entry_rc = descriptions
            .get_rc("SADIE_019")
            .expect("SADIE_019 entry not found in database");

        // Verify all expected fields match the provided values
        assert_eq!(entry_rc.hrtf, "SADIE", "HRTF field mismatch");
        assert_eq!(
            entry_rc.configuration,
            Some(Configuration::Headphones),
            "Configuration mismatch"
        );
        assert_eq!(entry_rc.description, "Human subject", "Description mismatch");
        assert_eq!(entry_rc.points, Some(170), "Points mismatch (expected 170)");

        // Optional: Also verify that source and credits are not empty (if they should contain data)
        assert!(!entry_rc.source.is_empty(), "Source field should not be empty");
        assert!(
            !entry_rc.credits.is_empty(),
            "Credits field should not be empty"
        );
    }
}
