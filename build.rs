fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Compile ISLE files for examples
    if let Err(e) = intarsia_build::compile_isle_dir("examples/boolean-optimizer/isle") {
        println!(
            "cargo:warning=Failed to compile boolean-optimizer ISLE: {}",
            e
        );
    }

    if let Err(e) = intarsia_build::compile_isle_dir("examples/database-optimizer/isle") {
        println!(
            "cargo:warning=Failed to compile database-optimizer ISLE: {}",
            e
        );
    }

    if let Err(e) = intarsia_build::compile_isle_dir("experiments/math-bench/isle") {
        println!("cargo:warning=Failed to compile math-bench ISLE: {}", e);
    }
}
