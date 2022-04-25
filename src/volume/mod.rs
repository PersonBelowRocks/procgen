mod basic;
mod iters;
mod stitching;
mod trait_impls;

use basic::VolumeStorage;
pub use basic::{Axis, CubicVolume, Volume};
pub use iters::{VolumeIdxIterator, VolumeIterator};
pub use stitching::stitch;

#[cfg(test)]
mod tests {
    use crate::volume::stitching::StitchError;

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
                    let idx_vector = na::vector![x, y, z];

                    assert_eq!(volume[idx_vector], 42)
                }
            }
        }
    }

    #[test]
    fn indexing() {
        let volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);
        let idx = na::vector![10, 5, 22];

        assert_eq!(volume[idx], 42);
    }

    #[test]
    fn mutable_indexing() {
        let mut volume: Volume<i32, 32, 32, 32> = Volume::new_filled(42);
        let idx = na::vector![10, 5, 22];

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

    #[test]
    fn stitching() {
        let mut vol1: Volume<i32, 16, 10, 16> = Default::default();
        let mut vol2: Volume<i32, 16, 24, 16> = Default::default();

        let vol1_anomaly = na::vector![6, 6, 6usize];
        let vol2_anomaly = na::vector![9, 21, 10usize];

        vol1[vol1_anomaly] = 42;
        vol2[vol2_anomaly] = 64;

        let stitched_volume: Volume<i32, 16, 34, 16> = stitch(&vol1, &vol2, Axis::Y).unwrap();

        assert_eq!(stitched_volume[vol1_anomaly], 42);
        assert_eq!(
            stitched_volume[na::vector![0, 10, 0usize] + vol2_anomaly],
            64
        );
    }

    #[test]
    fn fallible_stitching() {
        use anyhow::Result;

        let vol1: Volume<i32, 16, 10, 16> = Default::default();
        let vol2: Volume<i32, 16, 16, 24> = Default::default();

        let stitched_volume: Result<Volume<i32, 16, 34, 16>> = stitch(&vol1, &vol2, Axis::Y);

        if stitched_volume.is_ok() {
            panic!()
        }

        let vol1: Volume<i32, 16, 10, 16> = Default::default();
        let vol2: Volume<i32, 16, 24, 16> = Default::default();

        let stitched_volume: Result<Volume<i32, 16, 80, 16>> = stitch(&vol1, &vol2, Axis::Y);

        if stitched_volume.is_ok() {
            panic!()
        }
    }

    #[test]
    fn fallible_indexing() {
        let volume: Volume<i32, 10, 10, 10> = Default::default();

        let valid_idx = na::vector![7, 7, 7];
        let invalid_idx = na::vector![16, 7, 7];

        assert_eq!(volume.get(valid_idx), Some(&i32::default()));
        assert_eq!(volume.get(invalid_idx), None);
    }

    #[test]
    fn mutable_fallible_indexing() {
        let mut volume: Volume<i32, 10, 10, 10> = Default::default();

        let valid_idx = na::vector![7, 7, 7];
        let invalid_idx = na::vector![16, 7, 7];

        assert_eq!(volume.get_mut(valid_idx), Some(&mut i32::default()));
        assert_eq!(volume.get_mut(invalid_idx), None);

        // Try to mutate here
        let slot = volume.get_mut(valid_idx).unwrap();
        *slot = 42;

        assert_eq!(volume.get(valid_idx), Some(&42))
    }

    #[test]
    fn equality() {
        let mut vol1: Volume<i32, 16, 24, 16> = Default::default();
        let mut vol2: Volume<i32, 16, 24, 16> = Default::default();
        let mut vol3: Volume<i32, 16, 24, 16> = Default::default();

        let anomaly = na::vector![9, 21, 10usize];

        vol1[anomaly] = 42;
        vol2[anomaly] = 64;
        vol3[anomaly] = 42;

        assert!(vol1 != vol2);
        assert!(vol2 != vol3);
        assert!(vol1 == vol3);

        vol2[anomaly] = 42;

        assert!(vol1 == vol2);
        assert!(vol2 == vol3);
        assert!(vol1 == vol3);
    }
}
