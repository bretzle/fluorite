use cpu::GbaCpu;

mod bios;
mod cartridge;
mod cpu;
mod sysbus;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut cpu = GbaCpu::new();
	cpu.skip_bios();

	cpu.run();

    Ok(())
}
