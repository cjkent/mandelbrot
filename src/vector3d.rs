use std::ops::Add;
use std::ops::Sub;
use std::ops::Div;
use std::ops::Mul;
use colour::Colour;

// TODO are there type bounds so this could be defined for all numbers or all floating point numbers?
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3d {
    pub fn new(x: f64, y: f64, z: f64) -> Vector3d {
        Vector3d { x: x, y: y, z: z }
    }

    pub fn from_colour(col: &Colour) -> Vector3d {
        Vector3d::new(col.r as f64, col.g as f64, col.b as f64)
    }
}

impl Add for Vector3d {
    type Output = Vector3d;

    fn add(self, other: Vector3d) -> Vector3d {
        Vector3d { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl Sub for Vector3d {
    type Output = Vector3d;

    fn sub(self, other: Vector3d) -> Vector3d {
        Vector3d { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
}

impl Div<f64> for Vector3d {
    type Output = Vector3d;

    fn div(self, divisor: f64) -> Vector3d {
        Vector3d { x: self.x / divisor, y: self.y / divisor, z: self.z / divisor }
    }
}


impl Mul<f64> for Vector3d {
    type Output = Vector3d;

    fn mul(self, multiplier: f64) -> Vector3d {
        Vector3d { x: self.x * multiplier, y: self.y * multiplier, z: self.z * multiplier }
    }
}

//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div() {
        let v3d = Vector3d::new(3.0, 6.0, 9.0);
        assert_eq!(v3d / 3.0, Vector3d::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn mul() {
        let v3d = Vector3d::new(3.0, 6.0, 9.0);
        assert_eq!(v3d * 3.0, Vector3d::new(9.0, 18.0, 27.0));
    }

    #[test]
    fn sub() {
        let v3d1 = Vector3d::new(3.0, 6.0, 9.0);
        let v3d2 = Vector3d::new(1.0, 2.0, 3.0);
        assert_eq!(v3d1 - v3d2, Vector3d::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn add() {
        let v3d1 = Vector3d::new(3.0, 6.0, 9.0);
        let v3d2 = Vector3d::new(1.0, 2.0, 3.0);
        assert_eq!(v3d1 + v3d2, Vector3d::new(4.0, 8.0, 12.0));
    }
}
