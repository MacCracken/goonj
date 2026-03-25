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

fn bench_trace_single_shoebox(c: &mut Criterion) {
    let room = goonj::room::RoomGeometry::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let ray = goonj::ray::MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), Vec3::Z);
    c.bench_function("ray/trace_single_shoebox", |b| {
        b.iter(|| goonj::ray::trace_ray(black_box(&ray), black_box(&room.walls), black_box(50)));
    });
}

fn bench_trace_100_rays_shoebox(c: &mut Criterion) {
    let room = goonj::room::RoomGeometry::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let rays: Vec<goonj::ray::MultibandRay> = (0..100)
        .map(|i| {
            let angle = i as f32 * 0.0628; // ~100 directions spread over ~2π
            let dir = Vec3::new(angle.cos(), 0.1, angle.sin());
            goonj::ray::MultibandRay::new(Vec3::new(5.0, 1.5, 4.0), dir)
        })
        .collect();
    c.bench_function("ray/trace_100_rays_shoebox", |b| {
        b.iter(|| {
            for ray in &rays {
                let _ =
                    goonj::ray::trace_ray(black_box(ray), black_box(&room.walls), black_box(50));
            }
        });
    });
}

fn bench_image_source_order_3(c: &mut Criterion) {
    let room = goonj::room::AcousticRoom::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let source = Vec3::new(3.0, 1.5, 4.0);
    let listener = Vec3::new(7.0, 1.5, 4.0);
    let spd = goonj::propagation::speed_of_sound(20.0);
    c.bench_function("image_source/shoebox_order_3", |b| {
        b.iter(|| {
            goonj::image_source::compute_early_reflections(
                black_box(source),
                black_box(listener),
                black_box(&room),
                black_box(3),
                black_box(spd),
            )
        });
    });
}

fn bench_image_source_order_5(c: &mut Criterion) {
    let room = goonj::room::AcousticRoom::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let source = Vec3::new(3.0, 1.5, 4.0);
    let listener = Vec3::new(7.0, 1.5, 4.0);
    let spd = goonj::propagation::speed_of_sound(20.0);
    c.bench_function("image_source/shoebox_order_5", |b| {
        b.iter(|| {
            goonj::image_source::compute_early_reflections(
                black_box(source),
                black_box(listener),
                black_box(&room),
                black_box(5),
                black_box(spd),
            )
        });
    });
}

fn bench_diffuse_1000_rays(c: &mut Criterion) {
    let room = goonj::room::AcousticRoom::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let source = Vec3::new(3.0, 1.5, 4.0);
    let listener = Vec3::new(7.0, 1.5, 4.0);
    let config = goonj::diffuse::DiffuseRainConfig {
        num_rays: 1000,
        max_bounces: 50,
        max_time_seconds: 2.0,
        collection_radius: 1.5,
        speed_of_sound: goonj::propagation::speed_of_sound(20.0),
        seed: 42,
    };
    c.bench_function("diffuse/1000_rays_shoebox", |b| {
        b.iter(|| {
            goonj::diffuse::generate_diffuse_rain(
                black_box(source),
                black_box(listener),
                black_box(&room),
                black_box(&config),
            )
        });
    });
}

fn bench_generate_ir_shoebox(c: &mut Criterion) {
    let room = goonj::room::AcousticRoom::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let source = Vec3::new(3.0, 1.5, 4.0);
    let listener = Vec3::new(7.0, 1.5, 4.0);
    let config = goonj::impulse::IrConfig {
        num_diffuse_rays: 1000,
        max_time_seconds: 0.5,
        ..goonj::impulse::IrConfig::default()
    };
    c.bench_function("impulse/generate_ir_shoebox", |b| {
        b.iter(|| {
            goonj::impulse::generate_ir(
                black_box(source),
                black_box(listener),
                black_box(&room),
                black_box(&config),
            )
        });
    });
}

fn bench_analysis_c80(c: &mut Criterion) {
    let ir = goonj::impulse::ImpulseResponse {
        samples: (0..48000)
            .map(|i| (-0.005 * i as f32 / 48.0).exp() * 0.5)
            .collect(),
        sample_rate: 48000,
        rt60: 1.0,
    };
    c.bench_function("analysis/c80", |b| {
        b.iter(|| goonj::analysis::clarity_c80(black_box(&ir)));
    });
}

fn bench_analysis_sti(c: &mut Criterion) {
    let ir = goonj::impulse::ImpulseResponse {
        samples: (0..48000)
            .map(|i| (-0.005 * i as f32 / 48.0).exp() * 0.5)
            .collect(),
        sample_rate: 48000,
        rt60: 1.0,
    };
    c.bench_function("analysis/sti", |b| {
        b.iter(|| goonj::analysis::sti_estimate(black_box(&ir)));
    });
}

fn bench_atmospheric_ray_1000_steps(c: &mut Criterion) {
    let wind = goonj::propagation::WindProfile {
        direction: Vec3::X,
        speed_ground: 5.0,
        gradient: 0.1,
    };
    let temp = goonj::propagation::TemperatureProfile {
        ground_temp_celsius: 20.0,
        lapse_rate: -0.0065,
    };
    c.bench_function("propagation/atmospheric_ray_1000_steps", |b| {
        b.iter(|| {
            goonj::propagation::trace_ray_atmospheric(
                black_box(Vec3::new(0.0, 100.0, 0.0)),
                black_box(Vec3::new(1.0, -0.1, 0.0)),
                black_box(&wind),
                black_box(&temp),
                black_box(1000.0),
                black_box(1.0),
            )
        });
    });
}

fn bench_wav_export(c: &mut Criterion) {
    let samples: Vec<f32> = (0..96000)
        .map(|i| (-0.005 * i as f32 / 48.0).exp() * 0.5)
        .collect();
    c.bench_function("wav/export_48k_2s", |b| {
        b.iter(|| {
            let mut buf = Vec::with_capacity(96000 * 2 + 44);
            goonj::wav::write_wav_mono(black_box(&samples), black_box(48000), &mut buf).unwrap();
            buf
        });
    });
}

fn bench_binaural_ir(c: &mut Criterion) {
    let room = goonj::room::AcousticRoom::shoebox(
        10.0,
        8.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let hrtf = goonj::binaural::HrtfDataset::from_pairs(
        vec![
            goonj::binaural::HrtfPair {
                azimuth: 0.0,
                elevation: 0.0,
                left: vec![1.0, 0.5, 0.2],
                right: vec![1.0, 0.5, 0.2],
            },
            goonj::binaural::HrtfPair {
                azimuth: std::f32::consts::FRAC_PI_2,
                elevation: 0.0,
                left: vec![0.3, 0.1, 0.05],
                right: vec![1.0, 0.8, 0.4],
            },
        ],
        48000,
    );
    let config = goonj::impulse::IrConfig {
        max_time_seconds: 0.2,
        num_diffuse_rays: 0,
        ..goonj::impulse::IrConfig::default()
    };
    let source = Vec3::new(3.0, 1.5, 4.0);
    let listener = Vec3::new(7.0, 1.5, 4.0);
    c.bench_function("binaural/generate_ir_shoebox", |b| {
        b.iter(|| {
            goonj::binaural::generate_binaural_ir(
                black_box(source),
                black_box(listener),
                black_box(&room),
                black_box(&hrtf),
                black_box(&config),
            )
        });
    });
}

fn make_many_wall_room(num_extra_walls: usize) -> goonj::room::AcousticRoom {
    // Start with a shoebox and add internal partitions
    let mut room = goonj::room::AcousticRoom::shoebox(
        20.0,
        20.0,
        3.0,
        goonj::material::AcousticMaterial::concrete(),
    );
    let mat = goonj::material::AcousticMaterial::drywall();
    for i in 0..num_extra_walls {
        let x = 1.0 + (i as f32 * 18.0 / num_extra_walls as f32);
        room.geometry.walls.push(goonj::room::Wall {
            vertices: vec![
                Vec3::new(x, 0.0, 9.0),
                Vec3::new(x, 3.0, 9.0),
                Vec3::new(x, 3.0, 11.0),
                Vec3::new(x, 0.0, 11.0),
            ],
            material: mat.clone(),
            normal: Vec3::X,
        });
    }
    room
}

fn bench_trace_bvh_100_walls(c: &mut Criterion) {
    let room = make_many_wall_room(94); // 6 shoebox + 94 = 100 walls
    let accel = goonj::room::AcceleratedRoom::new(room);
    let ray = goonj::ray::MultibandRay::new(Vec3::new(10.0, 1.5, 10.0), Vec3::X);
    c.bench_function("ray/trace_bvh_100_walls", |b| {
        b.iter(|| goonj::ray::trace_ray_bvh(black_box(&ray), black_box(&accel), black_box(50)));
    });
}

fn bench_trace_linear_100_walls(c: &mut Criterion) {
    let room = make_many_wall_room(94);
    let ray = goonj::ray::MultibandRay::new(Vec3::new(10.0, 1.5, 10.0), Vec3::X);
    c.bench_function("ray/trace_linear_100_walls", |b| {
        b.iter(|| {
            goonj::ray::trace_ray(
                black_box(&ray),
                black_box(&room.geometry.walls),
                black_box(50),
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
    bench_trace_single_shoebox,
    bench_trace_100_rays_shoebox,
    bench_image_source_order_3,
    bench_image_source_order_5,
    bench_diffuse_1000_rays,
    bench_generate_ir_shoebox,
    bench_analysis_c80,
    bench_analysis_sti,
    bench_atmospheric_ray_1000_steps,
    bench_wav_export,
    bench_binaural_ir,
    bench_trace_bvh_100_walls,
    bench_trace_linear_100_walls,
);
criterion_main!(benches);
