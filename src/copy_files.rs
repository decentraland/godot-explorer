use std::{env, fs, io, path::Path};

use crate::{
    consts::{BIN_FOLDER, GODOT_PROJECT_FOLDER, RUST_LIB_PROJECT_FOLDER},
    path::adjust_canonicalization,
};

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

pub fn copy_library(
    target: &String,
    debug_mode: bool,
) -> Result<(), anyhow::Error> {
    let mode = if debug_mode { "debug" } else { "release" };

    match target.as_str() {
        "ios" => {
            let source_file = format!(
                "{RUST_LIB_PROJECT_FOLDER}target/aarch64-apple-ios/{mode}/libdclgodot.dylib"
            );
            let dest = format!(
                "{RUST_LIB_PROJECT_FOLDER}target/aarch64-apple-ios/libdclgodot.dylib"
            );

            copy_with_error_context(&source_file, &dest, false)?;

            // If you need ffmpeg for iOS specifically:
            // copy_ffmpeg_libraries(target, format!("{}ios/", GODOT_PROJECT_FOLDER), false)?;
        }

        "android" => {
            let source_file = format!(
                "{RUST_LIB_PROJECT_FOLDER}target/aarch64-linux-android/{mode}/libdclgodot.so"
            );

            let dest = format!(
                "{RUST_LIB_PROJECT_FOLDER}target/aarch64-linux-android/libdclgodot.so"
            );

            copy_with_error_context(&source_file, &dest, false)?;

            // If you need ffmpeg for Android specifically:
            // copy_ffmpeg_libraries(target, format!("{}android/arm64/", GODOT_PROJECT_FOLDER), false)?;
        }

        "win64" | "linux" | "macos" => {
            // For Windows, Linux, Mac we revert to the old logic:
            let file_name = match target.as_str() {
                "win64" => "dclgodot.dll",
                "linux" => "libdclgodot.so",
                "macos" => "libdclgodot.dylib",
                _ => unreachable!(), // already covered by the match above
            };

            let source_folder = format!("{RUST_LIB_PROJECT_FOLDER}target/{}/", mode);
            let source_path = adjust_canonicalization(
                std::fs::canonicalize(&source_folder)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to canonicalize source folder {}: {}",
                            source_folder,
                            e
                        )
                    })?
                    .join(file_name),
            );

            let lib_folder = format!("{RUST_LIB_PROJECT_FOLDER}target/");
            let destination_path = adjust_canonicalization(
                std::fs::canonicalize(&lib_folder)
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to canonicalize destination folder {}: {}",
                            lib_folder.to_string(),
                            e
                        )
                    })?
                    .join(file_name),
            );

            copy_if_modified(source_path.clone(), destination_path.clone(), false).map_err(
                |e| {
                    anyhow::anyhow!(
                        "Failed to copy from {:?} to {:?}: {}",
                        source_path,
                        destination_path,
                        e
                    )
                },
            )?;

            // If on Windows and debug mode, also copy PDB
            if debug_mode && target == "win64" {
                let pdb_name = "dclgodot.pdb";
                let pdb_source = adjust_canonicalization(
                    std::fs::canonicalize(&source_folder)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to canonicalize source folder {}: {}",
                                source_folder,
                                e
                            )
                        })?
                        .join(pdb_name),
                );
                let pdb_dest = adjust_canonicalization(
                    std::fs::canonicalize(&lib_folder)
                        .map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to canonicalize destination folder {}: {}",
                                lib_folder,
                                e
                            )
                        })?
                        .join(pdb_name),
                );

                copy_if_modified(pdb_source.clone(), pdb_dest.clone(), false).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to copy PDB from {:?} to {:?}: {}",
                        pdb_source,
                        pdb_dest,
                        e
                    )
                })?;
            }

            copy_ffmpeg_libraries(target, lib_folder.clone(), false).map_err(|e| {
                anyhow::anyhow!("Failed to copy FFmpeg libraries to {}: {}", lib_folder, e)
            })?;
        }

        other => return Err(anyhow::anyhow!("Unknown target: {}", other)),
    }

    Ok(())
}

/// A small helper to copy a file and provide better error messages.
fn copy_with_error_context(
    source: &str,
    destination: &str,
    link_libs: bool,
) -> Result<(), anyhow::Error> {
    // Ensure destination directory exists
    if let Some(parent) = std::path::Path::new(destination).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("Failed to create directory {}: {}", parent.display(), e)
        })?;
    }

    let source_path = std::fs::canonicalize(source)
        .map_err(|e| anyhow::anyhow!("Failed to canonicalize {}: {}", source, e))?;

    let dest_path = std::path::PathBuf::from(destination);

    copy_if_modified(source_path.clone(), dest_path.clone(), link_libs).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy from {:?} to {:?}: {}",
            source_path,
            dest_path,
            e
        )
    })?;

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
