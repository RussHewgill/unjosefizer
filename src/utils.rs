use na::{ArrayStorage, Dyn, RawStorage};
use nalgebra as na;

pub fn deg_to_rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.
}

#[allow(unused_macros)]
macro_rules! print_matrix {
    ($m:expr) => {
        for row in $m.row_iter() {
            for x in row.iter() {
                eprint!("{: >6.3}  ", x)
            }
            eprintln!();
        }
    };
}
pub(crate) use print_matrix;

// pub fn print_matrix3(m: &na::Matrix3<f64>) {
//     for row in m.row_iter() {
//         for x in row.iter() {
//             eprint!("{: >6.3}  ", x)
//         }
//         eprintln!();
//     }
// }

// pub fn print_matrix4(m: &na::Matrix4<f64>) {
//     for row in m.row_iter() {
//         for x in row.iter() {
//             eprint!("{: >6.3}  ", x)
//         }
//         eprintln!();
//     }
// }
