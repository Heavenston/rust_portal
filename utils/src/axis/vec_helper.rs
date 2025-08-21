use super::{ Axis };

use std::ops::{ Index, IndexMut };

pub trait AxisVecHelper: Index<Axis, Output = Self::Elem> + IndexMut<Axis> {
    type Elem;

    fn to_array(self) -> [Self::Elem; 3];

    fn max_axis(self) -> Axis
        where Self: Copy,
              Self::Elem: Copy + PartialOrd,
    {
        let arr = self.to_array();
        let mut max = arr[0];
        let mut axis = Axis::X;

        if arr[1] > max {
            max = arr[1];
            axis = Axis::Y;
        }

        if arr[2] > max {
            axis = Axis::Z;
        }

        axis
    }

    fn min_axis(self) -> Axis
        where Self: Copy,
              Self::Elem: Copy + PartialOrd,
    {
        let arr = self.to_array();
        let mut min = arr[0];
        let mut axis = Axis::X;

        if arr[1] < min {
            min = arr[1];
            axis = Axis::Y;
        }

        if arr[2] < min {
            axis = Axis::Z;
        }

        axis
    }
}

impl Index<Axis> for glam::Vec3 {
    type Output = f32;

    fn index(&self, index: Axis) -> &Self::Output {
        &self[index.idx()]
    }
}

impl IndexMut<Axis> for glam::Vec3 {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        &mut self[index.idx()]
    }
}

impl AxisVecHelper for glam::Vec3 {
    type Elem = f32;

    fn to_array(self) -> [Self::Elem; 3] {
        glam::Vec3::to_array(&self)
    }
}

impl Index<Axis> for glam::DVec3 {
    type Output = f64;

    fn index(&self, index: Axis) -> &Self::Output {
        &self[index.idx()]
    }
}

impl IndexMut<Axis> for glam::DVec3 {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        &mut self[index.idx()]
    }
}

impl AxisVecHelper for glam::DVec3 {
    type Elem = f64;

    fn to_array(self) -> [Self::Elem; 3] {
        glam::DVec3::to_array(&self)
    }
}

impl Index<Axis> for glam::IVec3 {
    type Output = i32;

    fn index(&self, index: Axis) -> &Self::Output {
        &self[index.idx()]
    }
}

impl IndexMut<Axis> for glam::IVec3 {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        &mut self[index.idx()]
    }
}

impl AxisVecHelper for glam::IVec3 {
    type Elem = i32;

    fn to_array(self) -> [Self::Elem; 3] {
        glam::IVec3::to_array(&self)
    }
}

impl Index<Axis> for glam::UVec3 {
    type Output = u32;

    fn index(&self, index: Axis) -> &Self::Output {
        &self[index.idx()]
    }
}

impl IndexMut<Axis> for glam::UVec3 {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        &mut self[index.idx()]
    }
}

impl AxisVecHelper for glam::UVec3 {
    type Elem = u32;

    fn to_array(self) -> [Self::Elem; 3] {
        glam::UVec3::to_array(&self)
    }
}
