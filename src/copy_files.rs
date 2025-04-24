use std::{env, fs, io, path::Path};

use crate::consts::BIN_FOLDER;

pub fn copy_if_modified<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dest: Q,
    link: bool,
) -> io::Result<()> {
    let src_path = src.as_ref();
    let dest_path = dest.as_ref();

    // Obtain the metadata of the source and destination file
    let metadata_src = fs::metadata(src_path);
    let metadata_dest = fs::metadata(dest_path);

    // If both files exist, we compare their modification times
    if metadata_src.is_ok() && metadata_dest.is_ok() {
        let time_src = metadata_src?.modified()?;
        let time_dest = metadata_dest?.modified()?;

        // If the destination file is more recent or equal to the source file, we do not copy
        if time_dest >= time_src {
            println!("Skip copy, equal file {}", dest_path.to_string_lossy());
            return Ok(());
        }
    }

    // If the destination file does not exist or is older, we copy the source file to the destination
    // Only linux: If link=true, link the file instead of copying
    if link && env::consts::OS == "linux" {
        if dest_path.exists() {
            fs::remove_file(dest_path)
                .map(|_| println!("Remove {}", dest_path.to_string_lossy()))?;
        }
        fs::hard_link(src_path, dest_path)
            .map(|_| println!("Link {}", dest_path.to_string_lossy()))?;
    } else {
        fs::copy(src_path, dest_path)
            .map(|_| println!("Copying {}", dest_path.to_string_lossy()))?;
    }
    Ok(())
}

pub fn copy_ffmpeg_libraries(
    target: &String,
    dest_folder: String,
    link_libs: bool,
) -> Result<(), anyhow::Error> {
    if target == "win64" {
        // copy ffmpeg .dll
        let ffmpeg_dll_folder = format!("{BIN_FOLDER}ffmpeg/ffmpeg-6.0-full_build-shared/bin");

        // copy all dlls in ffmpeg_dll_folder to exports folder
        for entry in fs::read_dir(ffmpeg_dll_folder)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_file() {
                let file_name = entry.file_name().to_str().unwrap().to_string();

                if file_name.ends_with(".dll") {
                    let dest_path = format!("{dest_folder}{file_name}");
                    copy_if_modified(entry.path(), dest_path, link_libs)?;
                }
            }
        }
    }
    Ok(())
}

// Function to move the directory and its contents recursively
pub fn move_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    // Check if destination exists, create it if it doesn't
    if !dest.exists() {
        fs::create_dir_all(dest)?; // Create the destination directory
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if path.is_dir() {
            // Recursively move subdirectory
            move_dir_recursive(&path, &dest_path)?;
        } else {
            // Move file
            fs::rename(&path, &dest_path)?; // Move the file
        }
    }

    // Remove the source directory after moving all contents
    fs::remove_dir_all(src)?;

    Ok(())
}
