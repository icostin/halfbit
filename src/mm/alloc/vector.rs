use super::AllocatorRef;

#[derive(Debug)]
pub struct Vector<'a, T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    allocator: AllocatorRef<'a>
}

impl<'a, T> Vector<'a, T> {

    pub fn new(allocator: AllocatorRef<'a>) -> Vector<'a, T> {
        Vector {
            ptr: core::ptr::null_mut::<T>(),
            len: 0,
            cap: 0,
            allocator: allocator
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn new_produces_an_empty_vector() {
        let n = NullRawAllocator::new();
        let v: Vector<'_, u8> = Vector::new(n.to_ref());
        assert!(v.is_empty());
        assert_eq!(v.len(), 0usize);
    }


}

