use std::alloc::{self, Layout};
use std::mem;
use std::ptr::NonNull;
use std::slice;

pub trait Header: Copy + Sized {
    // Alignment must exceed Self::alignment, size includes header
    fn layout(&self) -> Layout;
}

pub trait PackableStruct {
    type Header: Header;

    fn header(&self) -> Self::Header;

    // This is responsible for `mem::forget`ting its innards and writing `header` to te
    // `buf` is guaranteed to satisfy the header's size and alignment requirements.
    fn pack(self, header: Self::Header, buf: &mut [u8]);

    // This takes back ownership to drop the innards.
    fn unpack(header: Self::Header, buf: &[u8]) -> Self;
}

#[repr(packed)]
pub struct PackedBox<T: PackableStruct> {
    ptr: NonNull<T::Header>,
}

impl<T: PackableStruct> PackedBox<T> {
    pub fn new(value: T) -> Self {
        let header = value.header();
        let layout = header.layout();
        let size = layout.size();
        let header_size = mem::size_of::<T::Header>();
        assert!(size >= header_size);

        let p = match NonNull::new(unsafe { alloc::alloc_zeroed(layout) }) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        unsafe {
            let slice = slice::from_raw_parts_mut(p.as_ptr(), size);
            value.pack(header, slice);
        }
        Self { ptr: p.cast() }
    }

    pub fn header(&self) -> T::Header {
        unsafe { *self.ptr.as_ptr() }
    }

    pub fn slice(&self) -> &[u8] {
        let layout = self.header().layout();
        unsafe {
            let slice_ptr = self.ptr.cast::<u8>();
            slice::from_raw_parts(slice_ptr.as_ptr(), layout.size())
        }
    }

    #[allow(unused)]
    pub fn slice_mut(&mut self) -> &mut [u8] {
        let layout = self.header().layout();
        unsafe {
            let slice_ptr = self.ptr.cast::<u8>();
            slice::from_raw_parts_mut(slice_ptr.as_ptr(), layout.size())
        }
    }

    pub fn unpack(self) -> T {
        let header = self.header();
        let layout = header.layout();
        let value = T::unpack(header, self.slice());

        unsafe { alloc::dealloc(self.ptr.as_ptr().cast(), layout) };
        mem::forget(self);
        value
    }
}

impl<T: PackableStruct> Drop for PackedBox<T> {
    fn drop(&mut self) {
        let header = self.header();
        let layout = header.layout();
        let value = T::unpack(header, self.slice());
        drop(value);
        unsafe {
            alloc::dealloc(self.ptr.as_ptr().cast(), layout);
        }
    }
}

// Here's a simple example where we use `PackedBox` to create a variant of
// `&str` that has a "thin" pointer instead of a "fat" one.  That is, we store
// the size of the allocation in a header at the beginning of the string instead
// of in the pointer itself.
#[cfg(test)]
mod tests {
    use super::{Header, PackableStruct, PackedBox};

    use std::alloc::Layout;
    use std::mem;

    #[derive(Clone, Copy)]
    pub struct TestHeader {
        len: usize,
    }

    impl Header for TestHeader {
        fn layout(&self) -> Layout {
            Layout::from_size_align(mem::size_of::<Self>() + self.len, mem::align_of::<Self>()).unwrap()
        }
    }

    impl PackableStruct for String {
        type Header = TestHeader;

        fn header(&self) -> TestHeader {
            TestHeader { len: self.len() }
        }

        fn pack(self, header: TestHeader, buf: &mut [u8]) {
            let hdr_len = mem::size_of::<usize>();
            unsafe { buf[..hdr_len].as_mut_ptr().cast::<usize>().write(header.len) };
            buf[hdr_len..].copy_from_slice(self.as_bytes());
        }

        fn unpack(header: TestHeader, buf: &[u8]) -> Self {
            let hdr_len = mem::size_of::<usize>();
            assert_eq!(buf[hdr_len..].len(), header.len);
            String::from_utf8(buf[hdr_len..].to_owned()).unwrap()
        }
    }

    #[test]
    fn test_thin_string() {
        let s = "hello there";

        let fat_string = String::from(s);
        let mut thin_string = PackedBox::new(fat_string);

        let hdr_len = mem::size_of::<usize>();
        assert_eq!(&thin_string.slice()[hdr_len..], s.as_bytes());
        assert_eq!(&thin_string.slice_mut()[hdr_len..], s.as_bytes());
        assert_eq!(&thin_string.unpack()[..], s);

        assert_eq!(mem::size_of::<PackedBox<String>>(), mem::size_of::<usize>());
    }
}
