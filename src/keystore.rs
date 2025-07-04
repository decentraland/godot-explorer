use crate::consts::BIN_FOLDER;
use crate::ui::{print_message, print_section, MessageType};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn generate_keystore(keystore_type: &str) -> Result<(), anyhow::Error> {
    print_section(&format!("Generating Android {} keystore", keystore_type));

    // Check if keytool is available
    if !crate::platform::check_command("keytool") {
        return Err(anyhow::anyhow!(
            "keytool not found. Please install Java JDK to generate Android keystores."
        ));
    }

    // Ensure .bin directory exists
    fs::create_dir_all(BIN_FOLDER)?;
    
    let keystore_filename = format!("{}.keystore", keystore_type);
    let keystore_path = format!("{}{}", BIN_FOLDER, keystore_filename);
    
    // Check if keystore already exists
    if Path::new(&keystore_path).exists() {
        print_message(
            MessageType::Warning,
            &format!("Keystore already exists at: {}", keystore_path),
        );
        print_message(MessageType::Info, "Delete it first if you want to regenerate.");
        return Ok(());
    }

    let alias = format!("android{}key", keystore_type);
    let keypass = "android";
    let storepass = "android";
    let dname = format!("CN=Android {},O=Android,C=US", keystore_type.to_uppercase());

    print_message(MessageType::Info, &format!("Creating keystore at: {}", keystore_path));

    let status = Command::new("keytool")
        .args(&[
            "-keyalg", "RSA",
            "-genkeypair",
            "-alias", &alias,
            "-keypass", keypass,
            "-keystore", &keystore_path,
            "-storepass", storepass,
            "-dname", &dname,
            "-validity", "9999",
            "-deststoretype", "pkcs12",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to generate keystore"));
    }

    print_message(
        MessageType::Success,
        &format!("Keystore generated successfully at: {}", keystore_path),
    );

    // Show environment variables to set
    print_section("Environment Variables");
    print_message(MessageType::Info, "Set these environment variables for Godot export:");
    
    let keystore_abs_path = std::fs::canonicalize(&keystore_path)?;
    let keystore_abs_path_str = keystore_abs_path.to_string_lossy();
    
    let env_prefix = format!("GODOT_ANDROID_KEYSTORE_{}", keystore_type.to_uppercase());
    println!("\nexport {}_PATH=\"{}\"", env_prefix, keystore_abs_path_str);
    println!("export {}_USER=\"{}\"", env_prefix, alias);
    println!("export {}_PASSWORD=\"{}\"", env_prefix, storepass);

    Ok(())
}