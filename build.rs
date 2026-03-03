fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Compile ISLE files for examples
    if let Err(e) = intarsia_build::compile_isle_dir("examples/boolean_optimizer/isle") {
        println!(
            "cargo:warning=Failed to compile boolean_optimizer ISLE: {}",
            e
        );
    }

    if let Err(e) = intarsia_build::compile_isle_dir("examples/database_optimizer/isle") {
        println!(
            "cargo:warning=Failed to compile database_optimizer ISLE: {}",
            e
        );
    }

    if let Err(e) = intarsia_build::compile_isle_dir("examples/rewrite_explorer/isle") {
        println!(
            "cargo:warning=Failed to compile rewrite_explorer ISLE: {}",
            e
        );
    }
}
