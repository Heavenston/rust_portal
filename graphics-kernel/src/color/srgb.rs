use crevice::std140::AsStd140;
use glam::{ Vec4, Vec3 };

#[derive(Debug, Clone, Copy, PartialEq, bytemuck::NoUninit, bytemuck::AnyBitPattern)]
#[repr(C)]
pub struct Srgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Srgb {
    pub const BLACK: Self = Self::new(0., 0., 0.);
    pub const WHITE: Self = Self::new(1., 1., 1.);
    pub const RED: Self = Self::new(1., 0., 0.);
    pub const GREEN: Self = Self::new(0., 1., 0.);
    pub const BLUE: Self = Self::new(0., 0., 1.);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
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

    pub fn with_alpha(self, a: f32) -> Srgba {
        Srgba { r: self.r, g: self.g, b: self.b, a }
    }
}

impl From<Srgb> for [f32; 3] {
    fn from(value: Srgb) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[f32; 3]> for Srgb {
    fn from(value: [f32; 3]) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[u8; 3]> for Srgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self {
            r: r as f32 / 255.,
            g: g as f32 / 255.,
            b: b as f32 / 255.,
        }
    }
}

impl From<Vec3> for Srgb {
    fn from(value: Vec3) -> Self {
        Srgb::from_array(value.to_array())
    }
}

impl From<Srgb> for Vec3 {
    fn from(value: Srgb) -> Self {
        Vec3::from_array(value.to_array())
    }
}

impl From<Srgba> for Srgb {
    fn from(rgb: Srgba) -> Self {
        Self {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

impl AsStd140 for Srgb {
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
pub struct Srgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Srgba {
    pub const BLACK_TRANSPARENT: Self = Self::new(0., 0., 0., 0.);
    pub const WHITE_TRANSPARENT: Self = Self::new(1., 1., 1., 0.);

    pub const BLACK: Self = Self::new(0., 0., 0., 1.);
    pub const WHITE: Self = Self::new(1., 1., 1., 1.);
    pub const RED: Self = Self::new(1., 0., 0., 1.);
    pub const GREEN: Self = Self::new(0., 1., 0., 1.);
    pub const BLUE: Self = Self::new(0., 0., 1., 1.);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(self) -> [f32; 4] {
        self.into()
    }

    pub fn from_array(array: [f32; 4]) -> Self {
        array.into()
    }

    pub fn rgb(self) -> Srgb {
        self.into()
    }
}

impl From<Srgba> for [f32; 4] {
    fn from(value: Srgba) -> Self {
        bytemuck::cast(value)
    }
}

impl From<[f32; 4]> for Srgba {
    fn from(value: [f32; 4]) -> Self {
        bytemuck::cast(value)
    }
}

impl From<Vec4> for Srgba {
    fn from(value: Vec4) -> Self {
        Srgba::from_array(value.to_array())
    }
}

impl From<Srgba> for Vec4 {
    fn from(value: Srgba) -> Self {
        Vec4::from_array(value.to_array())
    }
}

impl AsStd140 for Srgba {
    type Output = crevice::std140::Vec4;

    fn as_std140(&self) -> Self::Output {
        Vec4::from(*self).as_std140()
    }

    fn from_std140(val: Self::Output) -> Self {
        Vec4::from_std140(val).into()
    }
}

impl From<Srgba> for wgpu::Color {
    fn from(val: Srgba) -> Self {
        wgpu::Color {
            r: val.r.into(),
            g: val.g.into(),
            b: val.b.into(),
            a: val.a.into(),
        }
    }
}
