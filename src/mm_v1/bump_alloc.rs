use core::cell::UnsafeCell;
use core::marker::PhantomData;

use crate::num::{
    NonZeroUsize,
    Pow2Usize,
};

use super::{
    NonNull,
    AllocError,
    Allocator,
};

struct BumpAllocatorState<'a> {
    begin_addr: usize,
    current_addr: usize,
    end_addr: usize,
    lifeline: PhantomData<&'a u8>,
}

pub struct BumpAllocator<'a> {
    state: UnsafeCell<BumpAllocatorState<'a>>
}

impl<'a> BumpAllocator<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        BumpAllocator {
            state: BumpAllocatorState {
                begin_addr: 0,
                current_addr: 0,
                end_addr: 0,
                lifeline: PhantomData
            }.into()
        }
    }
}

unsafe impl<'a> Allocator for BumpAllocator<'a> {

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_name_contains_bump() {
        let mut buffer = [0u8; 16];
        let a = BumpAllocator::new(&mut buffer);
        assert!(a.name().contains("bump"));
    }
}
