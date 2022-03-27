use crate::{Arm7tdmi, SysBus};

impl<Bus> Arm7tdmi<Bus>
where
    Bus: SysBus,
{
    pub(crate) fn arm_undefined(&mut self, instr: u32) {
        todo!()
    }
}
