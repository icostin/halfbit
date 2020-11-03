use core::result::Result;
use core::option::Option;
use core::mem;
use core::num::NonZeroUsize;

use crate::num::{
    Pow2Usize,
    usize_align_up,
};

#[derive(PartialEq, Debug)]
pub enum MemBlockLayoutError {
    InvalidAlignment, // alignment not power of 2
    AlignedSizeTooBig, // aligning the size overflows usize
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
            Err(MemBlockLayoutError::AlignedSizeTooBig)
        } else {
            Ok(MemBlockLayout { size, align })
        }
    }
    pub fn from_type<T>() -> MemBlockLayout {
        MemBlockLayout::new(mem::size_of::<T>(), mem::align_of::<T>()).unwrap()
    }
    pub fn is_zero_sized(&self) -> bool {
        self.size == 0
    }
    pub fn to_non_zero_layout(&self) -> Option<NonZeroMemBlockLayout> {
        NonZeroMemBlockLayout::new(&self)
    }
}

pub struct NonZeroMemBlockLayout {
    size: NonZeroUsize,
    align: Pow2Usize,
}

impl NonZeroMemBlockLayout {
    pub fn new(
        mbl: &MemBlockLayout
    ) -> Option<NonZeroMemBlockLayout> {
        if mbl.is_zero_sized() {
            None
        } else {
            Some(NonZeroMemBlockLayout {
                size: NonZeroUsize::new(mbl.size).unwrap(),
                align: mbl.align,
            })
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
        let l = l.unwrap();
        assert!(l.is_zero_sized());
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
        assert_eq!(mbl.unwrap_err(), MemBlockLayoutError::AlignedSizeTooBig);
    }

    #[test]
    fn non_zero_layout_from_zero_size() {
        let l = MemBlockLayout::new(0, 1).unwrap();
        assert!(l.to_non_zero_layout().is_none());
    }

    #[test]
    fn non_zero_layout_from_non_zero_size() {
        let l = MemBlockLayout::new(77, 16).unwrap();
        let n = l.to_non_zero_layout().unwrap();
        assert_eq!(n.size.get(), l.size);
        assert_eq!(n.align, l.align);
    }

}
