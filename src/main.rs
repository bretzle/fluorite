use crate::armcore::Arm7tdmi;
use crate::util::Shared;

mod util;
mod armcore;

fn main() {
    let arm = Arm7tdmi::new(Shared::default());

    println!("Hello, world!");
}
