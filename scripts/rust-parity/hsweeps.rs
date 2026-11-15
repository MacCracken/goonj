// tests/hardening.tcyr's golden sweeps (_h_g_dwm_sweep, _h_g_fdtd_sweep, _h_g_diffusion_sweep),
// run through Rust 2.1.3. Each value is hashed as its f64 bits, as the Cyrius test does.
use goonj::diffusion::*;
use goonj::dwm::*;
use goonj::fdtd::*;
use goonj::material::AcousticMaterial;

struct H(u64);
impl H {
    fn new() -> Self { H(0xCBF29CE484222325) }
    fn mix(&mut self, v: u64) { for i in 0..8 { self.0 ^= (v >> (i * 8)) & 255; self.0 = self.0.wrapping_mul(0x100000001B3); } }
    fn vec(&mut self, v: &[f32]) { self.mix(v.len() as u64); for x in v { self.mix((*x as f64).to_bits()); } }
}

fn signal() -> Vec<f32> { vec![1.0, -0.5, (1.0f64 / 3.0) as f32] }

fn walls(k: usize) -> WallMaterials {
    match k {
        0 => WallMaterials::rigid(),
        1 => WallMaterials::uniform(AcousticMaterial::carpet()),
        _ => WallMaterials { x_neg: AcousticMaterial::concrete(), x_pos: AcousticMaterial::carpet(), y_neg: AcousticMaterial::wood(),
                             y_pos: AcousticMaterial::glass(), z_neg: AcousticMaterial::tile(), z_pos: AcousticMaterial::drywall() },
    }
}

fn main() {
    let sqrt3 = f64::from_bits(0x3FFBB67AE6502B91);
    let mut h = H::new();
    let dx = ((343.0f64 / 22050.0) * sqrt3) as f32;
    let t = (1.0f64 / 2000.0) as f32;
    for nx in 3..=8usize { for ny in 3..=5usize { for nz in 3..=5usize { for w in 0..3usize { for sx in 0..nx {
        let src = DwmSource { ix: sx, iy: (nx + ny + nz + sx) % ny, iz: (sx * 3 + w) % nz, signal: signal() };
        let cfg = DwmConfig { sample_rate: 22050, dx, speed_of_sound: 343.0, nx, ny, nz, duration_seconds: t, wall_materials: walls(w) };
        let rv = [DwmReceiver { ix: nx - 1, iy: ny - 1, iz: nz - 1 }, DwmReceiver { ix: nx / 2, iy: ny / 2, iz: nz / 2 }];
        let r = solve_dwm_3d(&cfg, &src, &rv);
        h.vec(&r.final_pressure); h.vec(&r.receiver_signals[0]); h.vec(&r.receiver_signals[1]);
    }}}}}
    println!("dwm {}", h.0 as i64);

    let mut h = H::new();
    let dx = ((343.0f64 / 22050.0) * (3.0 / 2.0)) as f32;
    let t = (1.0f64 / 500.0) as f32;
    for nx in 3..=14usize { for ny in 3..=9usize { for sx in 0..nx {
        let cfg = FdtdConfig { sample_rate: 22050, dx, speed_of_sound: 343.0, nx, ny, duration_seconds: t };
        let rv = [FdtdReceiver { ix: nx - 1, iy: ny - 1 }, FdtdReceiver { ix: nx / 2, iy: ny / 2 }];
        let src = FdtdSource { ix: sx, iy: (sx * 5 + ny) % ny, signal: signal() };
        let r = solve_fdtd_2d(&cfg, &src, &rv);
        h.vec(&r.final_pressure); h.vec(&r.receiver_signals[0]); h.vec(&r.receiver_signals[1]);
    }}}
    println!("fdtd {}", h.0 as i64);

    let mut h = H::new();
    for nx in 3..=14usize { for ny in 3..=9usize {
        let cells = nx * ny;
        let a: Vec<f32> = (0..cells).map(|i| ((i % 7) as f64 / 20.0) as f32).collect();
        let mut s = vec![0.0f32; cells];
        s[cells / 2] = 1.0;
        let cfg = DiffusionConfig { dx: 0.5, dt: (1.0f64 / 10000.0) as f32, nx, ny, speed_of_sound: 343.0, mean_free_path: 2.0, max_time: (1.0f64 / 500.0) as f32 };
        let r = solve_diffusion_2d(&cfg, &a[..cells - nx], &s);
        h.vec(&r.energy_density);
    }}
    println!("diffusion {}", h.0 as i64);
}
