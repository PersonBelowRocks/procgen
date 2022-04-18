// TODO: reduce type complexity a lil so we can avoid typing X_SIZE, Y_SIZE, S_SIZE in every impl block...

/// 3D volume. Sort of a glorified 3D array with methods to index & access elements, stitch multiple volumes together, etc.
pub struct Volume<
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    >(VolumeStorage<T, X_SIZE, Y_SIZE, Z_SIZE>);

trait VolumeDimensions {
    const X_SIZE: usize;
    const Y_SIZE: usize;
    const Z_SIZE: usize;
}

/// 3D cubic volume. Dimensions of all axes are the same (i.e., a cube).
type CubicVolume<T, const SIZE: usize> = Volume<T, SIZE, SIZE, SIZE>;

/// Internal storage type for volumes. Alias for a 3D array.
type VolumeStorage<T, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize> = [[[T; Z_SIZE]; Y_SIZE]; X_SIZE];

pub use trait_impls::*;
mod trait_impls {
    use super::*;

    impl<
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > From<VolumeStorage<T, X_SIZE, Y_SIZE, Z_SIZE>> for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        #[inline]
        fn from(array: VolumeStorage<T, X_SIZE, Y_SIZE, Z_SIZE>) -> Self {
            Self(array)
        }
    }
    
    impl<
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > From<Volume<T, X_SIZE, Y_SIZE, Z_SIZE>> for VolumeStorage<T, X_SIZE, Y_SIZE, Z_SIZE> {
        #[inline]
        fn from(volume: Volume<T, X_SIZE, Y_SIZE, Z_SIZE>) -> Self {
            volume.0
        }
    }
    
    impl<
        T: Sized + Default + Copy, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Default for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        fn default() -> Self {
            Self([[[T::default(); Z_SIZE]; Y_SIZE]; X_SIZE])
        }
    }

    use num_traits::{PrimInt, NumCast};
    use std::ops::{Index, IndexMut};

    impl<
        N: PrimInt, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Index<[N; 3]> for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        type Output = T;

        fn index(&self, idx: [N; 3]) -> &Self::Output {
            let x = <usize as NumCast>::from(idx[0]).unwrap();
            let y = <usize as NumCast>::from(idx[1]).unwrap();
            let z = <usize as NumCast>::from(idx[2]).unwrap();

            &self.0[x][y][z]
        }
    }

    impl<
        N: PrimInt, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Index<na::Vector3<N>> for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        type Output = T;

        fn index(&self, idx: na::Vector3<N>) -> &Self::Output {
            &self[[idx[0], idx[1], idx[2]]]
        }
    }

    impl<
        N: PrimInt, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > IndexMut<[N; 3]> for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        fn index_mut(&mut self, idx: [N; 3]) -> &mut Self::Output {
            let x = <usize as NumCast>::from(idx[0]).unwrap();
            let y = <usize as NumCast>::from(idx[1]).unwrap();
            let z = <usize as NumCast>::from(idx[2]).unwrap();

            &mut self.0[x][y][z]
        }
    }

    impl<
        N: PrimInt, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > IndexMut<na::Vector3<N>> for Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        fn index_mut(&mut self, idx: na::Vector3<N>) -> &mut Self::Output {
            &mut self[[idx[0], idx[1], idx[2]]]
        }
    }
}

pub use iters::*;
mod iters {
    use super::Volume;

    pub struct VolumeIterator<
        'a, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > {
        pub volume: &'a Volume<T, X_SIZE, Y_SIZE, Z_SIZE>, 
        idx: [usize; 3]
    }

    pub struct VolumeIdxIterator< 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > {
        idx: [usize; 3]
    }

    impl<
        'a, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
        pub fn new(volume: &'a Volume<T, X_SIZE, Y_SIZE, Z_SIZE>) -> Self {
            Self {
                volume,
                idx: [0, 0, 0]
            }
        }
    }

    impl<
        'a, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Iterator for VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
        type Item = &'a T;

        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> {
            // Extract the element at the current index.
            let out = if self.idx[2] >= Z_SIZE {
                None
            } else {
                Some(&self.volume[self.idx])
            };
            
            // Increment the 'index vector' here. 
            // Code looks a bit ugly but it's the most clear and readable implementation I could come up with.
            self.idx[0] += 1;
            
            if self.idx[0] >= X_SIZE {

                self.idx[0] = 0;
                self.idx[1] += 1;
                
                if self.idx[1] >= Y_SIZE {
                    
                    self.idx[1] = 0;
                    self.idx[2] += 1;
                }
            }

            out
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let iterator_size = X_SIZE * Y_SIZE * Z_SIZE;
            (iterator_size, Some(iterator_size))
        }
    }

    impl<
        'a, 
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > ExactSizeIterator for VolumeIterator<'a, T, X_SIZE, Y_SIZE, Z_SIZE> {
        // Default implementations do everything we need here.
    }

    impl< 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Default for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
        fn default() -> Self {
            Self {
                idx: [0, 0, 0]
            }
        }
    }

    impl<
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Iterator for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
        type Item = na::Vector3<usize>;

        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> {
            
            // Extract the element at the current index.
            let out = if self.idx[2] >= Z_SIZE {
                None
            } else {
                Some(self.idx.into())
            };
            
            // Increment the 'index vector' here. 
            // Code looks a bit ugly but it's the most clear and readable implementation I could come up with.
            self.idx[0] += 1;
            
            if self.idx[0] >= X_SIZE {

                self.idx[0] = 0;
                self.idx[1] += 1;
                
                if self.idx[1] >= Y_SIZE {
                    
                    self.idx[1] = 0;
                    self.idx[2] += 1;
                }
            }

            out
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let iterator_size = X_SIZE * Y_SIZE * Z_SIZE;
            (iterator_size, Some(iterator_size))
        }
    }

    impl<
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > ExactSizeIterator for VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
        // Default implementations do everything we need here.
    }
}

pub use impls::*;
mod impls {

    use super::*;

    impl<
        T: Sized + Copy, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        /// Make new volume by cloning `value` into an array.
        pub fn new_filled(value: T) -> Self {
            Self([[[value; Z_SIZE]; Y_SIZE]; X_SIZE])
        }
    }

    impl<
        T: Sized, 
        const X_SIZE: usize, 
        const Y_SIZE: usize, 
        const Z_SIZE: usize
    > Volume<T, X_SIZE, Y_SIZE, Z_SIZE> {
        pub fn iter(&self) -> VolumeIterator<'_, T, X_SIZE, Y_SIZE, Z_SIZE> {
            VolumeIterator::new(self)
        }

        pub fn iter_indices(&self) -> VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
            Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constructor() {
        let volume: Volume<i32, 32, 32, 32> = Default::default();

        assert_eq!(volume[[3, 6, 10]], i32::default())
    }

    #[test]
    fn filled_constructor() {
        let volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);

        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let idx_vector = na::Vector3::new(x, y, z);

                    assert_eq!(volume[idx_vector], 42)
                }
            }
        }
        
    }

    #[test]
    fn indexing() {
        let mut volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);
        
        let idx = na::Vector3::new(10, 5, 22);

        assert_eq!(volume[idx], 42);

        volume[idx] = 100;

        assert_eq!(volume[idx], 100);
    }

    #[test]
    fn iterating() {
        let volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);

        for element in volume.iter() {
            assert_eq!(element, &42);
        }
    }

    #[test]
    fn iterating_indices() {
        let mut volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);

        for element in volume.iter() {
            assert_eq!(element, &42);
        }

        for idx in volume.iter_indices() {
            volume[idx] += 1;
        }

        for element in volume.iter() {
            assert_eq!(element, &43);
        }
    }

    #[test]
    fn iterator_size_hint() {
        let volume: Volume<i32, 10, 54, 3> = Default::default();

        let iterator = volume.iter();
        let (lower, upper) = iterator.size_hint();
        
        let mut count: usize = 0;
        for _ in iterator {
            count += 1;
        }

        assert_eq!(count, lower);
        assert_eq!(Some(count), upper);
    }

    #[test]
    fn idx_iterator_size_hint() {
        let volume: Volume<i32, 10, 54, 3> = Default::default();

        let iterator = volume.iter_indices();
        let (lower, upper) = iterator.size_hint();
        
        let mut count: usize = 0;
        for _ in iterator {
            count += 1;
        }

        assert_eq!(count, lower);
        assert_eq!(Some(count), upper);
    }
}
