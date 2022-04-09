mod registers;

pub use registers::*;

pub struct InterruptController {
    pub enable: InterruptEnable,
    pub master_enable: InterruptMasterEnable,
    pub request: InterruptRequest,
}

impl InterruptController {
    pub fn new() -> InterruptController {
        InterruptController {
            enable: InterruptEnable::new(),
            master_enable: InterruptMasterEnable::new(),
            request: InterruptRequest::new(),
        }
    }
}
