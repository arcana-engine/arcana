use core::{fmt, iter::FusedIterator, marker::PhantomData, mem, ptr, slice};
use std::mem::ManuallyDrop;

use scoped_arena::Scope;

/// An iterator that moves out of a vector based on `Scope` iterator.
///
/// # Example
///
/// ```
/// let v = vec![0, 1, 2];
/// let iter = ScopedVecIter::new(v);
/// ```
pub struct ScopedVecIter<'a, T> {
    phantom: PhantomData<(fn(T) -> &'a mut T)>,
    ptr: *const T,
    end: *const T,
}

impl<'a, T> ScopedVecIter<'a, T> {
    /// Creates a consuming iterator, that is, one that moves each value out of
    /// the vector (from start to end). The vector cannot be used after calling
    /// this.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = vec!["a".to_string(), "b".to_string()];
    /// for s in ScopedVecIter::new(v) {
    ///     // s has type String, not &String
    ///     println!("{}", s);
    /// }
    /// ```
    #[inline]
    pub fn new(vec: Vec<T, &'a Scope<'_>>) -> Self {
        unsafe {
            let mut vec = ManuallyDrop::new(vec);
            let begin = vec.as_mut_ptr();
            let end = if mem::size_of::<T>() == 0 {
                (begin as *const i8).wrapping_offset(vec.len() as isize) as *const T
            } else {
                begin.add(vec.len()) as *const T
            };
            ScopedVecIter {
                phantom: PhantomData,
                ptr: begin,
                end,
            }
        }
    }
}

impl<'a, T: fmt::Debug> fmt::Debug for ScopedVecIter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScopedVecIter")
            .field(&self.as_slice())
            .finish()
    }
}

impl<'a, T> ScopedVecIter<'a, T> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let vec = vec!['a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// let _ = into_iter.next().unwrap();
    /// assert_eq!(into_iter.as_slice(), &['b', 'c']);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let vec = vec!['a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// into_iter.as_mut_slice()[2] = 'z';
    /// assert_eq!(into_iter.next().unwrap(), 'a');
    /// assert_eq!(into_iter.next().unwrap(), 'b');
    /// assert_eq!(into_iter.next().unwrap(), 'z');
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *self.as_raw_mut_slice() }
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr as *mut T, self.len())
    }
}

impl<'a, T> AsRef<[T]> for ScopedVecIter<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

unsafe impl<'a, T: Send + Send> Send for ScopedVecIter<'a, T> {}
unsafe impl<'a, T: Sync> Sync for ScopedVecIter<'a, T> {}

impl<'a, T> Iterator for ScopedVecIter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.ptr as *const _ == self.end {
            None
        } else if mem::size_of::<T>() == 0 {
            // purposefully don't use 'ptr.offset' because for
            // vectors with 0-size elements this would return the
            // same pointer.
            self.ptr = (self.ptr as *const i8).wrapping_offset(1) as *mut T;

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            let old = self.ptr;
            self.ptr = unsafe { self.ptr.offset(1) };

            Some(unsafe { ptr::read(old) })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = if mem::size_of::<T>() == 0 {
            (self.end as usize).wrapping_sub(self.ptr as usize)
        } else {
            unsafe { self.end.offset_from(self.ptr) as usize }
        };
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<'a, T> DoubleEndedIterator for ScopedVecIter<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.end == self.ptr {
            None
        } else if mem::size_of::<T>() == 0 {
            // See above for why 'ptr.offset' isn't used
            self.end = (self.end as *const i8).wrapping_offset(-1) as *mut T;

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            self.end = unsafe { self.end.offset(-1) };

            Some(unsafe { ptr::read(self.end) })
        }
    }
}

impl<'a, T> ExactSizeIterator for ScopedVecIter<'a, T> {}

impl<'a, T> FusedIterator for ScopedVecIter<'a, T> {}

impl<'a, T> Drop for ScopedVecIter<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.as_raw_mut_slice());
        }
    }
}
