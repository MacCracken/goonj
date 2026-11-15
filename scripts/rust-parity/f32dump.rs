use goonj::diffusion::*;
use goonj::dwm::*;
use goonj::fdtd::*;
use goonj::material::AcousticMaterial;
use std::io::Write;

include!("cases_gen.rs");

thread_local! { static OUT: std::cell::RefCell<std::io::BufWriter<std::io::Stdout>> = std::cell::RefCell::new(std::io::BufWriter::new(std::io::stdout())); }

fn w(s: String) { OUT.with(|o| { writeln!(o.borrow_mut(), "{}", s).unwrap(); }); }

fn meta(case: usize, steps: u64, b: u64) {
    w(format!("#\n{}\nmeta\n0\n2\n{}\n{}", case, steps, b));
}

fn dump(case: usize, label: &str, idx: usize, v: &[f32]) {
    w(format!("#\n{}\n{}\n{}\n{}", case, label, idx, v.len()));
    for x in v { w(format!("{}", x.to_bits())); }
}

fn main() {
    run_cases();
    OUT.with(|o| o.borrow_mut().flush().unwrap());
}
