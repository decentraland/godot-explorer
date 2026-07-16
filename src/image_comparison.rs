use image::{GenericImageView, Pixel};
use std::fs;
use std::path::{Path, PathBuf};

/// General snapshot-similarity pass floor (avatar/scene/client). Equivalent to the
/// historical `0.90` under the old (broken) normalization — see the note in
/// [`compare_images_similarity`]. Kept behavior-identical by re-expressing it on the
/// corrected 0..1 scale: `1 - (sqrt(765)/510) * (1 - 0.90)`.
pub const SNAPSHOT_SIMILARITY_MIN: f64 = 0.994577;
/// Stricter floor used by the manual `compare-image-folders` utility — the old `0.995`,
/// re-expressed on the corrected scale: `1 - (sqrt(765)/510) * (1 - 0.995)`.
pub const FOLDER_COMPARE_SIMILARITY_MIN: f64 = 0.999729;

// Function to compare two images and calculate similarity
pub fn compare_images_similarity(image_path_1: &Path, image_path_2: &Path) -> Result<f64, String> {
    compare_images_similarity_masked(image_path_1, image_path_2, None)
}

/// Like [`compare_images_similarity`], but if `mask_path` is `Some`, only pixels where the
/// mask is "on" (bright — luminance > 127) count toward the score. Black mask regions are
/// skipped, so dynamic on-screen content (e.g. a random avatar or name) is ignored during
/// the comparison. The mask must match the images' dimensions. A fully-masked-out image
/// (nothing to compare) scores 1.0.
pub fn compare_images_similarity_masked(
    image_path_1: &Path,
    image_path_2: &Path,
    mask_path: Option<&Path>,
) -> Result<f64, String> {
    let img1 = image::open(image_path_1)
        .map_err(|_| format!("Failed to open image: {:?}", image_path_1))?;
    let img2 = image::open(image_path_2)
        .map_err(|_| format!("Failed to open image: {:?}", image_path_2))?;

    if img1.dimensions() != img2.dimensions() {
        return Err("Images have different dimensions".to_string());
    }

    let (width, height) = img1.dimensions();
    if width == 0 || height == 0 {
        return Ok(1.0);
    }

    let mask = match mask_path {
        Some(p) => {
            let m = image::open(p).map_err(|_| format!("Failed to open mask: {:?}", p))?;
            if m.dimensions() != img1.dimensions() {
                return Err(format!("Mask {:?} dimensions differ from the images", p));
            }
            Some(m)
        }
        None => None,
    };

    let mut total_diff = 0.0;
    let mut counted = 0.0;
    // `image::open` yields a DynamicImage whose `get_pixel` always returns RGBA
    // (alpha = 255 for opaque sources), so every pixel has 4 channels regardless of the
    // file's on-disk color type. Derive the count from the same accessor the loop uses.
    let num_channels = img1.get_pixel(0, 0).channels().len() as f64;

    for y in 0..height {
        for x in 0..width {
            // Skip pixels the mask blacks out (dynamic content).
            if let Some(m) = &mask {
                let mp = m.get_pixel(x, y);
                let lum = (mp[0] as u16 + mp[1] as u16 + mp[2] as u16) / 3;
                if lum <= 127 {
                    continue;
                }
            }

            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);

            let diff = pixel1
                .channels()
                .iter()
                .zip(pixel2.channels().iter())
                .map(|(p1, p2)| (*p1 as f64 - *p2 as f64).powi(2))
                .sum::<f64>();

            total_diff += diff.sqrt();
            counted += 1.0;
        }
    }

    if counted == 0.0 {
        return Ok(1.0);
    }

    // Max possible per-pixel Euclidean distance across the channels is
    // sqrt(sum of 255^2) = 255 * sqrt(num_channels) (= 510 for RGBA). The previous
    // constant `sqrt(255*3)` ≈ 27.66 was ~18x too small, so `similarity` could run far
    // negative (e.g. -300%) for very different images; the metric is now bounded to 0..1
    // and a percentage reads intuitively (a 99%-identical frame reports ~99%).
    let max_diff_per_pixel = 255.0 * num_channels.sqrt();
    let average_diff = total_diff / counted;
    let similarity = 1.0 - (average_diff / max_diff_per_pixel);

    Ok(similarity.clamp(0.0, 1.0))
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

    let mut failed_files = vec![];

    // Compare each corresponding file
    for (snapshot_file, result_file) in snapshot_files.iter().zip(result_files.iter()) {
        let similarity = compare_images_similarity(snapshot_file, result_file)?;

        // If similarity is less than the `similarity_threshold`, the test fails
        if similarity < similarity_threshold {
            failed_files.push((snapshot_file, result_file));
        }

        println!(
            "Files {:?} and {:?} are {:.5}% similar.",
            snapshot_file,
            result_file,
            similarity * 100.0
        );
    }

    if !failed_files.is_empty() {
        return Err(format!("{} files are too different!", failed_files.len()));
    }

    println!(
        "All files match with {:.2}% similarity or higher!",
        similarity_threshold * 100.0
    );
    Ok(())
}
