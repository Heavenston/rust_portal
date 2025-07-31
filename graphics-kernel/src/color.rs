mod srgb;
pub use srgb::*;
mod linear_rgb;
pub use linear_rgb::*;

fn linear_to_srgb_transfer_function(val: f32) -> f32 {
    if val <= 0.0031308 {
        val * 12.92
    } else {
        1.055 * val.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_to_linear_transfer_function(val: f32) -> f32 {
    if val <= 0.04045 {
        val / 12.92
    } else {
        ((val + 0.055) / 1.055).powf(2.4)
    }
}

impl From<LinearRgb> for Srgb {
    fn from(linear: LinearRgb) -> Srgb {
        Srgb::from_array(linear.to_array().map(linear_to_srgb_transfer_function))
    }
}

impl From<Srgb> for LinearRgb {
    fn from(srgb: Srgb) -> LinearRgb {
        LinearRgb::from_array(srgb.to_array().map(srgb_to_linear_transfer_function))
    }
}

impl From<LinearRgba> for Srgba {
    fn from(linear: LinearRgba) -> Srgba {
        Srgba {
            r: linear_to_srgb_transfer_function(linear.r),
            g: linear_to_srgb_transfer_function(linear.g),
            b: linear_to_srgb_transfer_function(linear.b),
            a: linear.a,
        }
    }
}

impl From<Srgba> for LinearRgba {
    fn from(srgba: Srgba) -> LinearRgba {
        LinearRgba {
            r: linear_to_srgb_transfer_function(srgba.r),
            g: linear_to_srgb_transfer_function(srgba.g),
            b: linear_to_srgb_transfer_function(srgba.b),
            a: srgba.a,
        }
    }
}
