use cpu::Gba;

mod bios;
mod cartridge;
mod cpu;
mod sysbus;
mod iodev;
mod gpu;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut cpu = Gba::new();
	cpu.skip_bios();

	cpu.run();

    Ok(())
}
