# Convention: Test File Organisation

## Rule

All test files live in the `src/tests/` subdirectory. No `#[cfg(test)]` blocks or inline test
functions appear inside the module source files themselves.

## Linking Tests to Their Module

Each module that has tests declares its test module at the **bottom** of the source file using the
`#[path]` attribute so the compiler resolves the file relative to `src/`:

```rust
#[cfg(test)]
#[path = "tests/<module_name>_tests.rs"]
mod tests;
```

### Examples

| Module source file    | Test file                       | Declaration in source                    |
|-----------------------|---------------------------------|------------------------------------------|
| `src/main.rs`         | `src/tests/main_tests.rs`       | `#[path = "tests/main_tests.rs"]`        |
| `src/installed.rs`    | `src/tests/installed_tests.rs`  | `#[path = "tests/installed_tests.rs"]`   |

## Inside the Test File

Test files begin with `use super::*;` to pull in all items from the parent module under test,
matching normal Rust inline-test hygiene:

```rust
use super::*;

#[test]
fn example_test() {
    // ...
}
```

## Rationale

- Keeps module source files focused on implementation, free of test boilerplate.
- Allows test files to grow independently without cluttering the source file.
- All tests remain discoverable by `cargo test` without any additional configuration.
