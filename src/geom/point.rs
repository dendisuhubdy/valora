use lyon::math::TypedPoint2D;
use lyon::tessellation::FillVertex;
use rand::{Rand, Rng};
use std::ops::*;

#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<Point> for Point {
    type Output = Self;

    fn mul(self, rhs: Point) -> Self { Self { x: self.x * rhs.x, y: self.y * rhs.y } }
}

impl MulAssign<Point> for Point {
    fn mul_assign(&mut self, rhs: Point) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self { Self { x: self.x * rhs, y: self.y * rhs } }
}

impl MulAssign<f32> for Point {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Point {
    const WORLD_OFFSET: f32 = 1.0;
    const WORLD_FACTOR: f32 = 2.0;

    pub fn center() -> Point { Point { x: 0.5, y: 0.5 } }

    pub fn abs(self) -> Point { Point { x: self.x.abs(), y: self.y.abs() } }

    pub fn distance(self, point: Point) -> f32 { self.raw_distance(point).sqrt() }

    pub fn raw_distance(self, point: Point) -> f32 {
        let delta = (self - point).abs();
        delta.x.powi(2) + delta.y.powi(2)
    }

    pub fn manhattan(self, point: Point) -> f32 {
        (self.x - point.x).abs() + (self.y - point.y).abs()
    }

    // OpenGL places the origin in the center of the screen. We rescale
    // and offset vertices one world unit so the origin is in the bottom
    // left, and y and x point up and right respectively. If you think
    // it should be done differently, you are wrong.
    pub fn fix_coord(coord: f32) -> f32 { (coord * Point::WORLD_FACTOR) - Point::WORLD_OFFSET }

    fn restore_coord(coord: f32) -> f32 { (coord + Point::WORLD_OFFSET) / Point::WORLD_FACTOR }
}

impl Rand for Point {
    fn rand<R: Rng>(rng: &mut R) -> Point { Point { x: rng.next_f32(), y: rng.next_f32() } }
}

impl<U> Into<TypedPoint2D<f32, U>> for Point {
    fn into(self) -> TypedPoint2D<f32, U> {
        TypedPoint2D::new(Point::fix_coord(self.x), Point::fix_coord(self.y))
    }
}

impl<U> From<TypedPoint2D<f32, U>> for Point {
    fn from(point: TypedPoint2D<f32, U>) -> Point {
        Point { x: Point::restore_coord(point.x), y: Point::restore_coord(point.y) }
    }
}

impl From<FillVertex> for Point {
    fn from(point: FillVertex) -> Point { point.position.into() }
}