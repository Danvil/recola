use crate::collision::Ray3;
use glam::{Affine3A, Mat3A, Vec3};
use magi::sdf::sd_box_n;

/// Signed distance between a point and an axis-aligned box
pub fn aabb_signed_distance(half_size: Vec3, point: Vec3) -> f32 {
    sd_box_n(point, half_size)
}

/// Intersects a ray with an axis-aligned box centered at the origin with half-extents `half_size`.
/// Returns the nearest non-negative hit distance and corresponding surface normal.
pub fn aabb_raycast(half_size: Vec3, ray: Ray3) -> Option<(f32, Vec3)> {
    // Reciprocal; inf handles zero components correctly for slab tests.
    let inv_dir = Vec3::ONE / ray.direction();

    // Parametric distances to the slabs on each axis.
    let t1 = (-half_size - ray.origin) * inv_dir;
    let t2 = (half_size - ray.origin) * inv_dir;

    // Entry is the maximum of the per-axis minima; exit is the minimum of the per-axis maxima.
    let t_min_v = t1.min(t2);
    let t_max_v = t1.max(t2);

    let t_enter = t_min_v.x.max(t_min_v.y).max(t_min_v.z);
    let t_exit = t_max_v.x.min(t_max_v.y).min(t_max_v.z);

    // No hit if the slabs miss or the box is entirely behind the ray.
    if t_enter > t_exit || t_exit < 0.0 {
        return None;
    }

    // If starting outside, return entry; if starting inside (t_enter < 0), return exit.
    // Determine which axis caused the entry intersection.
    let mut normal = Vec3::ZERO;
    let hit_t = if t_enter >= 0.0 {
        let nf = |a, u| {
            if a + u * t_enter > 0. { -1. } else { 1. }
        };

        if t_enter == t_min_v.x {
            normal.x = nf(ray.origin.x, ray.direction().x);
        } else if t_enter == t_min_v.y {
            normal.y = nf(ray.origin.y, ray.direction().y);
        } else {
            normal.z = nf(ray.origin.z, ray.direction().z);
        }
        t_enter
    } else {
        let nf = |a, u| {
            if a + u * t_exit > 0. { 1. } else { -1. }
        };

        // Ray starts inside: exit face normal.
        if t_exit == t_max_v.x {
            normal.x = nf(ray.origin.x, ray.direction().x);
        } else if t_exit == t_max_v.y {
            normal.y = nf(ray.origin.y, ray.direction().y);
        } else {
            normal.z = nf(ray.origin.z, ray.direction().z);
        }
        t_exit
    };

    Some((hit_t, normal))
}

/// Returns the closest exit point on the surface of an axis-aligned box centered at the origin
/// with half-extents `half_size`, for a point `p` that is inside or on the surface.
/// If `p` is outside, returns `None`.
pub fn aabb_closest_exit_point(half: Vec3, p: Vec3) -> Option<Vec3> {
    // Clearance to faces and signed face coordinates.
    let d = half - p.abs();

    // Axis with minimal clearance (deterministic tie-break: X > Y > Z).
    // Snap only that axis to its face.
    if d.x <= d.y && d.x <= d.z {
        if d.x >= 0. {
            return Some(p.with_x(half.x.copysign(p.x)));
        }
    } else if d.y <= d.z {
        if d.y >= 0. {
            return Some(p.with_y(half.y.copysign(p.y)));
        }
    } else {
        if d.z >= 0. {
            return Some(p.with_z(half.z.copysign(p.z)));
        }
    }

    None
}

/// Decompose Affine3A into translation, rotation and scale, and return the non-scale (R, T) and
/// the scale separated.
/// Returns None if there is shear (non-orthogonal axes) or degeneracy.
pub fn decompose_transform_3(tf: Affine3A) -> Option<(Affine3A, Vec3)> {
    const LEN_EPS: f32 = 1e-5; // reject near-zero axis lengths
    const ORTHO_EPS: f32 = 1e-4; // orthogonality tolerance

    // columns of linear part
    let x = tf.matrix3.x_axis;
    let y = tf.matrix3.y_axis;
    let z = tf.matrix3.z_axis;

    // scale (positive magnitudes)
    let sx = x.length();
    let sy = y.length();
    let sz = z.length();
    if sx < LEN_EPS || sy < LEN_EPS || sz < LEN_EPS {
        return None; // degenerate
    }

    // remove scale
    let nx = x / sx;
    let ny = y / sy;
    let nz = z / sz;

    // No-shear check: axes must be orthogonal
    if nx.dot(ny).abs() >= ORTHO_EPS {
        return None;
    }
    if nx.dot(nz).abs() >= ORTHO_EPS {
        return None;
    }
    if ny.dot(nz).abs() >= ORTHO_EPS {
        return None;
    }

    // Build rotation; enforce right-handedness by folding reflection into S
    let rotation = Mat3A::from_cols(nx, ny, nz);
    if rotation.determinant() < 0.0 {
        // Flip one axis and its scale to make det ≈ +1
        Some((
            Affine3A {
                matrix3: Mat3A::from_cols(nx, ny, -nz),
                translation: tf.translation,
            },
            Vec3::new(sx, sy, -sz),
        ))
    } else {
        Some((
            Affine3A {
                matrix3: rotation,
                translation: tf.translation,
            },
            Vec3::new(sx, sy, sz),
        ))
    }
}
