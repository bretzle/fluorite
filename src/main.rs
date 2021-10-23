use crate::{armcore::Arm7tdmi, cpu::GbaCpu};
use fluorite_common::Shared;

mod armcore;
mod cpu;

fn main() {
    let arm = Arm7tdmi::new(Shared::default());
    let mut cpu = GbaCpu::new(arm);

    cpu.run();

    println!("Hello, world!");
}
