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

pub fn copy_library(debug_mode: bool, link_libs: bool) -> Result<(), anyhow::Error> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let file_name = match (os, arch) {
        ("linux", _) => Some("libdclgodot.so".to_string()),
        ("windows", _) => Some("dclgodot.dll".to_string()),
        ("macos", _) => Some("libdclgodot.dylib".to_string()),
        _ => None,
    }
    .expect("Couldn't find a library for this platform");

    let source_folder: &str = if debug_mode {
        "target/debug/"
    } else {
        "target/release/"
    };

    let source_folder = format!("{RUST_LIB_PROJECT_FOLDER}{source_folder}");

    let source_file =
        adjust_canonicalization(fs::canonicalize(source_folder.clone())?.join(file_name.clone()));

    let lib_folder = format!("{GODOT_PROJECT_FOLDER}lib/");
    let destination_file =
        adjust_canonicalization(fs::canonicalize(lib_folder.as_str())?.join(file_name.clone()));
    copy_if_modified(source_file, destination_file, link_libs)?;

    if debug_mode && os == "windows" {
        let source_file =
            adjust_canonicalization(fs::canonicalize(source_folder)?.join("dclgodot.pdb"));
        let destination_file =
            adjust_canonicalization(fs::canonicalize(lib_folder.as_str())?.join("dclgodot.pdb"));
        copy_if_modified(source_file, destination_file, link_libs)?;
    }

    copy_ffmpeg_libraries(lib_folder, link_libs)?;

    Ok(())
}

pub fn copy_ffmpeg_libraries(dest_folder: String, link_libs: bool) -> Result<(), anyhow::Error> {
    let os = env::consts::OS;
    if os == "windows" {
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
