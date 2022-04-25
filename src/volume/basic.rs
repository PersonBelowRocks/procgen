// TODO: reduce type complexity a lil so we can avoid typing X_SIZE, Y_SIZE, Z_SIZE in every impl block...

use super::*;
use anyhow::Result;
use num_traits::{NumCast, PrimInt};

/// 3D volume. Sort of a glorified 3D array with methods to index & access elements, stitch multiple volumes together, etc.
#[derive(Clone)]
pub struct Volume<T: Sized, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize>(
    pub(super) VolumeStorage<T, X_SIZE, Y_SIZE, Z_SIZE>,
);

#[derive(Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// 3D cubic volume. Dimensions of all axes are the same (i.e., a cube).
pub type CubicVolume<T, const SIZE: usize> = Volume<T, SIZE, SIZE, SIZE>;

/// Internal storage type for volumes. Alias for a 3D array.
pub(super) type VolumeStorage<T, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize> =
    [[[T; Z_SIZE]; Y_SIZE]; X_SIZE];

impl<T: Sized + Copy, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize>
    Volume<T, X_SIZE, Y_SIZE, Z_SIZE>
{
    /// Make new volume by cloning `value` into an array.
    pub fn new_filled(value: T) -> Self {
        Self([[[value; Z_SIZE]; Y_SIZE]; X_SIZE])
    }

    #[inline]
    pub fn stitch_x<
        const RHS_X_SIZE: usize,
        const RHS_Y_SIZE: usize,
        const RHS_Z_SIZE: usize,
        const RESULT_X_SIZE: usize,
        const RESULT_Y_SIZE: usize,
        const RESULT_Z_SIZE: usize,
    >(
        &self,
        rhs: &Volume<T, RHS_X_SIZE, RHS_Y_SIZE, RHS_Z_SIZE>,
    ) -> Result<Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE>> {
        stitch(self, rhs, Axis::X)
    }

    #[inline]
    pub fn stitch_y<
        const RHS_X_SIZE: usize,
        const RHS_Y_SIZE: usize,
        const RHS_Z_SIZE: usize,
        const RESULT_X_SIZE: usize,
        const RESULT_Y_SIZE: usize,
        const RESULT_Z_SIZE: usize,
    >(
        &self,
        rhs: &Volume<T, RHS_X_SIZE, RHS_Y_SIZE, RHS_Z_SIZE>,
    ) -> Result<Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE>> {
        stitch(self, rhs, Axis::Y)
    }

    #[inline]
    pub fn stitch_z<
        const RHS_X_SIZE: usize,
        const RHS_Y_SIZE: usize,
        const RHS_Z_SIZE: usize,
        const RESULT_X_SIZE: usize,
        const RESULT_Y_SIZE: usize,
        const RESULT_Z_SIZE: usize,
    >(
        &self,
        rhs: &Volume<T, RHS_X_SIZE, RHS_Y_SIZE, RHS_Z_SIZE>,
    ) -> Result<Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE>> {
        stitch(self, rhs, Axis::Z)
    }
}

impl<T: Sized, const X_SIZE: usize, const Y_SIZE: usize, const Z_SIZE: usize>
    Volume<T, X_SIZE, Y_SIZE, Z_SIZE>
{
    pub fn iter(&self) -> VolumeIterator<'_, T, X_SIZE, Y_SIZE, Z_SIZE> {
        VolumeIterator::new(self)
    }

    pub fn iter_indices(&self) -> VolumeIdxIterator<X_SIZE, Y_SIZE, Z_SIZE> {
        Default::default()
    }

    /// The number of slots in this volume.
    #[inline]
    pub const fn capacity() -> usize {
        X_SIZE * Y_SIZE * Z_SIZE
    }

    /// Returns whether or not a vector is a valid index into this volume (i.e., not out of bounds).
    #[inline]
    pub fn within_bounds<N: PrimInt>(idx: na::Vector3<N>) -> bool {
        let x = <usize as NumCast>::from(idx[0]).unwrap();
        let y = <usize as NumCast>::from(idx[1]).unwrap();
        let z = <usize as NumCast>::from(idx[2]).unwrap();

        let no_undershot = 0 < x && 0 < y && 0 < z;
        let no_overshot = x < X_SIZE && y < Y_SIZE && z < Z_SIZE;

        no_undershot && no_overshot
    }

    /// Gets a borrow of the element at the specified index if the index is valid, if not then the function returns None
    #[inline]
    pub fn get<N: PrimInt>(&self, idx: na::Vector3<N>) -> Option<&T> {
        if Self::within_bounds(idx) {
            Some(&self[idx])
        } else {
            None
        }
    }

    /// Gets a mutable borrow of the element at the specified index if the index is valid, if not then the function returns None
    #[inline]
    pub fn get_mut<N: PrimInt>(&mut self, idx: na::Vector3<N>) -> Option<&mut T> {
        if Self::within_bounds(idx) {
            Some(&mut self[idx])
        } else {
            None
        }
    }

    /// Get a vector of this volume's dimensions (i.e., [X_SIZE, Y_SIZE, Z_SIZE]).
    #[inline]
    pub fn dimensions(&self) -> na::Vector3<usize> {
        na::vector![X_SIZE, Y_SIZE, Z_SIZE]
    }
}
