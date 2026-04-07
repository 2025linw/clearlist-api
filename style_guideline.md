# Code Style Guideline

## Module

```rust
//! # Math
//!
//! `math` contains collection of functions to perform mathematics operations
```

## Structs/Enums

```rust
/// Complex number type
struct Complex {
    real: i64,
    imag: i64,
}
```

```rust
/// Number types used for identifying number formats
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
