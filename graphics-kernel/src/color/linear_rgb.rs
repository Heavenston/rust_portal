use crevice::std140::AsStd140;
use glam::{ Vec4, Vec3 };

#[derive(Debug, Clone, Copy, PartialEq, bytemuck::NoUninit, bytemuck::AnyBitPattern)]
#[repr(C)]
pub struct LinearRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl LinearRgb {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn to_array(self) -> [f32; 3] {
        self.into()
    }

    pub fn from_array(array: [f32; 3]) -> Self {
        array.into()
    }

    pub fn from_array_u8(array: [u8; 3]) -> Self {
        array.into()
    }

    pub fn with_alpha(self, a: f32) -> LinearRgba {
        LinearRgba { r: self.r, g: self.g, b: self.b, a }
    }
}

impl From<LinearRgb> for [f32; 3] {
    fn from(value: LinearRgb) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[f32; 3]> for LinearRgb {
    fn from(value: [f32; 3]) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[u8; 3]> for LinearRgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
        }
    }
}

impl From<Vec3> for LinearRgb {
    fn from(value: Vec3) -> Self {
        LinearRgb::from_array(value.to_array())
    }
}

impl From<LinearRgb> for Vec3 {
    fn from(value: LinearRgb) -> Self {
        Vec3::from_array(value.to_array())
    }
}

impl From<LinearRgba> for LinearRgb {
    fn from(rgb: LinearRgba) -> Self {
        Self {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

impl AsStd140 for LinearRgb {
    type Output = crevice::std140::Vec3;

    fn as_std140(&self) -> Self::Output {
        Vec3::from(*self).as_std140()
    }

    fn from_std140(val: Self::Output) -> Self {
        Vec3::from_std140(val).into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, bytemuck::NoUninit, bytemuck::AnyBitPattern)]
#[repr(C)]
pub struct LinearRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl LinearRgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(self) -> [f32; 4] {
        self.into()
    }

    pub fn from_array(array: [f32; 4]) -> Self {
        array.into()
    }

    pub fn rgb(self) -> LinearRgb {
        self.into()
    }
}

impl From<LinearRgba> for [f32; 4] {
    fn from(value: LinearRgba) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[f32; 4]> for LinearRgba {
    fn from(value: [f32; 4]) -> Self {
        bytemuck::cast(value)
    }
}

impl From<Vec4> for LinearRgba {
    fn from(value: Vec4) -> Self {
        LinearRgba::from_array(value.to_array())
    }
}

impl From<LinearRgba> for Vec4 {
    fn from(value: LinearRgba) -> Self {
        Vec4::from_array(value.to_array())
    }
}

impl AsStd140 for LinearRgba {
    type Output = crevice::std140::Vec4;

    fn as_std140(&self) -> Self::Output {
        Vec4::from(*self).as_std140()
    }

    fn from_std140(val: Self::Output) -> Self {
        Vec4::from_std140(val).into()
    }
}


