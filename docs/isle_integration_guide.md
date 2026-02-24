# ISLE Integration Guide

This guide explains how to integrate ISLE-generated code with the Kymetica optimizer framework.

## Overview

The Kymetica framework provides utilities to simplify ISLE (Instruction Selection and Lowering Expressions) integration:

1. **Build helpers** - Compile `.isle` files to Rust code in your `build.rs`
2. **Integration macros** - Generate boilerplate type definitions

## Step 1: Add Kymetica as Build Dependency

In your `Cargo.toml`:

```toml
[dependencies]
kymetica = "0.1"

[build-dependencies]
kymetica = { version = "0.1", features = ["build-helpers"] }
```

## Step 2: Create build.rs

Create a `build.rs` file in your project root:

```rust
fn main() {
    // Compile all .isle files in the isle/ directory
    kymetica::build::compile_isle_auto()
        .expect("Failed to compile ISLE files");
}
```

### Alternative: Custom Directory

If your ISLE files are in a custom location:

```rust
fn main() {
    kymetica::build::compile_isle_dir("my_rules")
        .expect("Failed to compile ISLE files");
}
```

### Alternative: Specific Files

To compile specific files:

```rust
fn main() {
    kymetica::build::compile_isle_files(&[
        "isle/rules.isle",
        "isle/cost.isle",
    ]).expect("Failed to compile ISLE files");
}
```

## Step 3: Directory Structure

Organize your project like this:

```
your_optimizer/
├── Cargo.toml
├── build.rs           # Compiles ISLE files
├── src/
│   ├── lib.rs         # or main.rs
│   └── isle/
│       ├── rules.isle # Your ISLE DSL file
│       └── rules.rs   # Generated (add to .gitignore)
```

## Step 4: Integrate in Your Code

In your Rust code where you want to use the ISLE-generated rules:

```rust
use kymetica::isle_integration;

// Declare the ISLE-generated module
#[allow(dead_code, unused_variables, unused_imports, non_snake_case)]
#[allow(irrefutable_let_patterns, unused_assignments, non_camel_case_types)]
#[allow(unreachable_patterns, unreachable_code)]
#[path = "isle/rules.rs"]
pub(crate) mod rules;

// Generate required type definitions
isle_integration!();
```

### Alternative: All-in-One Macro

Use `isle_integration_full!` for a more concise approach:

```rust
use kymetica::isle_integration_full;

isle_integration_full! {
    path: "isle/rules.rs",
}
```

### Custom Max Returns

If your rules generate many alternatives:

```rust
isle_integration!(max_returns: 200);
```

## Step 5: Use in ExplorerHooks

Implement the `ExplorerHooks` trait to call your ISLE rules:

```rust
use kymetica::{ExplorerHooks, OptimizerFramework};

// ... your optimizer type definition ...

impl ExplorerHooks for MyOptimizer {
    fn explore(&mut self, id: egg::Id) -> Vec<egg::Id> {
        let mut new_ids = Vec::new();
        rules::constructor_explore(self, id, &mut new_ids);
        new_ids
    }
}
```

## Step 6: Implement Context Trait

The ISLE-generated code expects a `Context` trait to be implemented for your optimizer.
This trait provides extractors and constructors for each term in your ISLE DSL.

See the database_optimizer example for a complete implementation.

## Complete Example

See `examples/database_optimizer/` for a full working example of ISLE integration.

## Troubleshooting

### Build errors about missing files

Make sure your `build.rs` runs before trying to use the generated code:
- Clean and rebuild: `cargo clean && cargo build`
- Check that the `.rs` files are generated in the correct location

### Cannot find types in ISLE-generated code

The generated code uses `use super::*;` so make sure all types referenced in your
ISLE file are in scope from the parent module.

### Inner attribute errors

The ISLE-generated code uses `#![...]` attributes. Make sure you're using `#[path]`
to declare the module, not `include!()`.

## API Reference

### Build Functions

```rust
// Compile ISLE files in isle/ directory
pub fn compile_isle_auto() -> Result<(), Box<dyn Error>>

// Compile ISLE files in specific directory
pub fn compile_isle_dir(dir: &str) -> Result<(), Box<dyn Error>>

// Compile specific ISLE file
pub fn compile_isle_file(file: &Path) -> Result<(), Box<dyn Error>>

// Compile multiple specific files
pub fn compile_isle_files(files: &[impl AsRef<Path>]) -> Result<(), Box<dyn Error>>
```

### Integration Macros

```rust
// Generate type definitions only
isle_integration!();
isle_integration!(max_returns: 200);

// Generate module declaration + type definitions
isle_integration_full! {
    path: "isle/rules.rs",
}
isle_integration_full! {
    path: "isle/rules.rs",
    max_returns: 200,
}
```
