use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/main.c");
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Only build for RISC-V if explicitly requested
    if env::var("CARGO_FEATURE_BUILD_RISCV").is_ok() {
        // RISC-V toolchain prefix
        let riscv_prefix = env::var("RISCV_PREFIX").unwrap_or_else(|_| "riscv64-unknown-elf".to_string());
        
        // Compile the C code for RISC-V
        let status = Command::new(format!("{}-gcc", riscv_prefix))
            .args(&[
                "-march=rv64imac",
                "-mabi=lp64",
                "-nostdlib",
                "-nostartfiles",
                "-ffreestanding",
                "-O2",
                "-Wall",
                "-Wextra",
                "-c",
                "src/main.c",
                "-o",
            ])
            .arg(out_dir.join("token_lifecycle.o"))
            .status()
            .expect("Failed to compile C code");
        
        if !status.success() {
            panic!("Failed to compile token lifecycle module");
        }
        
        // Link to create the final ELF
        let status = Command::new(format!("{}-ld", riscv_prefix))
            .args(&[
                "-T",
                "riscv_link.ld", // We'll need to create this
                "-o",
            ])
            .arg(out_dir.join("token_lifecycle.elf"))
            .arg(out_dir.join("token_lifecycle.o"))
            .status()
            .expect("Failed to link ELF");
        
        if !status.success() {
            panic!("Failed to link token lifecycle module");
        }
        
        // Convert to binary format
        let status = Command::new(format!("{}-objcopy", riscv_prefix))
            .args(&[
                "-O",
                "binary",
            ])
            .arg(out_dir.join("token_lifecycle.elf"))
            .arg(out_dir.join("token_lifecycle.bin"))
            .status()
            .expect("Failed to convert to binary");
        
        if !status.success() {
            panic!("Failed to convert token lifecycle module to binary");
        }
        
        println!("cargo:rustc-env=TOKEN_LIFECYCLE_BIN={}", out_dir.join("token_lifecycle.bin").display());
    }
}