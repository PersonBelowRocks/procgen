use num_traits::{NumCast, ToPrimitive};

pub type IVec2 = na::Vector2<i32>;
pub type IVec3 = na::Vector3<i32>;

/// Casts a 3D vector of integer types (i32, u32, u16, usize, etc.) to a 3D vector of a different integer type.
#[allow(dead_code)]
#[inline]
pub fn cast_ivec3<T: NumCast, N: ToPrimitive + Copy>(v: na::Vector3<N>) -> Option<na::Vector3<T>> {
    let x = <T as NumCast>::from(v[0])?;
    let y = <T as NumCast>::from(v[1])?;
    let z = <T as NumCast>::from(v[2])?;

    Some(na::vector![x, y, z])
}

#[cfg(test)]
mod tests {

    use super::cast_ivec3;

    #[test]
    fn basic_casts() {
        let v1 = na::vector![500, 400, 300i32];

        let _ = cast_ivec3::<u16, i32>(v1).unwrap();
        let _ = cast_ivec3::<u32, i32>(v1).unwrap();
        let _ = cast_ivec3::<usize, i32>(v1).unwrap();
        let _ = cast_ivec3::<isize, i32>(v1).unwrap();
        let _ = cast_ivec3::<i16, i32>(v1).unwrap();
        let _ = cast_ivec3::<i64, i32>(v1).unwrap();
        let _ = cast_ivec3::<u64, i32>(v1).unwrap();
    }

    #[test]
    #[should_panic]
    fn casting_to_undersized_type() {
        let v1 = na::vector![500, 400, 300i32];

        // v1 contains numbers that cannot fit into 8 bits!
        let _ = cast_ivec3::<u8, i32>(v1).unwrap();
    }

    #[test]
    #[should_panic]
    fn casting_negative_number_to_unsigned_type() {
        let v1 = na::vector![-500, 300, 300i32];

        // v1 has -500 component, and u32 cannot hold negative numbers!
        let _ = cast_ivec3::<u32, i32>(v1).unwrap();
    }
}
