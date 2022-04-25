use num_traits::{NumCast, ToPrimitive};

#[inline]
pub fn cast_vec3<T: NumCast, N: ToPrimitive + Copy>(v: na::Vector3<N>) -> Option<na::Vector3<T>> {
    let x = <T as NumCast>::from(v[0])?;
    let y = <T as NumCast>::from(v[1])?;
    let z = <T as NumCast>::from(v[2])?;

    Some(na::vector![x, y, z])
}