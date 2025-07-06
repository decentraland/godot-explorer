use crate::consts::BIN_FOLDER;
use crate::helpers::BinPaths;
use crate::ui::{print_message, print_section, MessageType};
use std::fs;
use std::process::Command;

pub fn generate_keystore(keystore_type: &str) -> Result<String, anyhow::Error> {
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
    let keystore_path = BinPaths::keystore(&keystore_filename);

    // Check if keystore already exists
    if keystore_path.exists() {
        print_message(
            MessageType::Info,
            &format!("Using existing {} keystore", keystore_type),
        );
        return Ok(keystore_path.to_string_lossy().to_string());
    }

    let alias = format!("android{}key", keystore_type);
    let keypass = "android";
    let storepass = "android";
    let dname = format!("CN=Android {},O=Android,C=US", keystore_type.to_uppercase());

    print_message(
        MessageType::Info,
        &format!("Creating keystore at: {}", keystore_path.display()),
    );

    let status = Command::new("keytool")
        .args([
            "-keyalg",
            "RSA",
            "-genkeypair",
            "-alias",
            &alias,
            "-keypass",
            keypass,
            "-keystore",
            keystore_path.to_str().unwrap(),
            "-storepass",
            storepass,
            "-dname",
            &dname,
            "-validity",
            "9999",
            "-deststoretype",
            "pkcs12",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to generate keystore"));
    }

    print_message(
        MessageType::Success,
        &format!(
            "Keystore generated successfully at: {}",
            keystore_path.display()
        ),
    );

    Ok(keystore_path.to_string_lossy().to_string())
}

/// Get keystore credentials for a given keystore type
pub fn get_keystore_credentials(keystore_type: &str) -> (String, String) {
    let alias = format!("android{}key", keystore_type);
    let password = "android";
    (alias, password.to_string())
}
