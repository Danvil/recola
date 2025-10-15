use crate::collision::{PosBall3, Ray3, kernel::*};
use eyre::{Result, bail};
use glam::{Affine3A, Vec3};
use magi::geo::Aabb;

pub struct PosedCuboid {
    ref_t_cuboid: Affine3A,
    cuboid_t_ref: Affine3A,
    half_size: Vec3,
}

impl PosedCuboid {
    pub fn from_unit_cube_tf(ref_t_cuboid: Affine3A) -> Result<Self> {
        let Some((ref_t_cuboid, mut scale)) = decompose_transform_3(ref_t_cuboid) else {
            bail!("transform must not have shear: {ref_t_cuboid}");
        };

        if scale.cmplt(Vec3::ZERO).any() {
            log::warn!("negative transform scale: {scale}");
            scale = scale.abs();
        }

        Ok(Self::new(ref_t_cuboid, scale))
    }

    pub fn new(ref_t_cuboid: Affine3A, half_size: Vec3) -> Self {
        assert!(
            half_size.is_finite() && half_size.cmpge(Vec3::ZERO).all(),
            "invalid half_size: {half_size}"
        );

        // TODO assert this is a non-scale transformation
        let cuboid_t_ref = ref_t_cuboid.inverse();

        PosedCuboid {
            ref_t_cuboid,
            cuboid_t_ref,
            half_size,
        }
    }

    pub fn signed_distance_pos_ball(&self, pball: &PosBall3) -> f32 {
        aabb_signed_distance(
            self.half_size,
            self.cuboid_t_ref.transform_point3(pball.position),
        ) - pball.radius
    }

    pub fn closest_exit(&self, pball: &PosBall3) -> Option<Vec3> {
        aabb_closest_exit_point(
            self.half_size + pball.radius,
            self.cuboid_t_ref.transform_point3(pball.position),
        )
        .map(|exit| self.ref_t_cuboid.transform_point3(exit))
    }

    pub fn raycast(&self, ray: &Ray3, radius: f32) -> Option<(f32, Vec3)> {
        aabb_raycast(self.half_size + radius, ray.transform(&self.cuboid_t_ref))
            .map(|(lam, n)| (lam, self.ref_t_cuboid.transform_vector3(n)))
    }

    pub fn half_size(&self) -> Vec3 {
        self.half_size
    }

    pub fn ref_t_cuboid(&self) -> &Affine3A {
        &self.ref_t_cuboid
    }

    pub fn aabb(&self) -> Aabb<Vec3> {
        pub const CORNERS: [Vec3; 8] = [
            Vec3::new(1., 1., 1.),
            Vec3::new(1., 1., -1.),
            Vec3::new(1., -1., 1.),
            Vec3::new(1., -1., -1.),
            Vec3::new(-1., 1., 1.),
            Vec3::new(-1., 1., -1.),
            Vec3::new(-1., -1., 1.),
            Vec3::new(-1., -1., -1.),
        ];
        Aabb::from_points(
            CORNERS
                .iter()
                .map(|p| self.ref_t_cuboid.transform_point3(p * self.half_size)),
        )
    }
}
