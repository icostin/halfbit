#![no_std]

pub mod num; // numeric types/operations
pub mod mm; // memory manager
pub mod error; // error types
pub mod io; // input/output
pub mod exectx; // execution context

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

