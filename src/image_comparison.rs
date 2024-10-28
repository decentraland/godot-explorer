use image::{GenericImageView, Pixel};
use std::fs;
use std::path::{Path, PathBuf};

// Function to compare two images and calculate similarity
pub fn compare_images_similarity(image_path_1: &Path, image_path_2: &Path) -> Result<f64, String> {
    let img1 = image::open(image_path_1)
        .map_err(|_| format!("Failed to open image: {:?}", image_path_1))?;
    let img2 = image::open(image_path_2)
        .map_err(|_| format!("Failed to open image: {:?}", image_path_2))?;

    if img1.dimensions() != img2.dimensions() {
        return Err("Images have different dimensions".to_string());
    }

    let (width, height) = img1.dimensions();
    let mut total_diff = 0.0;
    let mut total_pixels = 0.0;

    for y in 0..height {
        for x in 0..width {
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);

            let diff = pixel1
                .channels()
                .iter()
                .zip(pixel2.channels().iter())
                .map(|(p1, p2)| (*p1 as f64 - *p2 as f64).powi(2))
                .sum::<f64>();

            total_diff += diff.sqrt();
            total_pixels += 1.0;
        }
    }

    let max_diff_per_pixel = (255.0_f64 * 3.0_f64).sqrt();
    let average_diff = total_diff / total_pixels;
    let similarity = 1.0 - (average_diff / max_diff_per_pixel);

    Ok(similarity)
}

// Function to list all PNG files in a directory
fn list_png_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = vec![];

    for entry in
        fs::read_dir(directory).map_err(|_| format!("Failed to read directory: {:?}", directory))?
    {
        let entry = entry.map_err(|_| "Failed to access entry in directory".to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("png") {
            files.push(path);
        }
    }

    files.sort(); // Ensure files are in the same order
    Ok(files)
}

// Function to compare all PNG files in two folders
pub fn compare_images_folders(
    snapshot_folder: &Path,
    result_folder: &Path,
    similarity_threshold: f64,
) -> Result<(), String> {
    let snapshot_files = list_png_files(snapshot_folder)?;
    let result_files = list_png_files(result_folder)?;

    // Ensure both folders have the same number of files
    if snapshot_files.len() != result_files.len() {
        return Err("Snapshot and result folders contain different numbers of files".to_string());
    }

    // Compare each corresponding file
    for (snapshot_file, result_file) in snapshot_files.iter().zip(result_files.iter()) {
        let similarity = compare_images_similarity(snapshot_file, result_file)?;

        // If similarity is less than the `similarity_threshold`, the test fails
        if similarity < similarity_threshold {
            return Err(format!(
                "Files {:?} and {:?} are too different! Similarity: {:.5}%",
                snapshot_file,
                result_file,
                similarity * 100.0
            ));
        }

        println!(
            "Files {:?} and {:?} are {:.5}% similar.",
            snapshot_file,
            result_file,
            similarity * 100.0
        );
    }

    println!("All files match with {:.2}% similarity or higher!", similarity_threshold * 100.0);
    Ok(())
}
