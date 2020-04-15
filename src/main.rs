use std::env;

mod result;
mod memory;
mod instructions;
mod gui;
mod control_unit;
mod gui;
pub use crate::result::SimResult;
pub use crate::memory::{Memory,InspectableMemory,DRAM,DMCache};
pub use crate::instructions::Instruction;
pub use crate::gui::Display;
pub use crate::control_unit::ControlUnit;
pub use crate::gui::Display;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 2 {
        panic!("Usage: {} [gui]", args[0]);
    }

    // Run GUI
    if args.len() == 2 && args[1] == "gui" {
        Display::start();
    } else {
        // Run text interface
        let mut cu = ControlUnit::new("test-data/example-prog.bin");
        let mut program_running = true;

        while program_running {
            println!("====================");
            match cu.step() {
                Err(e) => panic!("Failed to run processor cycle: {}", e),
                Ok(keep_running) => program_running = keep_running,
            };

            println!("{}", cu);
            if !program_running {
                println!("Program ended");
            }
        }
    }
}
