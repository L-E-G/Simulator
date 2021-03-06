use bit_field::BitField;

use web_sys::console;
use wasm_bindgen::JsValue;

use std::boxed::Box;
use std::fmt;
use std::cell::RefCell;
use std::rc::Rc;

use crate::result::SimResult;
use crate::memory::{Memory,DRAM,DMCache,Registers,PC};
use crate::instructions::{Instruction,InstructionT,
    MemoryOp,AddrMode,Load,Store,Push,Pop,
    ArithMode,ALUOp,Move,ArithSign,ArithUnsign,
    Comp,AS,LS,LogicType,ThreeOpLogic,Not,
    ControlOp,Jump,SIH,INT,RFI,Halt,Noop
};

/// Responsible for running instructions.
pub struct ControlUnit {
    /// Indicates if a pipeline should be used.
    pub pipeline_enabled: bool,

    /// Indicates if the cache should be used.
    pub cache_enabled: bool,
    
    /// Processor cycle counter.
    pub cycle_count: u32,
    
    /// Holds computation registers.
    pub registers: Registers,

    /// Memory system.
    pub dram: Rc<RefCell<dyn Memory<u32, u32>>>,
    pub cache: Rc<RefCell<dyn Memory<u32, u32>>>,

    /// Indicates that the processor has loaded the first instruction yet.
    pub first_instruction_loaded: bool,

    /// Indicates that a halt instruction was loaded and no more
    /// instructions should be fetched from memory.
    pub halt_encountered: bool,

    /// If control unit in no pipeline mode this stores the instruction which was
    /// just executed. Otherwise instructions are stored by stage in the following
    /// *_instruction fields.
    pub no_pipeline_instruction: Option<Box<dyn Instruction>>,

    /// Instruction which resulted from the fetch stage of the pipeline.
    pub fetch_instruction: Option<Box<dyn Instruction>>,

    /// Bits associated with fetch stage of pipeline.
    fetch_instruction_bits: u32,

    /// Instruction currently in the decode stage of the pipeline.
    pub decode_instruction: Option<Box<dyn Instruction>>,

    /// Instruction currently in the execute stage of the pipeline.
    pub execute_instruction: Option<Box<dyn Instruction>>,

    /// Instruction currently in the access memory stage of the pipeline.
    pub access_mem_instruction: Option<Box<dyn Instruction>>,

    /// Instruction currently in the write back stage of the pipeline.
    pub write_back_instruction: Option<Box<dyn Instruction>>,
}

/// Prepends 4 spaces to every line.
fn indent(src: String) -> String {
    let mut out = String::new();

    let mut i = 0;
    for line in src.lines() {
        out.push_str("    ");
        out.push_str(line);

        if i + 1 != src.lines().count() {
            out.push_str("\n");
        }
        
        i += 1;
    }

    out
}

impl fmt::Display for ControlUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instructions_str =  match self.pipeline_enabled {
            true => format!("\
Instructions:
    Fetch        : {:?}
    Decode       : {:?}
    Execute      : {:?}
    Access Memory: {:?}
    Write Back   : {:?}",
               self.fetch_instruction, self.decode_instruction,
               self.execute_instruction, self.access_mem_instruction,
               self.write_back_instruction),
            false => format!("\
Instruction : {:?}", self.no_pipeline_instruction),
        };
        
        write!(f, "\
Pipeline   : {}
Cache      : {}
Halted     : {}
Cycle Count: {}
Registers  :
{}
{}",
               self.pipeline_enabled, self.cache_enabled, self.halt_encountered,
               self.cycle_count, indent(format!("{}", self.registers)),
               instructions_str)
    }
}

impl ControlUnit {
    /// Creates a new ControlUnit.
    pub fn new(dram: Rc<RefCell<dyn Memory<u32, u32>>>, cache: Rc<RefCell<dyn Memory<u32, u32>>>) -> ControlUnit {
        ControlUnit{
            pipeline_enabled: true,
            cache_enabled: true,
            cycle_count: 0,
            registers: Registers::new(),
            dram: dram,
            cache: cache,
            first_instruction_loaded: false,
            halt_encountered: false,
            no_pipeline_instruction: None,
            fetch_instruction: None,
            fetch_instruction_bits: 0,
            decode_instruction: None,
            execute_instruction: None,
            access_mem_instruction: None,
            write_back_instruction: None,
        }
    }
    
    /// Step one instruction through the processor. Stores resulting state in self.
    /// If Result::Ok is returned the value embedded indicates if the program
    /// should keep running. False indicates it should not.
    pub fn step(&mut self) -> Result<bool, String> {
        self.first_instruction_loaded = true;

        let memory = match self.cache_enabled {
            true => self.cache.clone(),
            false => self.dram.clone(),
        };

        if self.pipeline_enabled {
            self.step_pipeline(memory)
        } else {
            self.step_no_pipeline(memory)
        }
    }

    /// Step one instruction through the processor without a pipeline. See step()
    /// for return documentation.
    pub fn step_no_pipeline(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> Result<bool, String> {
        if self.halt_encountered {
            return Ok(false);
        }
        
        // Fetch instruction
        let mut ibits: u32 = 0;

        let mut no_pipeline_inst = match memory.clone().borrow_mut().get(self.registers[PC]) {
            SimResult::Err(e) => return Err(
                format!("Failed to retrieve instruction from address {}: {}",
                        self.registers[PC], e)),
            SimResult::Wait(wait, fetched_bits) => {
                // Figure out which instruction the bits represent by
                // looking at the type and operation code.
                let icreate = self.instruction_factory(fetched_bits);

                // Set state
                self.cycle_count += wait as u32;
                ibits = fetched_bits;

                match icreate {
                    Err(e) => return Err(format!("Failed to determine type of \
                                                  instruction for bits {}: {}",
                                                 fetched_bits, e)),
                    Ok(v) => v,
                }
            },
        };
        
        // Decode instruction
        match no_pipeline_inst.decode(self.fetch_instruction_bits,
                                &self.registers) {
            SimResult::Err(e) => return Err(
                format!("Failed to decode instruction: {}",
                        e)),
            SimResult::Wait(wait, _v) => {
                // Update state
                self.cycle_count += wait as u32;
                
            },
        };

        // Execute instruction
        match no_pipeline_inst.execute() {
            SimResult::Err(e) => return Err(format!("Failed to execute \
                                                     instruction: {}", e)),
            SimResult::Wait(wait, _v) => {
                // Update state
                self.cycle_count += wait as u32;
            },
        };

        // Access memory
        match no_pipeline_inst.access_memory(memory.clone()) {
            SimResult::Err(e) => return Err(
                format!("Failed to access memory for instruction: {}",
                        e)),
            SimResult::Wait(wait, _v) => {
                // Update state
                self.cycle_count += wait as u32;
            },
        };

        // Write back
        match no_pipeline_inst.write_back(&mut self.registers) {
            SimResult::Err(e) => return Err(
                format!("Failed to write back for instruction: {}",
                        e)),
            SimResult::Wait(wait, _v) => {
                // Update state
                self.cycle_count += wait as u32;
            },
        };

        // Update state
        self.no_pipeline_instruction = Some(no_pipeline_inst);
        self.registers[PC] += 1;
        self.cycle_count += 5;

        // Determine if program should continue running
        Ok(self.program_is_running())
    }

    /// Step one instruction through the processor using the pipeline. See step()
    /// for return documentation.
    pub fn step_pipeline(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> Result<bool, String> {
        //  Write back stage
        match &mut self.access_mem_instruction {
            None => self.write_back_instruction = None,
            Some(access_mem_inst) => {
                match access_mem_inst.write_back(&mut self.registers) {
                    SimResult::Err(e) => return Err(
                        format!("Failed to write back for instruction: {}",
                                e)),
                    SimResult::Wait(wait, _v) => {
                        // Update state
                        self.cycle_count += wait as u32;
                    },
                };

                self.write_back_instruction = self.access_mem_instruction.take();
            },
        }
        
        // Access memory stage
        match &mut self.execute_instruction {
            None => self.access_mem_instruction = None,
            Some(exec_inst) => {
                console::log_1(&JsValue::from_serde(
                    &format!("control unit access memory stage")
                ).unwrap());
                
                match exec_inst.access_memory(memory.clone()) {
                    SimResult::Err(e) => return Err(
                        format!("Failed to access memory for instruction: {}",
                                e)),
                    SimResult::Wait(wait, _v) => {
                        // Update state
                        self.cycle_count += wait as u32;
                    },
                };

                self.access_mem_instruction = self.execute_instruction.take();
            },
        };
        
        // Execute stage
        match &mut self.decode_instruction {
            None => self.execute_instruction = None,
            Some(decode_inst) => {
                match decode_inst.execute() {
                    SimResult::Err(e) => return Err(format!("Failed to execute instruction: {}", e)),
                    SimResult::Wait(wait, _v) => {
                        // Update state
                        self.cycle_count += wait as u32;
                    },
                };

                self.execute_instruction = self.decode_instruction.take();
            },
        };

        // Decode stage
        match &mut self.fetch_instruction {
            None => self.decode_instruction = None,
            Some(fetch_inst) => {
                match fetch_inst.decode(self.fetch_instruction_bits,
                                        &self.registers) {
                    SimResult::Err(e) => return Err(
                        format!("Failed to decode instruction {}: {}",
                                fetch_inst, e)),
                    SimResult::Wait(wait, _v) => {
                        // Update state
                        self.cycle_count += wait as u32;
                        
                    },
                };

                self.decode_instruction = self.fetch_instruction.take();
            },
        };
    
        // Fetch stage
        if !self.halt_encountered {
            console::log_1(&JsValue::from_serde(
                &format!("fetching {}", self.registers[PC])
            ).unwrap());
            match memory.clone().borrow_mut().get(self.registers[PC]) {
                SimResult::Err(e) => return Err(
                    format!("Failed to retrieve instruction from address {}: {}",
                            self.registers[PC], e)),
                SimResult::Wait(wait, ibits) => {
                    console::log_1(&JsValue::from_serde(
                        &format!("fetched {} = {}", self.registers[PC], ibits)
                    ).unwrap());
                    // Figure out which instruction the bits represent by
                    // looking at the type and operation code.
                    let icreate = self.instruction_factory(ibits);

                    self.fetch_instruction = match icreate {
                        Err(e) => return Err(format!("Failed to determine type of \
                                                      instruction for bits {}: {}",
                                                     ibits, e)),
                        Ok(v) => Some(v),
                    };
                    self.fetch_instruction_bits = ibits;

                    // Set state
                    self.cycle_count += wait as u32;
                },
            };
        } else {
            self.fetch_instruction = None;
        }

        // Update state after all stages
        self.registers[PC] += 1;
        self.cycle_count += 1;

        // Determine if program should continue running
        Ok(self.program_is_running())
    }

    /// Initializes an instruction data structure based on instruction bits.
    fn instruction_factory(&mut self, ibits: u32) ->
        Result<Box<dyn Instruction>, String> {
            let itype = ibits.get_bits(5..=6) as u32;
            
            // Match instruction type
            match InstructionT::match_val(itype) {
                Some(InstructionT::Memory) => {
                    let iop = ibits.get_bits(7..=9) as u32;

                    match MemoryOp::match_val(iop) {
                        Some(MemoryOp::LoadRD) => Ok(Box::new(
                            Load::new(AddrMode::RegisterDirect))),
                        Some(MemoryOp::LoadI) => Ok(Box::new(
                            Load::new(AddrMode::Immediate))),
                        Some(MemoryOp::StoreRD) => Ok(Box::new(
                            Store::new(AddrMode::RegisterDirect))),
                        Some(MemoryOp::StoreI) => Ok(Box::new(
                            Store::new(AddrMode::Immediate))),
                        Some(MemoryOp::Push) => Ok(Box::new(
                            Push::new())),
                        Some(MemoryOp::Pop) => Ok(Box::new(
                            Pop::new())),
                        _ => Err(format!("Invalid operation code {} for \
                                          mememory type instruction", iop)),
                    }
                },

                // Subrouting/notsub
                // Sub = true
                // notsub = false
                Some(InstructionT::Control) => {
                    let iop = ibits.get_bits(7..=9) as u32;
                    match ControlOp::match_val(iop) {
                        Some(ControlOp::Halt) => {
                            self.halt_encountered = true;
                            Ok(Box::new(Halt::new()))
                        },
                        Some(ControlOp::JmpRD) => Ok(Box::new(
                            Jump::new(AddrMode::RegisterDirect, false))),
                        Some(ControlOp::JmpI) => Ok(Box::new(
                            Jump::new(AddrMode::Immediate, false))),
                        Some(ControlOp::JmpSRD) => Ok(Box::new(
                            Jump::new(AddrMode::RegisterDirect, true))),
                        Some(ControlOp::JmpSI) => Ok(Box::new(
                            Jump::new(AddrMode::Immediate, true))),
                        // Some(ControlOp::Sih) => Ok(Box::new(
                        //     SIH::new())),
                        // Some(ControlOp::IntRD) => Ok(Box::new(
                        //     INT::new(AddrMode::RegisterDirect))),
                        // Some(ControlOp::IntI) => Ok(Box::new(
                        //     INT::new(AddrMode::Immediate))),
                        Some(ControlOp::RFI) => Ok(Box::new(
                            RFI::new())),
                        Some(ControlOp::Noop) => Ok(Box::new(
                            Noop::new())),
                        _ => Err(format!("Invalid operation code {} for \
                                          Control type instruction", iop)),
                    }
                }

                // sign/unsign:
                // Unsigned = false
                // Signed = true
                Some(InstructionT::ALU) => {
                    let iop = ibits.get_bits(7..=12) as u32;

                    match ALUOp::match_val(iop) {    // Don't quite know how to add sign/unsign
                        Some(ALUOp::Move) => Ok(Box::new(
                            Move::new())),
                        // ---- Add ----
                        Some(ALUOp::AddUIRD) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::RegisterDirect, ArithMode::Add))),
                        Some(ALUOp::AddUII) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::Immediate, ArithMode::Add))),
                        Some(ALUOp::AddSIRD) => Ok(Box::new(
                            ArithSign::new(AddrMode::RegisterDirect, ArithMode::Add))),
                        Some(ALUOp::AddSII) => Ok(Box::new(
                            ArithSign::new(AddrMode::Immediate, ArithMode::Add))),
                        // ---- Sub ----
                        Some(ALUOp::SubUIRD) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::RegisterDirect, ArithMode::Sub))),
                        Some(ALUOp::SubUII) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::Immediate, ArithMode::Sub))),
                        Some(ALUOp::SubSIRD) => Ok(Box::new(
                            ArithSign::new(AddrMode::RegisterDirect, ArithMode::Sub))),
                        Some(ALUOp::SubSII) => Ok(Box::new(
                            ArithSign::new(AddrMode::Immediate, ArithMode::Sub))),
                        // ---- Mul ----
                        Some(ALUOp::MulUIRD) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::RegisterDirect, ArithMode::Mul))),
                        Some(ALUOp::MulUII) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::Immediate, ArithMode::Mul))),
                        Some(ALUOp::MulSIRD) => Ok(Box::new(
                            ArithSign::new(AddrMode::RegisterDirect, ArithMode::Mul))),
                        Some(ALUOp::MulSII) => Ok(Box::new(
                            ArithSign::new(AddrMode::Immediate, ArithMode::Mul))),
                        // ---- Div ----
                        Some(ALUOp::DivUIRD) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::RegisterDirect, ArithMode::Div))),
                        Some(ALUOp::DivUII) => Ok(Box::new(
                            ArithUnsign::new(AddrMode::Immediate, ArithMode::Div))),
                        Some(ALUOp::DivSIRD) => Ok(Box::new(
                            ArithSign::new(AddrMode::RegisterDirect, ArithMode::Div))),
                        Some(ALUOp::DivSII) => Ok(Box::new(
                            ArithSign::new(AddrMode::Immediate, ArithMode::Div))),
                        // ---- Comp ----
                        Some(ALUOp::Comp) => Ok(Box::new(
                            Comp::new())),
                        // ---- Arithmetic Shift ----
                        Some(ALUOp::ASLRD) => Ok(Box::new(
                            AS::new(AddrMode::RegisterDirect, false))),
                        Some(ALUOp::ASLI) => Ok(Box::new(
                            AS::new(AddrMode::Immediate, false))),
                        Some(ALUOp::ASRRD) => Ok(Box::new(
                            AS::new(AddrMode::RegisterDirect, true))),
                        Some(ALUOp::ASRI) => Ok(Box::new(
                            AS::new(AddrMode::Immediate, true))),
                        // ---- Logical Shift ----
                        Some(ALUOp::LSLRD) => Ok(Box::new(
                            LS::new(AddrMode::RegisterDirect, false))),
                        Some(ALUOp::LSLI) => Ok(Box::new(
                            LS::new(AddrMode::Immediate, false))),
                        Some(ALUOp::LSRRD) => Ok(Box::new(
                            LS::new(AddrMode::RegisterDirect, true))),
                        Some(ALUOp::LSRI) => Ok(Box::new(
                            LS::new(AddrMode::Immediate, true))),
                        // ---- 3 Operation Logic ----
                        Some(ALUOp::AndRD) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::RegisterDirect, LogicType::And))),
                        Some(ALUOp::AndI) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::Immediate, LogicType::And))),
                        Some(ALUOp::OrRD) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::RegisterDirect, LogicType::Or))),
                        Some(ALUOp::OrI) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::Immediate, LogicType::Or))),
                        Some(ALUOp::XorRD) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::RegisterDirect, LogicType::Xor))),
                        Some(ALUOp::XorI) => Ok(Box::new(
                            ThreeOpLogic::new(AddrMode::Immediate, LogicType::Xor))),
                        // ---- Not ----
                        Some(ALUOp::Not) => Ok(Box::new(
                            Not::new())),
                        
                        _ => Err(format!("Invalid operation code {} for \
                                          ALU type instruction", iop)),
                    }
                }
                _ => Err(format!("Invalid type value {} for instruction",
                                 itype)),
            }
        }

    /// Returns if the program should keep running.
    pub fn program_is_running(&self) -> bool {
        if self.pipeline_enabled {
            !self.first_instruction_loaded ||
                self.decode_instruction.is_some() ||
                self.fetch_instruction.is_some() ||
                self.execute_instruction.is_some() ||
                self.access_mem_instruction.is_some()
        } else {
            !self.first_instruction_loaded ||
                self.no_pipeline_instruction.is_some()
        }
    }
}
