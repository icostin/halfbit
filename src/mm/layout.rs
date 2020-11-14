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

#[derive(PartialEq, Debug)]
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

    pub fn to_layout_for_array(&self, count: usize) -> Option<MemBlockLayout> {
        if count == 0 || self.size == 0 {
            Some(MemBlockLayout { size: 0usize, align: self.align })
        } else {
            let aligned_size = usize_align_up(self.size, self.align).unwrap();
            if count <= usize::MAX / aligned_size {
                Some(MemBlockLayout {
                    size: aligned_size * (count - 1) + self.size,
                    align: self.align
                })
            } else {
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct NonZeroMemBlockLayout {
    size: NonZeroUsize,
    align: Pow2Usize,
}

impl NonZeroMemBlockLayout {

    pub fn new(
        mbl: &MemBlockLayout
    ) -> Option<Self> {
        if mbl.is_zero_sized() {
            None
        } else {
            Some(NonZeroMemBlockLayout {
                size: NonZeroUsize::new(mbl.size).unwrap(),
                align: mbl.align,
            })
        }
    }

    pub fn from_type<T: Sized>() -> Self {
        NonZeroMemBlockLayout::new(&MemBlockLayout::from_type::<T>()).unwrap()
    }

    pub fn size(&self) -> NonZeroUsize {
        self.size
    }

    pub fn size_as_usize(&self) -> usize {
        self.size().get()
    }

    pub fn align(&self) -> Pow2Usize {
        self.align
    }

    pub fn align_as_usize(&self) -> usize {
        self.align().get()
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
        assert_eq!(n.size_as_usize(), l.size);
        assert_eq!(n.align_as_usize(), l.align.get());
    }

    #[test]
    fn zero_items_array_layout() {
        let l = MemBlockLayout::from_type::<u64>();
        assert_eq!(l.size, 8usize);
        let al = l.to_layout_for_array(0).unwrap();
        assert_eq!(al.size, 0usize);
    }

    #[test]
    fn zero_sized_layout_with_large_alignment_converts_to_zero_sized_array_layout() {
        let l = MemBlockLayout::new(0, 0x100000).unwrap();
        let al = l.to_layout_for_array(usize::MAX).unwrap();
        assert_eq!(al.align, l.align);
        assert_eq!(al.size, 0usize);
    }

    struct Toothy(u64, u64, u8);

    #[test]
    fn array_of_1_copies_layout() {
        let l = MemBlockLayout::from_type::<Toothy>();
        let al = l.to_layout_for_array(1usize).unwrap();
        assert_eq!(l, al);
    }


    #[test]
    fn unaligned_struct_size_is_multiplied_aligned_for_array_layout() {
        let l = MemBlockLayout::new(0x11, 8).unwrap();
        assert_eq!(l.align.get(), 8usize);
        assert_eq!(l.size, 17usize);
        let al = l.to_layout_for_array(5usize).unwrap();
        assert_eq!(al.align, l.align);
        assert_eq!(al.size, 0x71usize); // 24 bytes * 4 + 17
    }

    #[test]
    fn array_layout_at_max_size() {
        let l = MemBlockLayout::new(0xF, 1).unwrap();
        let al = l.to_layout_for_array(usize::MAX / 0xF).unwrap();
        assert_eq!(al.align, l.align);
        assert_eq!(al.size, usize::MAX);
    }
    #[test]
    fn array_layout_too_large_when_aligned() {
        let l = MemBlockLayout::new(0xF, 2).unwrap();
        assert!(l.to_layout_for_array(usize::MAX / 0xF).is_none());
    }

}
