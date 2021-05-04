use bumpalo::Bump;

pub struct BumpBitSet<'a> {
    level0: u64,
    level1: [u64; 64],
    level2: [Option<&'a mut [u64; 64]>; 64],
}

impl Default for BumpBitSet<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BumpBitSet<'a> {
    pub const UPPER_BOUND: u32 = 64 * 64 * 64;

    pub fn new() -> Self {
        BumpBitSet {
            level0: 0,
            level1: [0; 64],
            level2: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
        }
    }

    /// Returns first set bit index.
    pub fn find_set(&self) -> Option<u32> {
        match self.level0.trailing_zeros() {
            64 => None,
            i0 => {
                let i1 = unsafe {
                    debug_assert!(i0 < 64);
                    self.level1.get_unchecked(i0 as usize)
                }
                .trailing_zeros();

                let i2 = unsafe {
                    debug_assert!(i1 < 64);
                    let level2 = self.level2.get_unchecked(i0 as usize);
                    debug_assert!(level2.is_some());
                    match level2 {
                        Some(level2) => level2.get_unchecked(i1 as usize).trailing_zeros(),
                        None => core::hint::unreachable_unchecked(),
                    }
                };
                let result = (i0 << 12) + (i1 << 6) + i2;
                Some(result)
            }
        }
    }

    pub fn get(&mut self, index: u32) -> bool {
        let (i0, i1, i2) = Self::split_index(index);

        match &self.level2[usize::from(i0)] {
            Some(level2_array) => 0 < level2_array[usize::from(i1)] & (1 << i2),
            None => false,
        }
    }

    /// Sets specified bit.
    pub fn set(&mut self, index: u32, bump: &'a Bump) -> bool {
        let (i0, i1, i2) = Self::split_index(index);

        self.level0 |= 1 << i0;
        self.level1[usize::from(i0)] |= 1 << i1;
        match &mut self.level2[usize::from(i0)] {
            Some(level2_array) => {
                let level2 = &mut level2_array[usize::from(i1)];
                let old = (*level2) & (1 << i2);
                *level2 |= 1 << i2;
                old > 0
            }
            None => {
                let level2_array = bump.alloc([0; 64]);
                level2_array[usize::from(i1)] |= 1 << i2;
                self.level2[usize::from(i0)] = Some(level2_array);
                false
            }
        }
    }

    /// Unsets specified bit.
    pub fn unset(&mut self, index: u32) -> bool {
        let (i0, i1, i2) = Self::split_index(index);
        self.level0 &= !(1 << i0);
        self.level1[usize::from(i0)] &= !(1 << i1);
        if let Some(level2_array) = &mut self.level2[usize::from(i0)] {
            let level2 = &mut level2_array[usize::from(i1)];
            let old = (*level2) & (1 << i2);
            *level2 &= !(1 << i2);
            old > 0
        } else {
            false
        }
    }

    fn split_index(index: u32) -> (u8, u8, u8) {
        debug_assert!(
            Self::UPPER_BOUND > index,
            "`index` = {} must not exceed `64 ^ 3 - 1`",
            index
        );
        let i0 = index >> 12;
        let i1 = (index >> 6) & 63;
        let i2 = index & 63;

        (i0 as u8, i1 as u8, i2 as u8)
    }
}

pub struct BoxedBitSet {
    level0: u64,
    level1: [u64; 64],
    level2: [Option<Box<[u64; 64]>>; 64],
}

impl Default for BoxedBitSet {
    fn default() -> Self {
        Self::new()
    }
}

impl BoxedBitSet {
    pub const UPPER_BOUND: u32 = 64 * 64 * 64;

    pub const fn new() -> Self {
        BoxedBitSet {
            level0: 0,
            level1: [0; 64],
            level2: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
        }
    }

    /// Returns first set bit index.
    pub fn find_set(&self) -> Option<u32> {
        match self.level0.trailing_zeros() {
            64 => None,
            i0 => {
                let i1 = unsafe {
                    debug_assert!(i0 < 64);
                    self.level1.get_unchecked(i0 as usize)
                }
                .trailing_zeros();

                let i2 = unsafe {
                    debug_assert!(i1 < 64);
                    let level2 = self.level2.get_unchecked(i0 as usize);
                    debug_assert!(level2.is_some());
                    match level2 {
                        Some(level2) => level2.get_unchecked(i1 as usize).trailing_zeros(),
                        None => core::hint::unreachable_unchecked(),
                    }
                };
                let result = (i0 << 12) + (i1 << 6) + i2;
                Some(result)
            }
        }
    }

    pub fn get(&mut self, index: u32) -> bool {
        let (i0, i1, i2) = Self::split_index(index);

        match &self.level2[usize::from(i0)] {
            Some(level2_array) => 0 < level2_array[usize::from(i1)] & (1 << i2),
            None => false,
        }
    }

    /// Sets bit.
    pub fn set(&mut self, index: u32) -> bool {
        let (i0, i1, i2) = Self::split_index(index);

        self.level0 |= 1 << i0;
        self.level1[usize::from(i0)] |= 1 << i1;
        match &mut self.level2[usize::from(i0)] {
            Some(level2_array) => {
                let level2 = &mut level2_array[usize::from(i1)];
                let old = (*level2) & (1 << i2);
                *level2 |= 1 << i2;
                old > 0
            }
            None => {
                let mut level2_array = Box::new([0; 64]);
                level2_array[usize::from(i1)] |= 1 << i2;
                self.level2[usize::from(i0)] = Some(level2_array);
                false
            }
        }
    }

    /// Unsets bit.
    pub fn unset(&mut self, index: u32) -> bool {
        let (i0, i1, i2) = Self::split_index(index);
        self.level0 &= !(1 << i0);
        self.level1[usize::from(i0)] &= !(1 << i1);
        if let Some(level2_array) = &mut self.level2[usize::from(i0)] {
            let level2 = &mut level2_array[usize::from(i1)];
            let old = (*level2) & (1 << i2);
            *level2 &= !(1 << i2);
            old > 0
        } else {
            false
        }
    }

    fn split_index(index: u32) -> (u8, u8, u8) {
        debug_assert!(
            Self::UPPER_BOUND > index,
            "`index` = {} must not exceed `64 ^ 3 - 1`",
            index
        );
        let i0 = index >> 12;
        let i1 = (index >> 6) & 63;
        let i2 = index & 63;

        (i0 as u8, i1 as u8, i2 as u8)
    }
}
