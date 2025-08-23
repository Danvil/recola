#[macro_export]
macro_rules! decimal_component {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(excess::prelude::Component, Debug, Clone, Copy)]
        pub struct $name(f64);

        impl $name {
            pub fn new(v: f64) -> Self {
                Self(v)
            }
        }

        impl std::ops::Deref for $name {
            type Target = f64;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl std::ops::Add<f64> for $name {
            type Output = f64;
            fn add(self, rhs: f64) -> Self::Output {
                self.0 + rhs
            }
        }

        impl std::ops::Sub<f64> for $name {
            type Output = f64;
            fn sub(self, rhs: f64) -> Self::Output {
                self.0 - rhs
            }
        }

        impl std::ops::Mul<f64> for $name {
            type Output = f64;
            fn mul(self, rhs: f64) -> Self::Output {
                self.0 * rhs
            }
        }

        impl std::ops::Div<f64> for $name {
            type Output = f64;
            fn div(self, rhs: f64) -> Self::Output {
                self.0 / rhs
            }
        }

        impl std::ops::AddAssign<f64> for $name {
            fn add_assign(&mut self, rhs: f64) {
                self.0 += rhs;
            }
        }

        impl std::ops::SubAssign<f64> for $name {
            fn sub_assign(&mut self, rhs: f64) {
                self.0 -= rhs;
            }
        }

        impl std::ops::MulAssign<f64> for $name {
            fn mul_assign(&mut self, rhs: f64) {
                self.0 *= rhs;
            }
        }

        impl std::ops::DivAssign<f64> for $name {
            fn div_assign(&mut self, rhs: f64) {
                self.0 /= rhs;
            }
        }

        impl std::ops::Add<$name> for f64 {
            type Output = f64;
            fn add(self, rhs: $name) -> Self::Output {
                self + rhs.0
            }
        }

        impl std::ops::Sub<$name> for f64 {
            type Output = f64;
            fn sub(self, rhs: $name) -> Self::Output {
                self - rhs.0
            }
        }

        impl std::ops::Mul<$name> for f64 {
            type Output = f64;
            fn mul(self, rhs: $name) -> Self::Output {
                self * rhs.0
            }
        }

        impl std::ops::Div<$name> for f64 {
            type Output = f64;
            fn div(self, rhs: $name) -> Self::Output {
                self / rhs.0
            }
        }

        impl std::ops::AddAssign<$name> for f64 {
            fn add_assign(&mut self, rhs: $name) {
                *self += *rhs;
            }
        }

        impl std::ops::SubAssign<$name> for f64 {
            fn sub_assign(&mut self, rhs: $name) {
                *self -= *rhs;
            }
        }

        impl std::ops::MulAssign<$name> for f64 {
            fn mul_assign(&mut self, rhs: $name) {
                *self *= *rhs;
            }
        }

        impl std::ops::DivAssign<$name> for f64 {
            fn div_assign(&mut self, rhs: $name) {
                *self /= *rhs;
            }
        }
    };
}

// #[macro_export]
// macro_rules! newtype_component {
//     ($(#[$meta:meta])* $name:ident, $inner:ty) => {
//         $(#[$meta])*
//         #[derive(flecs_ecs::prelude::Component, Clone, Copy)]
//         pub struct $name(pub $inner);

//         impl std::ops::Deref for $name {
//             type Target = $inner;

//             fn deref(&self) -> &Self::Target {
//                 &self.0
//             }
//         }
//     };
// }
