#![no_std]

pub mod num;
pub mod mm_v0; // memory manager v0
pub mod mm_v1; // memory manager v1
pub use mm_v1 as mm;

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

