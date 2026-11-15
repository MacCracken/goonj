#!/usr/bin/env python3
"""Generate the same grid-solver cases as Rust (cases_gen.rs) and Cyrius (cases_gen.cyr).

Usage: gen.py out.rs out.cyr [seed]

Every float input is an f32 value, written as its bit pattern, so both sides see the same
numbers: Rust as f32::from_bits, Cyrius as f32_to(bits), the exact f64 of that f32.
Output protocol, one token per line: '#', case id, label, count, then count u32 values.
"""
import random
import struct
import sys

R = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 20261115)
NUM_BANDS = 8


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def bits(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def rnd_signal(n, scale=1.0, special=False):
    out = [f32(R.uniform(-scale, scale)) for _ in range(n)]
    if special and n > 6:
        out[1] = 0.0
        out[2] = -0.0
        out[3] = f32(1e-40)       # subnormal
        out[4] = f32(-3e-39)
        out[5] = f32(1e30)
    return out


rs = []   # Rust statements
cy = []   # Cyrius statements (inside per-case functions)
cy_calls = []
case_id = 0


def rs_vec(vals):
    return 'vec![' + ', '.join('f32::from_bits(0x%08X)' % bits(v) for v in vals) + ']'


def cy_vec(name, vals):
    lines = ['    var %s = vec_new();' % name]
    for v in vals:
        lines.append('    vec_push(%s, f32_to(0x%08X));' % (name, bits(v)))
    return lines


def cy_buf(name, vals):
    lines = ['    var %s = alloc(%d);' % (name, max(len(vals), 1) * 8)]
    for i, v in enumerate(vals):
        lines.append('    store64(%s + %d, f32_to(0x%08X));' % (name, i * 8, bits(v)))
    return lines


def new_case(kind):
    global case_id
    case_id += 1
    cy.append('fn case_%d() {' % case_id)
    cy.append('    # %s' % kind)
    cy_calls.append('case_%d();' % case_id)
    rs.append('    {')
    rs.append('        // case %d: %s' % (case_id, kind))
    return case_id


def end_case():
    cy.append('    return 0;')
    cy.append('}')
    cy.append('')
    rs.append('    }')


# ---------------------------------------------------------------- FDTD
def fdtd_case(sr, dx, c, nx, ny, dur, src, receivers, kind='fdtd'):
    cid = new_case(kind)
    rs.append('        let cfg = FdtdConfig { sample_rate: %d, dx: f32::from_bits(0x%08X), speed_of_sound: f32::from_bits(0x%08X), nx: %d, ny: %d, duration_seconds: f32::from_bits(0x%08X) };'
              % (sr, bits(dx), bits(c), nx, ny, bits(dur)))
    cy.append('    var cfg = fdtd_config_new(%d, f32_to(0x%08X), f32_to(0x%08X), %d, %d, f32_to(0x%08X));'
              % (sr, bits(dx), bits(c), nx, ny, bits(dur)))
    if src[0] == 'sig':
        _, ix, iy, sig = src
        rs.append('        let src = FdtdSource { ix: %d, iy: %d, signal: %s };' % (ix, iy, rs_vec(sig)))
        cy.extend(cy_vec('sig', sig))
        cy.append('    var src = fdtd_source_new(%d, %d, sig);' % (ix, iy))
    else:
        _, ix, iy, peak, sigma, amp = src
        rs.append('        let src = FdtdSource::gaussian_pulse(%d, %d, %d, f32::from_bits(0x%08X), f32::from_bits(0x%08X));'
                  % (ix, iy, peak, bits(sigma), bits(amp)))
        cy.append('    var src = fdtd_source_gaussian_pulse(%d, %d, %d, f32_to(0x%08X), f32_to(0x%08X));'
                  % (ix, iy, peak, bits(sigma), bits(amp)))
        rs.append('        dump(%d, "signal", 0, &src.signal);' % cid)
        cy.append('    dump64(%d, "signal", FdtdSource_signal(src));' % cid)
    rs.append('        let rx = vec![' + ', '.join('FdtdReceiver { ix: %d, iy: %d }' % r for r in receivers) + '];')
    cy.append('    var rx = vec_new();')
    for r in receivers:
        cy.append('    vec_push(rx, fdtd_receiver_new(%d, %d));' % r)
    rs.append('        let r = solve_fdtd_2d(&cfg, &src, &rx);')
    rs.append('        meta(%d, r.time_steps as u64, r.dt.to_bits() as u64);' % cid)
    rs.append('        for (i, s) in r.receiver_signals.iter().enumerate() { dump(%d, "r", i, s); }' % cid)
    rs.append('        dump(%d, "final", 0, &r.final_pressure);' % cid)
    cy.append('    var r = solve_fdtd_2d(cfg, src, rx);')
    cy.append('    meta(%d, FdtdResult_time_steps(r), f32_from(FdtdResult_dt(r)) & 0xFFFFFFFF);' % cid)
    cy.append('    dump_traces(%d, FdtdResult_receiver_signals(r));' % cid)
    cy.append('    dump64(%d, "final", FdtdResult_final_pressure(r));' % cid)
    end_case()


def fdtd_cases():
    sizes = [(3, 3), (4, 3), (3, 5), (5, 7), (8, 8), (9, 6), (16, 11), (31, 33), (40, 40),
             (63, 9), (64, 10), (65, 7), (66, 5), (70, 12), (97, 6), (130, 5), (33, 64)]
    for (nx, ny) in sizes:
        sr = R.choice([8000, 22050, 44100, 48000, 11025])
        c = f32(R.choice([343.0, 340.0, 1480.0, 331.3]))
        cfl = R.uniform(0.25, 0.70)
        dt = 1.0 / sr
        dx = f32(c * dt / cfl)
        steps = R.randint(20, 220)
        dur = f32(steps * dt + R.uniform(0, 0.9) * dt)
        ix, iy = R.randrange(nx), R.randrange(ny)
        if R.random() < 0.6:
            src = ('sig', ix, iy, rnd_signal(R.randint(1, steps), special=(nx == 16)))
        else:
            src = ('gauss', ix, iy, R.randint(0, 30), f32(R.uniform(0.5, 6.0)), f32(R.uniform(0.1, 2.0)))
        rx = [(R.randrange(nx), R.randrange(ny)) for _ in range(R.randint(0, 3))]
        rx += [(0, 0), (nx - 1, ny - 1), (nx, 0)]
        fdtd_case(sr, dx, c, nx, ny, dur, src, rx)
    # CFL right at the limit: f32 and f64 decide this differently.
    for k in range(4):
        sr = 22050
        c = f32(343.0)
        dt32 = f32(1.0 / sr)
        dx = f32(c * dt32 / f32(0.70710677) * (1 + (k - 1.5) * 1e-7))
        fdtd_case(sr, dx, c, 12, 10, f32(60 * dt32), ('sig', 5, 4, rnd_signal(30)), [(6, 6)], 'fdtd-cfl-edge')
    # Durations a hair either side of a whole number of steps.
    for k in range(4):
        sr = 44100
        dt32 = f32(1.0 / sr)
        dur = f32(37 * dt32 * (1 + (k - 1.5) * 2e-7))
        fdtd_case(sr, f32(0.05), f32(343.0), 10, 10, dur, ('sig', 4, 4, rnd_signal(12)), [(3, 3)], 'fdtd-steps-edge')


# ---------------------------------------------------------------- DWM
def material(name):
    return [f32(R.uniform(0, 1)) for _ in range(NUM_BANDS)], f32(R.uniform(0, 1))


def dwm_case(sr, dx, c, nx, ny, nz, dur, mats, src, receivers, kind='dwm'):
    cid = new_case(kind)
    rs.append('        let m = |a: [u32; 8], s: u32| AcousticMaterial { name: "m".into(), absorption: a.map(f32::from_bits), scattering: f32::from_bits(s) };')
    names = ['x_neg', 'x_pos', 'y_neg', 'y_pos', 'z_neg', 'z_pos']
    parts = []
    for k, (ab, sc) in enumerate(mats):
        parts.append('%s: m([%s], 0x%08X)' % (names[k], ', '.join('0x%08X' % bits(a) for a in ab), bits(sc)))
        cy.extend(cy_buf('ab%d' % k, ab))
        cy.append('    var m%d = acoustic_material_new("m", ab%d, f32_to(0x%08X));' % (k, k, bits(sc)))
    rs.append('        let walls = WallMaterials { %s };' % ', '.join(parts))
    rs.append('        let cfg = DwmConfig { sample_rate: %d, dx: f32::from_bits(0x%08X), speed_of_sound: f32::from_bits(0x%08X), nx: %d, ny: %d, nz: %d, duration_seconds: f32::from_bits(0x%08X), wall_materials: walls };'
              % (sr, bits(dx), bits(c), nx, ny, nz, bits(dur)))
    cy.append('    var walls = wall_materials_new(m0, m1, m2, m3, m4, m5);')
    cy.append('    var cfg = dwm_config_new(%d, f32_to(0x%08X), f32_to(0x%08X), %d, %d, %d, f32_to(0x%08X), walls);'
              % (sr, bits(dx), bits(c), nx, ny, nz, bits(dur)))
    if src[0] == 'sig':
        _, ix, iy, iz, sig = src
        rs.append('        let src = DwmSource { ix: %d, iy: %d, iz: %d, signal: %s };' % (ix, iy, iz, rs_vec(sig)))
        cy.extend(cy_vec('sig', sig))
        cy.append('    var src = alloc(sizeof(DwmSource));')
        cy.append('    DwmSource_set_ix(src, %d); DwmSource_set_iy(src, %d); DwmSource_set_iz(src, %d);' % (ix, iy, iz))
        cy.append('    DwmSource_set_signal(src, sig);')
    else:
        _, ix, iy, iz, peak, sigma, amp = src
        rs.append('        let src = DwmSource::gaussian_pulse(%d, %d, %d, %d, f32::from_bits(0x%08X), f32::from_bits(0x%08X));'
                  % (ix, iy, iz, peak, bits(sigma), bits(amp)))
        cy.append('    var src = dwm_source_gaussian_pulse(%d, %d, %d, %d, f32_to(0x%08X), f32_to(0x%08X));'
                  % (ix, iy, iz, peak, bits(sigma), bits(amp)))
        rs.append('        dump(%d, "signal", 0, &src.signal);' % cid)
        cy.append('    dump64(%d, "signal", DwmSource_signal(src));' % cid)
    rs.append('        let rx = vec![' + ', '.join('DwmReceiver { ix: %d, iy: %d, iz: %d }' % r for r in receivers) + '];')
    cy.append('    var rx = vec_new();')
    for r in receivers:
        cy.append('    vec_push(rx, dwm_receiver_new(%d, %d, %d));' % r)
    rs.append('        let r = solve_dwm_3d(&cfg, &src, &rx);')
    rs.append('        meta(%d, r.time_steps as u64, r.dt.to_bits() as u64);' % cid)
    rs.append('        for (i, s) in r.receiver_signals.iter().enumerate() { dump(%d, "r", i, s); }' % cid)
    rs.append('        dump(%d, "final", 0, &r.final_pressure);' % cid)
    cy.append('    var r = solve_dwm_3d(cfg, src, rx);')
    cy.append('    meta(%d, DwmResult_time_steps(r), f32_from(DwmResult_dt(r)) & 0xFFFFFFFF);' % cid)
    cy.append('    dump_traces(%d, DwmResult_receiver_signals(r));' % cid)
    cy.append('    dump64(%d, "final", DwmResult_final_pressure(r));' % cid)
    end_case()


def dwm_cases():
    sizes = [(3, 3, 3), (4, 3, 5), (5, 6, 4), (7, 7, 7), (9, 8, 6), (12, 10, 9), (17, 13, 11),
             (20, 18, 16), (66, 3, 3), (70, 4, 3), (130, 3, 3)]
    for (nx, ny, nz) in sizes:
        sr = R.choice([8000, 16000, 22050, 44100])
        c = f32(R.choice([343.0, 340.0, 1480.0]))
        dx = f32(c / sr * 1.7320508 * (1 + R.uniform(-0.05, 0.05)))
        steps = R.randint(15, 120)
        dur = f32(steps / sr + R.uniform(0, 0.9) / sr)
        if R.random() < 0.3:
            mats = [([0.0] * 8, 0.0)] * 6
        elif R.random() < 0.5:
            mats = [material('u')] * 6
        else:
            mats = [material('w%d' % k) for k in range(6)]
        ix, iy, iz = R.randrange(nx), R.randrange(ny), R.randrange(nz)
        if R.random() < 0.6:
            src = ('sig', ix, iy, iz, rnd_signal(R.randint(1, steps), special=(nx == 7)))
        else:
            src = ('gauss', ix, iy, iz, R.randint(0, 20), f32(R.uniform(0.5, 5.0)), f32(R.uniform(0.1, 2.0)))
        rx = [(R.randrange(nx), R.randrange(ny), R.randrange(nz)) for _ in range(R.randint(0, 3))]
        rx += [(0, 0, 0), (nx - 1, ny - 1, nz - 1), (0, ny, 0)]
        dwm_case(sr, dx, c, nx, ny, nz, dur, mats, src, rx)
    # Absorption at the edges of the filter fit: 0, 1, equal ends, and one end at 1.
    edge = [([0.0] * 8, 0.0), ([1.0] * 8, 0.5), ([0.3] * 8, 0.1),
            ([1.0] + [0.2] * 7, 0.0), ([0.0] * 7 + [1.0], 0.0), ([0.999] + [0.0] * 7, 0.0)]
    edge = [([f32(a) for a in ab], f32(s)) for ab, s in edge]
    dwm_case(22050, f32(343.0 / 22050 * 1.7320508), f32(343.0), 8, 7, 6, f32(60 / 22050),
             edge, ('sig', 3, 3, 3, rnd_signal(25)), [(1, 1, 1), (7, 6, 5)], 'dwm-filter-edge')
    # dx deviation either side of the 1% and 10% tolerances.
    for dev in (0.0099, 0.0101, 0.0999, 0.1001):
        sr = 22050
        c = f32(343.0)
        dx = f32(c * f32(1.0 / sr) * f32(1.7320508) * (1 + dev))
        dwm_case(sr, dx, c, 6, 5, 4, f32(30 / sr), [([0.1] * 8, 0.0)] * 6,
                 ('sig', 2, 2, 2, rnd_signal(10)), [(3, 2, 1)], 'dwm-dx-edge')


# ---------------------------------------------------------------- diffusion
def diff_case(dx, dt, nx, ny, c, mfp, tmax, absorb, source, kind='diffusion'):
    cid = new_case(kind)
    rs.append('        let cfg = DiffusionConfig { dx: f32::from_bits(0x%08X), dt: f32::from_bits(0x%08X), nx: %d, ny: %d, speed_of_sound: f32::from_bits(0x%08X), mean_free_path: f32::from_bits(0x%08X), max_time: f32::from_bits(0x%08X) };'
              % (bits(dx), bits(dt), nx, ny, bits(c), bits(mfp), bits(tmax)))
    rs.append('        let ab: Vec<f32> = %s;' % rs_vec(absorb))
    rs.append('        let so: Vec<f32> = %s;' % rs_vec(source))
    rs.append('        let r = solve_diffusion_2d(&cfg, &ab, &so);')
    rs.append('        meta(%d, r.time_steps as u64, r.dx.to_bits() as u64);' % cid)
    rs.append('        dump(%d, "energy", 0, &r.energy_density);' % cid)
    cy.append('    var cfg = diffusion_config_new(f32_to(0x%08X), f32_to(0x%08X), %d, %d, f32_to(0x%08X), f32_to(0x%08X), f32_to(0x%08X));'
              % (bits(dx), bits(dt), nx, ny, bits(c), bits(mfp), bits(tmax)))
    cy.extend(cy_buf('ab', absorb))
    cy.extend(cy_buf('so', source))
    cy.append('    var r = solve_diffusion_2d(cfg, ab, %d, so, %d);' % (len(absorb), len(source)))
    cy.append('    meta(%d, DiffusionResult_time_steps(r), f32_from(DiffusionResult_dx(r)) & 0xFFFFFFFF);' % cid)
    cy.append('    dump64(%d, "energy", DiffusionResult_energy_density(r));' % cid)
    end_case()


def diff_cases():
    sizes = [(3, 3), (4, 5), (7, 6), (12, 9), (25, 20), (33, 31), (60, 40), (65, 5), (66, 6), (100, 4)]
    for (nx, ny) in sizes:
        n = nx * ny
        c = f32(343.0)
        mfp = f32(R.uniform(0.3, 3.0))
        dx = f32(R.uniform(0.2, 1.0))
        d = c * mfp / 3
        dtmax = dx * dx / (4 * d)
        dt = f32(dtmax * R.choice([0.5, 0.85, 1.2, 3.0]))
        steps = R.randint(30, 300)
        tmax = f32(steps * min(dt, dtmax * 0.9) * R.uniform(1.0, 1.05))
        absorb = [f32(R.uniform(0, 0.5)) for _ in range(R.choice([n, n // 2, 0]))]
        source = [0.0] * n
        for _ in range(R.randint(1, 5)):
            source[R.randrange(n)] = f32(R.uniform(0.1, 10.0))
        if nx == 12:
            source[1] = -0.0
            source[nx + 1] = f32(-0.5)
        diff_case(dx, dt, nx, ny, c, mfp, tmax, absorb, source)
    # Grid clamps and the step cap's neighbourhood.
    diff_case(f32(0.5), f32(1e-4), 2, 1, f32(343.0), f32(1.0), f32(0.01), [], [f32(1.0)] * 3, 'diffusion-clamp')


fdtd_cases()
dwm_cases()
diff_cases()

with open(sys.argv[1], 'w') as f:
    f.write('// generated by gen.py\n#[allow(unused_variables, clippy::all)]\npub fn run_cases() {\n')
    f.write('\n'.join(rs))
    f.write('\n}\n')
with open(sys.argv[2], 'w') as f:
    f.write('# generated by gen.py\n')
    f.write('\n'.join(cy))
    f.write('\nfn run_cases() {\n    ' + '\n    '.join(cy_calls) + '\n    return 0;\n}\n')
print('%d cases' % case_id)
