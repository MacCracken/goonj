use goonj::{impulse, material::AcousticMaterial, propagation, resonance, room::AcousticRoom};

fn main() {
    // Speed of sound at room temperature
    let speed = propagation::speed_of_sound(20.0);
    println!("Speed of sound at 20°C: {speed:.1} m/s");

    // Create a shoebox concert hall
    let hall = AcousticRoom::shoebox(30.0, 20.0, 12.0, AcousticMaterial::wood());
    let volume = hall.geometry.volume_shoebox();
    let absorption = hall.geometry.total_absorption();

    println!("Hall volume: {volume:.0} m³");
    println!("Total absorption: {absorption:.1} Sabins");

    // Reverberation time
    let rt60 = impulse::sabine_rt60(volume, absorption);
    println!("Sabine RT60: {rt60:.2}s");

    // Schroeder frequency
    let fs = resonance::schroeder_frequency(rt60, volume);
    println!("Schroeder frequency: {fs:.0} Hz");

    // First axial mode
    let first_mode = resonance::room_mode(30.0, 1, speed);
    println!("First axial mode (length): {first_mode:.1} Hz");

    // Doppler shift (ambulance approaching at 30 m/s)
    let shifted = propagation::doppler_shift(440.0, -30.0, 0.0, speed);
    println!("Doppler shift (approaching at 30 m/s): {shifted:.1} Hz");
}
