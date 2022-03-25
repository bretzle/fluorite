pub struct EmulatorState {
    pub show_registers: bool,
    pub show_about: bool,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self {
            show_registers: true,
            show_about: false,
        }
    }
}
