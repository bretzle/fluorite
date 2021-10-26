use fluorite_arm::{arm::ArmInstruction, disassembler::Disassembler};

// mod cpu;

const DATA: &[u8] = include_bytes!("../roms/beeg.bin");

fn main() {
    // let arm = Arm7tdmi::new(Shared::default());
    // let mut cpu = GbaCpu::new(arm);

    // cpu.run();

    // println!("Hello, world!");

    let disassem: Disassembler<ArmInstruction> = Disassembler::new(0, DATA);

    for x in disassem {
        println!("{}", x.1);
    }
}
