use core::result::Result;

use crate::num::{
    Pow2Usize,
    usize_align_up,
};

#[derive(PartialEq, Debug)]
pub enum MemBlockLayoutError {
    InvalidAlignment, // alignment not power of 2
    SizeTooBig, // aligning the size overflows usize
}

#[derive(Debug)]
pub struct MemBlockLayout {
    size: usize,
    align: Pow2Usize,
}

impl MemBlockLayout {
    pub fn new(
        size: usize,
        align: usize
    ) -> Result<Self, MemBlockLayoutError> {
        let align = Pow2Usize::new(align);
        if align.is_none() {
            return Err(MemBlockLayoutError::InvalidAlignment);
        }
        let align = align.unwrap();
        if usize_align_up(size, align).is_none() {
            Err(MemBlockLayoutError::SizeTooBig)
        } else {
            Ok(MemBlockLayout { size, align })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sized_layout() {
        let l = MemBlockLayout::new(0, 1);
        assert!(l.is_ok());
    }

    #[test]
    fn bad_alignment_mem_layout() {
        let mbl = MemBlockLayout::new(usize::MAX, 3usize);
        assert!(mbl.is_err());
        assert_eq!(mbl.unwrap_err(), MemBlockLayoutError::InvalidAlignment);
    }

    #[test]
    fn size_too_big_mem_layout() {
        let mbl = MemBlockLayout::new(usize::MAX, 2usize);
        assert!(mbl.is_err());
        assert_eq!(mbl.unwrap_err(), MemBlockLayoutError::SizeTooBig);
    }
}
