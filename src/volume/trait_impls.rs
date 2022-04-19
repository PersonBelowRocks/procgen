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
use std::cmp::{Eq, PartialEq};

impl<T, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize> PartialEq for Volume<T, X_SIZE, Y_SIZE, Z_SIZE>
where
    T: Sized + PartialEq 
{
    fn eq(&self, rhs: &Self) -> bool {
        for idx in self.iter_indices() {
            if self[idx] != rhs[idx] {
                return false
            }
        }
        true
    }
}

impl<T, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize> Eq for Volume<T, X_SIZE, Y_SIZE, Z_SIZE>
where T: Sized + PartialEq {
    // Default implementation does everything we need.
}

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