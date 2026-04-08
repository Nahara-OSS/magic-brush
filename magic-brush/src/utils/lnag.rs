//! Linear algebra module.
//!
//! The purpose of this module is to provide linear algebra utilities used across Nahara's Magic Brush.

use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::utils::lerp::Lerpable;

/// A two-component vector.
#[derive(Clone, Copy, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    #[inline]
    pub fn x(&self) -> f32 {
        self.0
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.1
    }
}

impl Add<Self> for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Vec2(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl Sub<Self> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec2(self.0 * rhs, self.1 * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Vec2(self.0 / rhs, self.1 / rhs)
    }
}

impl Lerpable for Vec2 {
    fn lerp(a: &Self, b: &Self, fraction: f32) -> Self {
        a.clone() * (1.0 - fraction) + b.clone() * fraction
    }
}

impl Vec2 {
    pub fn len(&self) -> f32 {
        (self.0 * self.0 + self.1 * self.1).sqrt()
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(value: Vec2) -> Self {
        [value.0, value.1]
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(value: [f32; 2]) -> Self {
        Vec2(value[0], value[1])
    }
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn expand_mut<T: Into<RectArg>>(&mut self, t: T) {
        match t.into() {
            RectArg::Vec2(Vec2(x, y)) => {
                self.min.0 = self.min.0.min(x);
                self.min.1 = self.min.1.min(y);
                self.max.0 = self.max.0.max(x);
                self.max.1 = self.max.1.max(y);
            }
            RectArg::Rect(Rect {
                min: Vec2(nx, ny),
                max: Vec2(mx, my),
            }) => {
                self.min.0 = self.min.0.min(nx);
                self.min.1 = self.min.1.min(ny);
                self.max.0 = self.max.0.max(mx);
                self.max.1 = self.max.1.max(my);
            }
        }
    }

    pub fn expand<T: Into<RectArg>>(&self, t: T) -> Self {
        let mut output = self.clone();
        output.expand_mut(t);
        output
    }

    pub fn intersect<T: Into<RectArg>>(&self, t: T) -> bool {
        match t.into() {
            RectArg::Vec2(Vec2(x, y)) => x >= self.min.0 && y >= self.min.1 && x <= self.max.0 && y <= self.max.1,
            RectArg::Rect(Rect {
                min: Vec2(nx, ny),
                max: Vec2(mx, my),
            }) => self.min.0 <= mx && self.max.0 >= nx && self.min.1 <= my && self.max.1 >= ny,
        }
    }
}

pub enum RectArg {
    Vec2(Vec2),
    Rect(Rect),
}

impl From<(f32, f32)> for RectArg {
    fn from(value: (f32, f32)) -> Self {
        Self::Vec2(Vec2(value.0, value.1))
    }
}

impl From<Rect> for RectArg {
    fn from(value: Rect) -> Self {
        Self::Rect(value)
    }
}
