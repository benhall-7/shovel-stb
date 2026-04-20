//! STB binary on-disk representation: shared layout types and read/write paths.
//!
//! The `layout` submodule holds `BinRead` / `BinWrite` structs shared by both
//! directions. The `read` and `write` submodules contain [`crate::Stb`]
//! serialization APIs; they are not re-exported from the parent `stb` module.

pub(crate) mod layout;

mod read;
mod write;
