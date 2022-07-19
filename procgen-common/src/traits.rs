use crate::IVec3;
use std::cmp::{max, min};

pub trait Bounds3D {
    fn min(&self) -> IVec3;
    fn max(&self) -> IVec3;

    fn contains(&self, pos: IVec3) -> bool {
        let min = self.min();
        let max = self.max();

        min.x <= pos.x
            && pos.x < max.x
            && min.y <= pos.y
            && pos.y < max.y
            && min.z <= pos.z
            && pos.z < max.z
    }
}

impl Bounds3D for std::ops::Range<IVec3> {
    fn min(&self) -> IVec3 {
        na::vector![
            min(self.start.x, self.end.x),
            min(self.start.y, self.end.y),
            min(self.start.z, self.end.z)
        ]
    }

    fn max(&self) -> IVec3 {
        na::vector![
            max(self.start.x, self.end.x),
            max(self.start.y, self.end.y),
            max(self.start.z, self.end.z)
        ]
    }
}
