//! Minimal 3-vector helpers used inside the crate.

pub type Vec3 = [f64; 3];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, k: f64) -> Vec3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// Rotation about the z axis by `angle` (the frame rotates, the vector stays):
/// `x' = cos x + sin y`, `y' = -sin x + cos y`.
pub fn rot_z(a: Vec3, angle: f64) -> Vec3 {
    let (s, c) = angle.sin_cos();
    [c * a[0] + s * a[1], -s * a[0] + c * a[1], a[2]]
}

/// Angle between two vectors, radians in `[0, pi]`.
pub fn angle_between(a: Vec3, b: Vec3) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}
