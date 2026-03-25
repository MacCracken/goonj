use goonj::*;
use goonj::material::AcousticMaterial;
use goonj::room::AcousticRoom;
use goonj::resonance;
use goonj::ray::AcousticRay;

#[test]
fn shoebox_room_sabine_rt60() {
    let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());
    let vol = room.geometry.volume_shoebox();
    let abs = room.geometry.total_absorption();
    let rt60 = impulse::sabine_rt60(vol, abs);
    // Carpet is absorptive, RT60 should be short (~0.3-0.5s)
    assert!(rt60 > 0.1 && rt60 < 2.0, "carpet room RT60 should be moderate, got {rt60}");
}

#[test]
fn concrete_longer_than_carpet() {
    let concrete = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
    let carpet = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::carpet());

    let rt60_concrete = impulse::sabine_rt60(concrete.geometry.volume_shoebox(), concrete.geometry.total_absorption());
    let rt60_carpet = impulse::sabine_rt60(carpet.geometry.volume_shoebox(), carpet.geometry.total_absorption());

    assert!(rt60_concrete > rt60_carpet, "concrete should be more reverberant than carpet");
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
    assert!(fs > 50.0 && fs < 1000.0, "Schroeder frequency should be reasonable, got {fs}");
}
