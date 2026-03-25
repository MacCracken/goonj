use criterion::{Criterion, black_box, criterion_group, criterion_main};
use hisab::Vec3;

fn bench_speed_of_sound(c: &mut Criterion) {
    c.bench_function("propagation/speed_of_sound", |b| {
        b.iter(|| goonj::propagation::speed_of_sound(black_box(20.0)));
    });
}

fn bench_sabine_rt60(c: &mut Criterion) {
    c.bench_function("impulse/sabine_rt60", |b| {
        b.iter(|| goonj::impulse::sabine_rt60(black_box(240.0), black_box(50.0)));
    });
}

fn bench_room_mode(c: &mut Criterion) {
    c.bench_function("resonance/room_mode", |b| {
        b.iter(|| goonj::resonance::room_mode(black_box(5.0), black_box(1), black_box(343.0)));
    });
}

fn bench_schroeder_frequency(c: &mut Criterion) {
    c.bench_function("resonance/schroeder_frequency", |b| {
        b.iter(|| goonj::resonance::schroeder_frequency(black_box(1.0), black_box(100.0)));
    });
}

fn bench_doppler_shift(c: &mut Criterion) {
    c.bench_function("propagation/doppler_shift", |b| {
        b.iter(|| {
            goonj::propagation::doppler_shift(
                black_box(440.0),
                black_box(-30.0),
                black_box(0.0),
                black_box(343.0),
            )
        });
    });
}

fn bench_inverse_square(c: &mut Criterion) {
    c.bench_function("propagation/inverse_square_law", |b| {
        b.iter(|| goonj::propagation::inverse_square_law(black_box(100.0), black_box(5.0)));
    });
}

fn bench_ray_wall_intersection(c: &mut Criterion) {
    let ray = goonj::ray::AcousticRay::new(Vec3::new(2.5, 1.5, 0.0), Vec3::Z, 1000.0);
    let wall = goonj::room::Wall {
        vertices: vec![
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(5.0, 0.0, 5.0),
            Vec3::new(5.0, 3.0, 5.0),
            Vec3::new(0.0, 3.0, 5.0),
        ],
        material: goonj::material::AcousticMaterial::concrete(),
        normal: Vec3::new(0.0, 0.0, -1.0),
    };
    c.bench_function("ray/wall_intersection", |b| {
        b.iter(|| goonj::ray::ray_wall_intersection(black_box(&ray), black_box(&wall)));
    });
}

fn bench_all_axial_modes(c: &mut Criterion) {
    c.bench_function("resonance/all_axial_modes_200hz", |b| {
        b.iter(|| {
            goonj::resonance::all_axial_modes(
                black_box(10.0),
                black_box(8.0),
                black_box(3.0),
                black_box(200.0),
                black_box(343.0),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_speed_of_sound,
    bench_sabine_rt60,
    bench_room_mode,
    bench_schroeder_frequency,
    bench_doppler_shift,
    bench_inverse_square,
    bench_ray_wall_intersection,
    bench_all_axial_modes,
);
criterion_main!(benches);
