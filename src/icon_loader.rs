//! Icon loading functionality for IrateGoose

use eframe::egui::IconData;

/// Load application icon from embedded PNG data
pub fn load_icon() -> IconData {
    // Include the icon file at compile time
    // This will cause a compilation error if the file doesn't exist
    let icon_bytes = include_bytes!("../data/IrateGoose.png");
    
    // Use eframe's built-in PNG decoder for Wayland compatibility
    match eframe::icon_data::from_png_bytes(icon_bytes) {
        Ok(icon_data) => icon_data,
        Err(e) => {
            let err = format!("Failed to decode icon PNG: {}", e);
            log::error!("{err}");
            panic!("{err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_icon() {
        // Test that load_icon function doesn't panic and returns a valid IconData
        let icon_data = load_icon();
        
        // Verify that we got a valid IconData object
        // IconData has fields: rgba (Vec<u8>), width (u32), height (u32)
        assert!(!icon_data.rgba.is_empty(), "Icon should have non-empty RGBA data");
        assert!(icon_data.width > 0, "Icon should have positive width");
        assert!(icon_data.height > 0, "Icon should have positive height");
        
        // Verify RGBA data size matches dimensions (4 bytes per pixel: R, G, B, A)
        let expected_size = (icon_data.width * icon_data.height * 4) as usize;
        assert_eq!(
            icon_data.rgba.len(),
            expected_size,
            "RGBA data size should match width * height * 4"
        );
        
        println!(
            "Successfully loaded icon: {}x{} pixels, {} bytes",
            icon_data.width,
            icon_data.height,
            icon_data.rgba.len()
        );
    }
}