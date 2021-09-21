use super::*;

pub fn run_orbit_test(
    orbit: Vec<f64>,
    grav_param: f64,
    at_time: f64,
    position: Point3<f64>,
    velocity: Vector3<f64>,
) {
    assert_eq!(
        orbit.len(),
        7,
        "orbit has {} parameters instead of 7",
        orbit.len()
    );
    panic!("run_orbit_test() not implemented");
}
