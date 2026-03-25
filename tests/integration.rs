use goonj::material::AcousticMaterial;
use goonj::resonance;
use goonj::room::AcousticRoom;
use goonj::*;
use hisab::Vec3;

#[test]
fn shoebox_room_sabine_rt60() {
    let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
    let vol = room.geometry.volume_shoebox();
    let abs = room.geometry.total_absorption();
    let rt60 = impulse::sabine_rt60(vol, abs);
    // Carpet is absorptive, RT60 should be short (~0.3-0.5s)
    assert!(
        rt60 > 0.1 && rt60 < 2.0,
        "carpet room RT60 should be moderate, got {rt60}"
    );
}

#[test]
fn concrete_longer_than_carpet() {
    let concrete = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
    let carpet = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());

    let rt60_concrete = impulse::sabine_rt60(
        concrete.geometry.volume_shoebox(),
        concrete.geometry.total_absorption(),
    );
    let rt60_carpet = impulse::sabine_rt60(
        carpet.geometry.volume_shoebox(),
        carpet.geometry.total_absorption(),
    );

    assert!(
        rt60_concrete > rt60_carpet,
        "concrete should be more reverberant than carpet"
    );
}

#[test]
fn speed_and_doppler_consistency() {
    let c = propagation::speed_of_sound(20.0);
    // Stationary → no shift
    let f = propagation::doppler_shift(440.0, 0.0, 0.0, c);
    assert!((f - 440.0).abs() < 0.01);
}

#[test]
fn schroeder_frequency_shoebox() {
    let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
    let vol = room.geometry.volume_shoebox();
    let abs = room.geometry.total_absorption();
    let rt60 = impulse::sabine_rt60(vol, abs);
    let fs = resonance::schroeder_frequency(rt60, vol);
    // For a reverberant concrete room, Schroeder frequency should be in the hundreds of Hz
    assert!(
        fs > 50.0 && fs < 1000.0,
        "Schroeder frequency should be reasonable, got {fs}"
    );
}

#[test]
fn ray_traces_through_shoebox() {
    use goonj::ray::{AcousticRay, RayHit, ray_wall_intersection, reflect_ray};

    let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
    let mut ray = AcousticRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z, 1000.0);

    let mut bounces = 0;
    let mut last_wall: Option<usize> = None;
    while ray.is_alive() && bounces < 50 {
        let mut closest: Option<(f32, usize)> = None;
        for (i, wall) in room.geometry.walls.iter().enumerate() {
            // Skip the wall we just bounced off to avoid self-intersection
            if last_wall == Some(i) {
                continue;
            }
            if let Some(t) = ray_wall_intersection(&ray, wall)
                && (closest.is_none() || t < closest.unwrap().0)
            {
                closest = Some((t, i));
            }
        }
        let Some((t, idx)) = closest else { break };
        let wall = &room.geometry.walls[idx];
        let hit = RayHit {
            point: ray.origin + ray.direction * t,
            normal: wall.normal,
            distance: t,
            wall_index: idx,
        };
        ray = reflect_ray(
            &ray,
            &hit,
            wall.material.average_absorption(),
            wall.material.scattering,
        );
        last_wall = Some(idx);
        bounces += 1;
    }
    assert!(
        bounces > 5,
        "ray should bounce many times in concrete room, got {bounces}"
    );
    assert!(ray.energy < 1.0, "ray should lose energy from reflections");
}

#[test]
fn diffraction_occlusion_through_wall() {
    use goonj::diffraction::is_occluded;
    use goonj::room::Wall;

    // Place a wall between source and listener (vertices wound CCW when viewed from -X)
    let wall = Wall {
        vertices: vec![
            Vec3::new(5.0, -5.0, 5.0),
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(5.0, 5.0, -5.0),
            Vec3::new(5.0, -5.0, -5.0),
        ],
        material: AcousticMaterial::concrete(),
        normal: Vec3::new(-1.0, 0.0, 0.0),
    };

    let source = Vec3::new(0.0, 0.0, 0.0);
    let listener = Vec3::new(10.0, 0.0, 0.0);
    assert!(
        is_occluded(source, listener, &[wall]),
        "wall between source and listener should cause occlusion"
    );
}
