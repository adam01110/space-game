use crate::abilities::{ProjectileInterest, interested};

#[test]
fn interest_has_hysteresis_and_never_hides_owned_shots() {
    let policy = ProjectileInterest::default();
    assert!(interested(false, false, 4096.0_f32.powi(2), &policy));
    assert!(!interested(false, false, 4300.0_f32.powi(2), &policy));
    assert!(interested(true, false, 4300.0_f32.powi(2), &policy));
    assert!(!interested(true, false, 5000.0_f32.powi(2), &policy));
    assert!(interested(false, true, f32::INFINITY, &policy));
    assert!(!interested(false, false, f32::INFINITY, &policy));
}
