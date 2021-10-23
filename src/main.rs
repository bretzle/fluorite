use crate::cpu::GbaCpu;
use fluorite_arm::cpu::Arm7tdmi;
use fluorite_common::Shared;

mod cpu;

fn main() {
    let arm = Arm7tdmi::new(Shared::default());
    let mut cpu = GbaCpu::new(arm);

    cpu.run();

    println!("Hello, world!");
}
