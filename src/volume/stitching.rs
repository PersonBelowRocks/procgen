use super::{Volume, Axis};

/// Stitch 2 volumes together along an axis.
/// 
/// # Panics
/// 
/// This is best explained by an example.
/// Say you have 2 volumes:
/// ```should_panic
/// let v1: Volume<_, 8, 8, 8> = Volume::new_filled(50);
/// let v2: Volume<_, 7, 8, 9> = Volume::new_filled(60);
/// 
/// let stitched = stitch(v1, v2, Axis::Y); // fails
/// ```
/// Notice how v1's dimensions along the X and Z axis are both 8, but they're different in v2?
/// Think of volumes like boxes, and imagine that by stitching volumes together we're stacking these boxes.
/// This function will panic if a box doesn't "seamlessly" stack on top of another.
/// 
/// Visualized in 2D it looks like this:
/// 
/// This one's fine! | This one panics :(
///       ###        |      #####
///       ###        |      ##### <<< too wide!
///       ###        |       
///                  |       |
///        |         |       v
///        v         |
///                  |      ### <<< too thin!
///       ###        |      ###
///       ###        |      ###
///       ###        |      ###
///       ###        |
///       ###        |
/// 
/// This same thing happens in 3D which is why the function panics.
/// 
/// # Panics
/// 
/// If the capacity of the result volume differs from the actual capacity of the volume created by stitching the 2 arguments together.
pub fn stitch<
            T: Sized + Copy,

            const LHS_X_SIZE: usize,
            const LHS_Y_SIZE: usize,
            const LHS_Z_SIZE: usize,

            const RHS_X_SIZE: usize, 
            const RHS_Y_SIZE: usize, 
            const RHS_Z_SIZE: usize,

            const RESULT_X_SIZE: usize,
            const RESULT_Y_SIZE: usize,
            const RESULT_Z_SIZE: usize
        >(
            lhs: &Volume<T, LHS_X_SIZE, LHS_Y_SIZE, LHS_Z_SIZE>, 
            rhs: &Volume<T, RHS_X_SIZE, RHS_Y_SIZE, RHS_Z_SIZE>, 
            axis: Axis) -> Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE> {
            
            let required_capacity = RESULT_X_SIZE * RESULT_Y_SIZE * RESULT_Z_SIZE;

            match axis {
                Axis::X => {

                    assert_eq!(LHS_Y_SIZE, RHS_Y_SIZE);
                    assert_eq!(LHS_Z_SIZE, RHS_Z_SIZE);

                    let actual_capacity = (LHS_X_SIZE + RHS_X_SIZE) * LHS_Y_SIZE * LHS_Z_SIZE;
                    assert_eq!(required_capacity, actual_capacity);
                    
                    // Initialize a volume with a dummy value (point 0, 0, 0 of the LHS volume). We immediatelly fill this volume in with actual values, but Rust requires all arrays to be initialized (in safe code).
                    // The compiler will hopefully optimize the redundant copies away here, the alternative would be to use an unsafe block to make an array of uninitialized memory but that's not very good practice.
                    let mut out: Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE> = Volume::new_filled(lhs[[0, 0, 0usize]]);
                    
                    // The left hand side is going to be at the smallest X, and the right hand side is going to be at the furthest X.
                    for idx in lhs.iter_indices() {
                        out[idx] = lhs[idx];
                    }

                    // For the right hand side we just need to add the X size of the left hand side.
                    for raw_idx in rhs.iter_indices() {
                        let idx: na::Vector3<_> = [raw_idx[0] + LHS_X_SIZE, raw_idx[1], raw_idx[2]].into();
                        out[idx] = rhs[raw_idx];
                    }

                    out
                },
                Axis::Y => {

                    assert_eq!(LHS_X_SIZE, RHS_X_SIZE);
                    assert_eq!(LHS_Z_SIZE, RHS_Z_SIZE);

                    let actual_capacity = LHS_X_SIZE * (LHS_Y_SIZE + RHS_Y_SIZE) * LHS_Z_SIZE;
                    assert_eq!(required_capacity, actual_capacity);
                    
                    let mut out: Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE> = Volume::new_filled(lhs[[0, 0, 0usize]]);
                    
                    for idx in lhs.iter_indices() {
                        out[idx] = lhs[idx];
                    }

                    for raw_idx in rhs.iter_indices() {
                        let idx: na::Vector3<_> = [raw_idx[0], raw_idx[1] + LHS_Y_SIZE, raw_idx[2]].into();
                        out[idx] = rhs[raw_idx];
                    }

                    out
                },
                Axis::Z => {

                    assert_eq!(LHS_X_SIZE, RHS_X_SIZE);
                    assert_eq!(LHS_Y_SIZE, RHS_Y_SIZE);

                    let actual_capacity = LHS_X_SIZE * LHS_Y_SIZE * (LHS_Z_SIZE + RHS_Z_SIZE);
                    assert_eq!(required_capacity, actual_capacity);
                    
                    let mut out: Volume<T, RESULT_X_SIZE, RESULT_Y_SIZE, RESULT_Z_SIZE> = Volume::new_filled(lhs[[0, 0, 0usize]]);
                    
                    for idx in lhs.iter_indices() {
                        out[idx] = lhs[idx];
                    }

                    for raw_idx in rhs.iter_indices() {
                        let idx: na::Vector3<_> = [raw_idx[0], raw_idx[1], raw_idx[2] + LHS_Z_SIZE].into();
                        out[idx] = rhs[raw_idx];
                    }

                    out
                },
            }
        }