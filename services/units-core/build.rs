fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set build time
    println!("cargo:rustc-env=BUILD_TIME={}", chrono::Utc::now().to_rfc3339());
    
    // Set git commit if available
    if let Ok(output) = std::process::Command::new("git").args(&["rev-parse", "HEAD"]).output() {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_COMMIT={}", commit.trim());
        }
    }
    
    Ok(())
}