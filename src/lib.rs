#![no_std]
#![feature(unsize)]
#![feature(unsized_tuple_coercion)]
#[macro_use]


extern crate num_derive;
extern crate num_traits;

pub mod num; // numeric types/operations

pub mod mm; // memory manager

pub mod error; // error types
pub use error::Error;

pub mod io; // input/output

pub mod exectx; // execution context
pub use exectx::ExecutionContext;
pub use exectx::LogLevel;

pub mod data_cell_v0;
pub use data_cell_v0::DataCell;

pub mod conv; // converters


pub fn lib_name() -> &'static str {
    "halfbit"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lib_name_compliant() {
        let n: &'static str = lib_name();
        assert!(n.contains("halfbit"));
    }

}

