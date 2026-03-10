//! Build-time helpers for compiling ISLE DSL files to Rust code.
//!
//! This crate provides utilities for [Cargo build scripts] to compile
//! [ISLE] (Instruction Selection Lowering Expressions) domain-specific language
//! files into Rust code. It uses the [`cranelift-isle`] compiler internally.
//!
//! [ISLE]: https://github.com/bytecodealliance/wasmtime/blob/main/cranelift/isle/docs/language-reference.md
//! [Cargo build scripts]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
//! [`cranelift-isle`]: https://docs.rs/cranelift-isle/
//!
//! # Example
//!
//! In your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! intarsia = "0.1"
//!
//! [build-dependencies]
//! intarsia-build = "*"
//! ```
//!
//! In your `build.rs`:
//! ```no_run
//! fn main() {
//!     intarsia_build::compile_isle_auto().unwrap();
//! }
//! ```

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Automatically discover and compile ISLE files in a conventional location.
///
/// This function looks for an `isle/` directory in the current directory
/// and compiles all `.isle` files found there. The generated Rust code
/// is written to the same `isle/` directory with a `.rs` extension.
///
/// This is a convenience wrapper around [`compile_isle_dir`] with a fixed path.
///
/// # Directory Structure
///
/// ```text
/// your_project/
///   ├── build.rs          (calls this function)
///   ├── Cargo.toml
///   └── src/
///       ├── main.rs
///       └── isle/
///         ├── rules.isle    (your ISLE file)
///         └── rules.rs      (generated - git ignore this)
/// ```
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     intarsia::build::compile_isle_auto().unwrap();
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The `isle/` directory doesn't exist
/// - No `.isle` files are found
/// - ISLE compilation fails (see [`cranelift_isle::error::Errors`])
/// - File I/O fails (see [`std::io::Error`])
///
/// [`cranelift_isle::error::Errors`]: https://docs.rs/cranelift-isle/latest/cranelift_isle/error/struct.Errors.html
pub fn compile_isle_auto() -> Result<(), Box<dyn Error>> {
    compile_isle_dir("src/isle")
}

/// Compile all ISLE files in a specified directory.
///
/// This function finds all `.isle` files in the given directory,
/// compiles them using the ISLE compiler from [`cranelift-isle`], and writes
/// the generated Rust code to `.rs` files in the same directory.
///
/// [`cranelift-isle`]: https://docs.rs/cranelift-isle/
///
/// # Arguments
///
/// * `isle_dir` - Path to directory containing `.isle` files (relative to current dir)
///
/// # Generated Files
///
/// For each `name.isle` file, generates `name.rs` in the same directory.
/// The generated files should be added to `.gitignore`.
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     // Compile ISLE files in examples/optimizer/isle/
///     intarsia_build::compile_isle_dir("examples/optimizer/isle").unwrap();
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The directory doesn't exist
/// - No `.isle` files are found
/// - ISLE compilation fails (see [`cranelift_isle::error::Errors`])
/// - File I/O fails (see [`std::io::Error`])
///
/// [`cranelift_isle::error::Errors`]: https://docs.rs/cranelift-isle/latest/cranelift_isle/error/struct.Errors.html
pub fn compile_isle_dir(isle_dir: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let isle_dir = isle_dir.as_ref();

    // Check if directory exists
    if !isle_dir.exists() {
        return Err(format!("ISLE directory not found: {}", isle_dir.display()).into());
    }

    if !isle_dir.is_dir() {
        return Err(format!("Not a directory: {}", isle_dir.display()).into());
    }

    // Find all .isle files in the directory
    let isle_files: Vec<PathBuf> = fs::read_dir(isle_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "isle" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if isle_files.is_empty() {
        println!(
            "cargo:warning=No .isle files found in {}",
            isle_dir.display()
        );
        return Ok(());
    }

    // Compile each .isle file
    for isle_file in isle_files {
        compile_isle_file(&isle_file)?;
    }

    Ok(())
}

/// Compile a single ISLE file to Rust code.
///
/// This is a lower-level function that compiles a single ISLE file using
/// [`cranelift_isle::compile::from_files`]. The generated Rust code is written
/// to a `.rs` file in the same directory as the input file.
///
/// [`cranelift_isle::compile::from_files`]: https://docs.rs/cranelift-isle/latest/cranelift_isle/compile/fn.from_files.html
///
/// # Arguments
///
/// * `isle_file` - Path to the `.isle` file to compile
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     intarsia_build::compile_isle_file("isle/rules.isle").unwrap();
///     intarsia_build::compile_isle_file("isle/custom.isle").unwrap();
/// }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist
/// - ISLE compilation fails (see [`cranelift_isle::error::Errors`])
/// - File I/O fails (see [`std::io::Error`])
///
/// [`cranelift_isle::error::Errors`]: https://docs.rs/cranelift-isle/latest/cranelift_isle/error/struct.Errors.html
pub fn compile_isle_file(isle_file: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let isle_file = isle_file.as_ref();

    // Validate input file
    if !isle_file.exists() {
        return Err(format!("ISLE file not found: {}", isle_file.display()).into());
    }

    let file_name = isle_file
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("Non-UTF8 file name")?;

    // Set up cargo rerun-if-changed
    println!("cargo:rerun-if-changed={}", isle_file.display());
    println!("cargo:warning=Compiling ISLE file: {}", file_name);

    // Output the generated Rust code to same directory with .rs extension
    let output_file = isle_file.with_extension("rs");

    // Compile the ISLE file
    let code =
        cranelift_isle::compile::from_files(vec![isle_file.to_path_buf()], &Default::default())
            .map_err(|e| format!("ISLE compilation failed for {}: {:?}", file_name, e))?;

    fs::write(&output_file, code)?;
    println!("cargo:warning=Generated: {}", output_file.display());

    Ok(())
}

/// Compile multiple specific ISLE files.
///
/// This is a convenience function for compiling a list of ISLE files.
/// It calls [`compile_isle_file`] for each file in the provided slice.
///
/// # Example
///
/// ```no_run
/// // build.rs
/// fn main() {
///     intarsia_build::compile_isle_files(&[
///         "isle/rules.isle",
///         "isle/cost.isle",
///     ]).unwrap();
/// }
/// ```
pub fn compile_isle_files(isle_files: &[impl AsRef<Path>]) -> Result<(), Box<dyn Error>> {
    for isle_file in isle_files {
        compile_isle_file(isle_file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_isle_dir_not_exists() {
        let result = compile_isle_dir("nonexistent_directory");
        assert!(result.is_err());
    }
}
