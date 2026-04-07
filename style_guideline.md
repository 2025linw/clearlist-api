# Code Style Guideline

## Module

```rust
//! # Math Module
//!
//! This module contains collection of functions to perform mathematics operations
```

## Structs/Enums

```rust
/// Complex Number Type
struct Complex {
    real: i64,
    imag: i64,
}
```

```rust
/// Number Type Representation
enum NumberType {
    Integer,
    Float,
    Complex
}
```

## Functions

```rust
/// Adds two numbers
///
/// # Arguments
///
/// * `left`: left number
/// * `right`: right number
///
/// # Returns
///
/// The sum of `left` and `right`
fn add(left: u64, right: u64) -> u64 {
    left + right
}
```
