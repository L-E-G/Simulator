use bit_field::BitField;

use std::fmt;
use std::fmt::{Debug,Display};
use std::cell::RefCell;
use std::rc::Rc;

use crate::result::SimResult;
use crate::memory::{Memory,DRAM,Registers,PC,STS,LR,IHDLR,INTLR,SP};

/// Defines operations which a single instruction must perform while it is in
/// the pipeline.
pub trait Instruction: Display + Debug {
    /// Extracts parameters from instruction bits and stores them in the
    /// implementing struct for use by future stages. It also retrieves register
    /// values if necessary and does the same.
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String>;

    /// Executes the instruction.
    fn execute(&mut self) -> SimResult<(), String>;

    /// Accesses memory.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String>;

    /// Write results to registers.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String>;
}

/// An instruction which performs no operations.
#[derive(Debug)]
pub struct Noop {}

impl Noop {
    pub fn new() -> Noop {
        Noop{}
    }
}

impl Display for Noop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Noop")
    }
}

impl Instruction for Noop {
    fn decode(&mut self, _instruction: u32, _registers: &Registers) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }

    fn execute(&mut self) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }

    fn access_memory(&mut self, _memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }

    fn write_back(&mut self, _registers: &mut Registers) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }
}

/// Identifies types of instructions.
#[derive(PartialEq,Debug)]
pub enum InstructionT {
    ALU,
    Memory,
    Control,
    Graphics,
}

impl InstructionT {
    /// Returns the value of the type field for the represented instruction type.
    pub fn value(self) -> u32 {
        match self {
            InstructionT::Control => 0,
            InstructionT::ALU => 1,
            InstructionT::Memory => 2,
            InstructionT::Graphics => 3,
        }
    }

    /// Matches a value to an instruction type.
    pub fn match_val(val: u32) -> Option<InstructionT> {
        match val {
            0 => Some(InstructionT::Control),
            1 => Some(InstructionT::ALU),
            2 => Some(InstructionT::Memory),
            3 => Some(InstructionT::Graphics),
            _ => None,
        }
    }
}

pub enum InterruptCodes {
    UPARROW, DOWNARROW, LEFTARROW, 
    RIGHTARROW, ENTER, ESCAPE, SPACE, 
    NOT_SET_INITIAL, NOT_SET, SET
}

impl InterruptCodes {
    pub fn value(self) -> usize {
        match self {
            InterruptCodes::UPARROW => 0000,
            InterruptCodes::DOWNARROW => 0001,
            InterruptCodes::LEFTARROW => 0010,
            InterruptCodes::RIGHTARROW => 0011,
            InterruptCodes::ENTER => 0100,
            InterruptCodes::ESCAPE => 0101,
            InterruptCodes::SPACE => 0110,
            InterruptCodes::NOT_SET_INITIAL => 111111,
            InterruptCodes::NOT_SET => 000000,
            InterruptCodes::SET => 100000,
        }
    }
}

pub enum ConditionCodes {
    NS, NE, E, GT, LT,
    GTE, LTE, OF, Z, NZ,
    NEG, POS,
}

impl ConditionCodes {
    pub fn value(self) -> u32 {
        match self {
            ConditionCodes::NS => 0,
            ConditionCodes::NE => 1,
            ConditionCodes::E => 2,
            ConditionCodes::GT => 3,
            ConditionCodes::LT => 4,
            ConditionCodes::GTE => 5,
            ConditionCodes::LTE => 6,
            ConditionCodes::OF => 7,
            ConditionCodes::Z => 8,
            ConditionCodes::NZ => 9,
            ConditionCodes::NEG => 10,
            ConditionCodes::POS => 11,
        }
    }
}

/// Identifies the addressing mode of an instruction operand.
#[derive(PartialEq,Debug)]
pub enum AddrMode {
    /// Value is contained in the specified register.
    RegisterDirect,

    /// Value is the operand.
    Immediate,
}

impl Display for AddrMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddrMode::RegisterDirect => write!(f, "RD"),
            AddrMode::Immediate => write!(f, "I"),
        }
    }
}

#[derive(PartialEq,Debug)]
pub enum ArithMode {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for ArithMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArithMode::Add => write!(f, "Add"),
            ArithMode::Sub => write!(f, "Sub"),
            ArithMode::Mul => write!(f, "Mult"),
            ArithMode::Div => write!(f, "Div"),
        }
    }
}

#[derive(PartialEq,Debug)]
pub enum LogicType {
    And,
    Or,
    Xor,
}

impl Display for LogicType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicType::And => write!(f, "And"),
            LogicType::Or => write!(f, "Or"),
            LogicType::Xor => write!(f, "Xor"),
        }
    }
}

/// Identifies memory operations.
#[derive(PartialEq,Debug)]
pub enum MemoryOp {
    LoadRD, LoadI,
    StoreRD, StoreI,
    Push,
    Pop,
}

impl MemoryOp {
    /// Returns the value of the operation field for the represented operation.
    pub fn value(self) -> u32 {
        match self {
            MemoryOp::LoadRD => 0,
            MemoryOp::LoadI => 1,
            MemoryOp::StoreRD => 2,
            MemoryOp::StoreI => 3,
            MemoryOp::Push => 4,
            MemoryOp::Pop => 5,
        }
    }

    /// Matches a value with a MemoryOp.
    pub fn match_val(val: u32) -> Option<MemoryOp> {
        match val {
            0 => Some(MemoryOp::LoadRD),
            1 => Some(MemoryOp::LoadI),
            2 => Some(MemoryOp::StoreRD),
            3 => Some(MemoryOp::StoreI),
            4 => Some(MemoryOp::Push),
            5 => Some(MemoryOp::Pop),
            _ => None,
        }
    }
}

/// UI = Unsigned Integer
/// SI = Signed Integer
/// RD = Register Direct
/// I  = Immediate
#[derive(PartialEq,Debug)]
pub enum ALUOp {
    AddUIRD, AddUII, AddSIRD, AddSII,
    SubUIRD, SubUII, SubSIRD, SubSII,
    MulUIRD, MulUII, MulSIRD, MulSII, 
    DivUIRD, DivUII, DivSIRD, DivSII,
    Move, 
    Comp,
    ASLRD, ASLI, ASRRD, ASRI,
    LSLRD, LSLI, LSRRD, LSRI,
    AndRD, AndI,
    OrRD, OrI,
    XorRD, XorI,
    Not, 
}
impl ALUOp {
    /// Returns the value of the operation field for the represented operation.
    pub fn value(self) -> u32 {
        match self {
            ALUOp::AddUIRD => 0,
            ALUOp::AddUII => 1,
            ALUOp::AddSIRD => 2,
            ALUOp::AddSII => 3,
            ALUOp::SubUIRD => 4,
            ALUOp::SubUII => 5,
            ALUOp::SubSIRD => 6,
            ALUOp::SubSII => 7,
            ALUOp::MulUIRD => 8,
            ALUOp::MulUII => 9,
            ALUOp::MulSIRD => 10,
            ALUOp::MulSII => 11,
            ALUOp::DivUIRD => 12,
            ALUOp::DivUII => 13,
            ALUOp::DivSIRD => 14,
            ALUOp::DivSII => 15,
            ALUOp::Move => 16,
            ALUOp::Comp => 17,
            ALUOp::ASLRD => 19,
            ALUOp::ASLI => 20,
            ALUOp::ASRRD => 21,
            ALUOp::ASRI => 22,
            ALUOp::LSLRD => 23,
            ALUOp::LSLI => 24,
            ALUOp::LSRRD => 25,
            ALUOp::LSRI => 26,
            ALUOp::AndRD => 27,
            ALUOp::AndI => 28,
            ALUOp::OrRD => 29,
            ALUOp::OrI => 30,
            ALUOp::XorRD => 31,
            ALUOp::XorI => 32,
            ALUOp::Not => 33,
        }
    }

    /// Matches a value with a MemoryOp.
    pub fn match_val(val: u32) -> Option<ALUOp> {
        match val {
            0 => Some(ALUOp::AddUIRD),
            1 => Some(ALUOp::AddUII),
            2 => Some(ALUOp::AddSIRD),
            3 => Some(ALUOp::AddSII),
            4 => Some(ALUOp::SubUIRD),
            5 => Some(ALUOp::SubUII),
            6 => Some(ALUOp::SubSIRD),
            7 => Some(ALUOp::SubSII),
            8 => Some(ALUOp::MulUIRD),
            9 => Some(ALUOp::MulUII),
            10 => Some(ALUOp::MulSIRD),
            11 => Some(ALUOp::MulSII),
            12 => Some(ALUOp::DivUIRD),
            13 => Some(ALUOp::DivUII),
            14 => Some(ALUOp::DivSIRD),
            15 => Some(ALUOp::DivSII),
            16 => Some(ALUOp::Move),
            17 => Some(ALUOp::Comp),
            19 => Some(ALUOp::ASLRD),
            20 => Some(ALUOp::ASLI),
            21 => Some(ALUOp::ASRRD),
            22 => Some(ALUOp::ASRI),
            23 => Some(ALUOp::LSLRD),
            24 => Some(ALUOp::LSLI),
            25 => Some(ALUOp::LSRRD),
            26 => Some(ALUOp::LSRI),
            27 => Some(ALUOp::AndRD),
            28 => Some(ALUOp::AndI),
            29 => Some(ALUOp::OrRD),
            30 => Some(ALUOp::OrI),
            31 => Some(ALUOp::XorRD),
            32 => Some(ALUOp::XorI),
            33 => Some(ALUOp::Not),
            _ => None,
        }
    }
}

#[derive(PartialEq,Debug)]
pub enum ControlOp {
    JmpRD, JmpI,
    JmpSRD, JmpSI,
    // Sih,
    // IntRD, IntI, 
    RFI,
    Halt,
    Noop,
}

impl ControlOp {
    /// Returns the value of the operation field for the represented operation.
    pub fn value(self) -> u32 {
        match self {
            ControlOp::Halt => 0,
            ControlOp::JmpRD => 1,
            ControlOp::JmpI => 2,
            ControlOp::JmpSRD => 3,
            ControlOp::JmpSI => 4,
            // ControlOp::Sih => 1,
            // ControlOp::IntRD => 1,
            // ControlOp::IntI => 1,
            ControlOp::RFI => 5,
            ControlOp::Noop => 6,
        }
    }

    /// Matches a value with a MemoryOp.
    pub fn match_val(val: u32) -> Option<ControlOp> {
        match val {
            0 => Some(ControlOp::Halt),
            1 => Some(ControlOp::JmpRD),
            2 => Some(ControlOp::JmpI),
            3 => Some(ControlOp::JmpSRD),
            4 => Some(ControlOp::JmpSI),
            // 1 => Some(ControlOp::Sih),
            // 1 => Some(ControlOp::IntRD),
            // 1 => Some(ControlOp::IntI),
            5 => Some(ControlOp::RFI),
            6 => Some(ControlOp::Noop),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Halt {}

impl Halt {
    pub fn new() -> Halt {
        Halt{}
    }
}

impl Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Halt")
    }
}

impl Instruction for Halt {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }
}

// ---------------------------------- Memory Instructions ----------------------------------

/// Read a value from an address in memory and place it in a register.
#[derive(Debug)]
pub struct Load {
    /// Indicates the addressing mode of the memory address operand.
    mem_addr_mode: AddrMode,
    
    /// Register to place value from memory.
    dest_reg: usize,

    /// Memory address to load into register.
    mem_addr: u32,

    /// Value loaded from mememory during access_memory.
    value: u32,
}

impl Display for Load {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Load Instruction ({})", self.mem_addr_mode)
    }
}

impl Load {
    /// Creates an empty load instruction.
    pub fn new(mem_addr_mode: AddrMode) -> Load {
        Load{
            mem_addr_mode: mem_addr_mode,
            dest_reg: 0,
            mem_addr: 0,
            value: 0,
        }
    }
}

impl Instruction for Load {
    /// Extract dest_reg and mem_addr operands.
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.dest_reg = instruction.get_bits(10..=14) as usize;
        
        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.mem_addr = registers[instruction.get_bits(15..=19) as usize];
        } else if self.mem_addr_mode == AddrMode::Immediate {
            // self.mem_addr = instruction.get_bits(15..=19) as u32;
            self.mem_addr = (((registers[PC] + 1) as i32) + (instruction.get_bits(15..=31) as i32)) as u32;
        }

        return SimResult::Wait(0, ());
    }

    /// No execute step.
    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Load value at mem_addr from memory into value.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        match memory.borrow_mut().get(self.mem_addr) {
            SimResult::Err(e) => SimResult::Err(
                format!("failed to retrieve memory address {}: {}",
                        self.mem_addr, e)),
            SimResult::Wait(wait, val) => {
                self.value = val;
                SimResult::Wait(wait, ())
            },
        }
    }

    /// Write value from memory into register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest_reg] = self.value;
        SimResult::Wait(0, ())
    }
}

/// Writes a value in memory from a register.
#[derive(Debug)]
pub struct Store {
    /// Address mode of instruction.
    mem_addr_mode: AddrMode,
    /// Address in memory to save value.
    dest_addr: u32,

    /// Value in register to save in memory.
    value: u32,
}

impl Store {
    /// Create an empty store instruction.
    pub fn new(mem_addr_mode: AddrMode) -> Store {
        Store{
            mem_addr_mode: mem_addr_mode,
            dest_addr: 0,
            value: 0,
        }
    }
}

impl Display for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Store Instruction")
    }
}

impl Instruction for Store {
    /// Extract operands and retrieve value to save in memory from registers.
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.dest_addr = registers[instruction.get_bits(10..=14) as usize] as u32;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.value = registers[instruction.get_bits(15..=19) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.value = (((registers[PC] + 1) as i32) + (instruction.get_bits(15..=31) as i32)) as u32;
        }

        SimResult::Wait(0, ())
    }

    /// No execution stage.
    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Set address in memory to value.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        match memory.borrow_mut().set(self.dest_addr, self.value) {
            SimResult::Err(e) => SimResult::Err(
                format!("Failed to store value in {}: {}", self.dest_addr, e)),
            SimResult::Wait(wait, _res) => SimResult::Wait(wait, ()),
        }
    }

    /// No write back stage.
    fn write_back(&mut self, _registers: &mut Registers) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }
}

#[derive(Debug)]
pub struct Push {
    addr: u32,
    value: u32,
}

impl Push {
    pub fn new() -> Push {
        Push{
            addr: 0,
            value: 0,
        }
    }
}

impl Display for Push {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Push")
    }
}

impl Instruction for Push {
    /// Extract operands and retrieve value to save in memory from registers.
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.addr = registers[instruction.get_bits(11..=15) as usize] as u32;
        self.value = registers[SP] - 1;
        SimResult::Wait(0, ())
    }

    /// No execution stage.
    fn execute(&mut self) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }

    /// Set address in memory to value.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        match memory.borrow_mut().set(self.addr, self.value) {
            SimResult::Err(e) => SimResult::Err(
                format!("Failed to Push value in {}: {}", self.addr, e)),
            SimResult::Wait(wait, _res) => SimResult::Wait(wait, ()),
        }
    }

    /// No write back stage.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[SP] -= 1;
        SimResult::Wait(0, ())
    }
}

#[derive(Debug)]
pub struct Pop {
    dest: usize,
    addr: u32,
    value: u32,
}

impl Pop {
    pub fn new() -> Pop {
        Pop{
            dest: 0,
            addr: 0,
            value: 0,
        }
    }
}

impl Display for Pop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pop")
    }
}

impl Instruction for Pop {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.dest = instruction.get_bits(11..=15) as usize;
        self.addr = registers[SP];
        SimResult::Wait(0, ())
    }

    /// No execution stage.
    fn execute(&mut self) -> SimResult<(), String> {
        SimResult::Wait(0, ())
    }

    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        match memory.borrow_mut().get(self.addr) {
            SimResult::Err(e) => SimResult::Err(
                format!("failed to Pop {}: {}",
                        self.addr, e)),
            SimResult::Wait(wait, val) => {
                self.value = val;
                SimResult::Wait(wait, ())
            },
        }
    }

    /// No write back stage.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.value;
        registers[SP] += 1;
        SimResult::Wait(0, ())
    }
}

// ---------------------------------- ALU Instructions ----------------------------------

#[derive(Debug)]
pub struct Move {
    dest: usize,
    value: u32,
}

impl Move {
    pub fn new() -> Move {
        Move{
            dest: 0,
            value: 0,
        }
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Move Instruction")
    }
}

impl Instruction for Move {
    /// Convert instruction to String, then to &str so we can convert it to a usize
    /// so that we can perform binary operations on it.
    /// Extract destination register from the instruction.
    /// Extract source register that holds the value to move.
    /// Get the value to move and add it to the value field.
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.value = registers[instruction.get_bits(18..=22) as usize];

        self.dest = instruction.get_bits(13..=17) as usize;

        return SimResult::Wait(0, ());
    }

    /// No execution stage.
    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// No memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Set the value of the destination register to the value from the source register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.value;
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct ArithSign {
    mem_addr_mode: AddrMode,
    dest: usize,
    operation: ArithMode,
    op1: i32,
    op2: i32,
    result: i32,
}

impl ArithSign {
    pub fn new(mem_addr_mode: AddrMode, operation: ArithMode) -> ArithSign {
        ArithSign{
            mem_addr_mode: mem_addr_mode,
            operation: operation,
            dest: 0,
            op1: 0,
            op2: 0,
            result: 0,
        }
    }
}

impl Display for ArithSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} signed", self.operation)
    }
}

/// The one instruction that takes care of all arithmetic instructions
impl Instruction for ArithSign {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.dest = instruction.get_bits(14..=18) as usize;

        self.op1 = registers[instruction.get_bits(19..=23) as usize] as i32;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.op2 = registers[instruction.get_bits(24..=28) as usize] as i32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.op2 = instruction.get_bits(24..=31) as i32;
        }
        
        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        match self.operation {
            ArithMode::Add => self.result = self.op1 + self.op2,
            ArithMode::Sub => self.result = self.op1 - self.op2,
            ArithMode::Mul => self.result = self.op1 * self.op2,
            ArithMode::Div => self.result = self.op1 / self.op2,
        }
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.result as u32;
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct ArithUnsign {
    mem_addr_mode: AddrMode,
    dest: usize,
    operation: ArithMode,
    op1: u32,
    op2: u32,
    result: u32,
}

impl ArithUnsign {
    pub fn new(mem_addr_mode: AddrMode, operation: ArithMode) -> ArithUnsign {
        ArithUnsign{
            mem_addr_mode: mem_addr_mode,
            operation: operation,
            dest: 0,
            op1: 0,
            op2: 0,
            result: 0,
        }
    }
}

impl Display for ArithUnsign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} unsigned", self.operation)
    }
}

/// The one instruction that takes care of all arithmetic instructions
impl Instruction for ArithUnsign {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.dest = instruction.get_bits(13..=17) as usize;

        self.op1 = registers[instruction.get_bits(18..=22) as usize] as u32;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.op2 = registers[instruction.get_bits(23..=27) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.op2 = instruction.get_bits(23..=31) as u32;
        }
        
        return SimResult::Wait(0, ());
        // return SimResult::Err(format!("Instruction details: dest: {}, op1: {}, op2: {}",self.dest, self.op1, self.op2));
    }

    fn execute(&mut self) -> SimResult<(), String> {
        match self.operation {
            ArithMode::Add => {
                self.result = self.op1 + self.op2;
                // return SimResult::Err(format!("Instruction details: result: {}, op1: {}, op2: {}",self.result, self.op1, self.op2));
            },
            ArithMode::Sub => self.result = self.op1 - self.op2,
            ArithMode::Mul => self.result = self.op1 * self.op2,
            ArithMode::Div => self.result = self.op1 / self.op2,
        }
        return SimResult::Wait(0, ());
        // return SimResult::Err(format!("Instruction details: result: {}, op1: {}, op2: {}",self.result, self.op1, self.op2));
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.result as u32;
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct Comp {
    op1: u32,
    op2: u32,
}

impl Comp {
    pub fn new() -> Comp {
        Comp{
            op1: 0,
            op2: 0,
        }
    }
}

impl Display for Comp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Comp Instruction")
    }
}

impl Instruction for Comp {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.op1 = registers[instruction.get_bits(13..=17) as usize] as u32;

        self.op2 = registers[instruction.get_bits(18..=22) as usize] as u32;
        
        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        
        if self.op1 < self.op2 {
            registers[STS] = ConditionCodes::LT.value();
        } else if self.op1 > self.op2 {
            registers[STS] = ConditionCodes::GT.value();
        } else {
            registers[STS] = ConditionCodes::E.value();
        }
        
        return SimResult::Wait(0, ());
    }
}


#[derive(Debug)]
pub struct AS {
    mem_addr_mode: AddrMode,
    direction: bool,
    dest: usize,
    op: u32,
    amount: u32,
    result: u32,
}

impl AS {
    // direction: Left = false, right = true
    pub fn new(mem_addr_mode: AddrMode, d: bool) -> AS {
        AS{
            mem_addr_mode: mem_addr_mode,
            direction: d,
            dest: 0,
            op: 0,
            amount: 0,
            result: 0,
        }
    }
}

impl Display for AS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Arithmetic Shift")
    }
}

impl Instruction for AS {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.dest = instruction.get_bits(13..=17) as usize;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.amount = registers[instruction.get_bits(18..=22) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.amount = instruction.get_bits(18..=31) as u32;
        }
        
        self.op = registers[self.dest] as u32;

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        if self.direction {
            self.result = self.op << self.amount;
        } else {
            self.result = self.op >> self.amount;
        }

        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.result;
        
        return SimResult::Wait(0, ());
    }
}


#[derive(Debug)]
pub struct LS {
    mem_addr_mode: AddrMode,
    direction: bool,
    dest: usize,
    op: i32,
    amount: i32,
    result: i32,
}

impl LS {
    // direction: Left = false, right = true
    pub fn new(mem_addr_mode: AddrMode, d: bool) -> LS {
        LS{
            mem_addr_mode: mem_addr_mode,
            direction: d,
            dest: 0,
            op: 0,
            amount: 0,
            result: 0,
        }
    }
}

impl Display for LS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Logical Shift")
    }
}

impl Instruction for LS {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.dest = instruction.get_bits(13..=17) as usize;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.amount = registers[instruction.get_bits(18..=22) as usize] as i32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.amount = instruction.get_bits(18..=31) as i32;
        }
        
        self.op = registers[self.dest] as i32;

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        if self.direction {
            self.result = self.op << self.amount;
        } else {
            self.result = self.op >> self.amount;
        }

        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.result as u32;
        
        return SimResult::Wait(0, ());
    }
}


#[derive(Debug)]
pub struct ThreeOpLogic {
    mem_addr_mode: AddrMode,
    OpType: LogicType,
    dest: usize,
    op1: u32,
    op2: u32,
    result: u32,
}

impl ThreeOpLogic {
    pub fn new(mem_addr_mode: AddrMode, LT: LogicType) -> ThreeOpLogic {
        ThreeOpLogic{
            mem_addr_mode: mem_addr_mode,
            OpType: LT,
            dest: 0,
            op1: 0,
            op2: 0,
            result: 0,
        }
    }
}

impl Display for ThreeOpLogic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "3 Operation Logic")
    }
}

impl Instruction for ThreeOpLogic {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {

        self.dest = instruction.get_bits(13..=17) as usize;

        self.op1 = registers[instruction.get_bits(18..=22) as usize] as u32;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.op2 = registers[instruction.get_bits(23..=27) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.op2 = instruction.get_bits(23..=31) as u32;
        }

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        match self.OpType {
            LogicType::And => self.result = self.op1 & self.op2,
            LogicType::Or => self.result = self.op1 | self.op2,
            LogicType::Xor => self.result = self.op1 ^ self.op2,
        }

        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = self.result;
        
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct Not {
    dest: usize,
    op: u32,
    result: u32,
}

impl Not {
    pub fn new() -> Not {
        Not{
            dest: 0,
            op: 0,
            result: 0,
        }
    }
}

impl Display for Not {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not")
    }
}

impl Instruction for Not {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.dest = instruction.get_bits(13..=17) as usize;

        self.op = registers[instruction.get_bits(18..=22) as usize] as u32;

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Store the value of the result in the destination register and invert it.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[self.dest] = !self.op;
        
        return SimResult::Wait(0, ());
    }
}

// ---------------------------------- Control Instructions ----------------------------------

#[derive(Debug)]
pub struct Jump {
    mem_addr_mode: AddrMode,
    is_sub: bool,
    condition: u32,
    addr: u32,
}

impl Jump {
    pub fn new(mem_addr_mode: AddrMode, is_sub: bool) -> Jump {
        Jump{
            mem_addr_mode: mem_addr_mode,
            is_sub: is_sub,
            condition: 0,
            addr: 0,
        }
    }
}

impl Display for Jump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Jump Instruction")
    }
}

impl Instruction for Jump {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.condition = instruction.get_bits(0..=4) as u32;

        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.addr = registers[instruction.get_bits(10..=14) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.addr = instruction.get_bits(10..=31) as u32;
        }

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        
        if self.condition != 0 {
            if self.condition == registers[STS] {
                if self.is_sub {
                    registers[LR] = (PC + 1) as u32;
                } 
                registers[PC] = self.addr;
                // else if self.mem_addr_mode == AddrMode::RegisterDirect {
                //     registers[PC] = self.addr;
                // }
                // else if self.mem_addr_mode == AddrMode::Immediate {
                //     registers[PC] += self.addr;
                // }
            }

        } else {
            registers[PC] = self.addr;
            // if self.mem_addr_mode == AddrMode::RegisterDirect {
            //     registers[PC] = self.addr;
            // }
            // else if self.mem_addr_mode == AddrMode::Immediate {
            //     registers[PC] += self.addr;
            // }
        }
        
        
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct SIH {
    addr: u32,
}

impl SIH {
    pub fn new() -> SIH {
        SIH{
            addr: 0,
        }
    }
}

impl Display for SIH {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SIH")
    }
}

impl Instruction for SIH {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        self.addr = instruction.get_bits(10..=14) as u32;

        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        registers[IHDLR] = self.addr;
        
        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct INT {
    mem_addr_mode: AddrMode,
    proceed: bool,
    code: u32,
    addr: u32,
}

impl INT {
    pub fn new(mem_addr_mode: AddrMode) -> INT {
        INT{
            mem_addr_mode: mem_addr_mode,
            proceed: false,
            code: 0,
            addr: 0,
        }
    }
}

impl Display for INT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Perform Interrupt")
    }
}

impl Instruction for INT {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        if self.mem_addr_mode == AddrMode::RegisterDirect {
            self.code = registers[instruction.get_bits(10..=14) as usize] as u32;
        } else if self.mem_addr_mode == AddrMode::Immediate {
            self.code = instruction.get_bits(10..=13) as u32;
        }

        if registers[STS] != InterruptCodes::NOT_SET as u32 && registers[IHDLR] != InterruptCodes::NOT_SET_INITIAL as u32 {
            self.proceed = true;
        }

        return SimResult::Wait(0, ());
    }

    /// Execute the binary operation using usize's function checked_add().
    /// Store value in result field.
    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        if self.proceed {
            match memory.borrow_mut().set(1111111111, self.code) {
                SimResult::Err(e) => SimResult::Err(format!("Failed to store interrupt code, value in {}: {}", self.code, e)),
                SimResult::Wait(wait, _res) => SimResult::Wait(wait, ()),
            }
        }
        else {return SimResult::Wait(0, ());}
    }

    /// Store the value of the result in the destination register.
    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {

        if self.proceed {
            self.proceed = true;
            registers[STS] = InterruptCodes::SET as u32;
            registers[INTLR] = registers[PC];
            registers[PC] = registers[IHDLR];
        }

        return SimResult::Wait(0, ());
    }
}

#[derive(Debug)]
pub struct RFI {}

impl RFI {
    pub fn new() -> RFI {
        RFI{}
    }
}

impl Display for RFI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Jump out of Interrupt")
    }
}

impl Instruction for RFI {
    fn decode(&mut self, instruction: u32, registers: &Registers) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn execute(&mut self) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    /// Skipped, no memory accessing.
    fn access_memory(&mut self, memory: Rc<RefCell<dyn Memory<u32, u32>>>) -> SimResult<(), String> {
        return SimResult::Wait(0, ());
    }

    fn write_back(&mut self, registers: &mut Registers) -> SimResult<(), String> {
        if registers[STS] != InterruptCodes::NOT_SET_INITIAL as u32 {
            registers[STS] = InterruptCodes::NOT_SET as u32;
            registers[PC] = registers[INTLR];
        }
        
        return SimResult::Wait(0, ());
    }
}

// ------------------------------------ Tests ---------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mockers::Scenario;
    
    /// Ensures that the load instruction functions correctly.
    #[test]
    fn test_load_instruction() {
        let scenario = Scenario::new();
        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        let mut regs = Registers::new();
        
        // Setup registers
        const DEST_VAL: usize = 444;
        const ADDR_VAL: u32 = 777;
        regs[ADDR_REG_IDX] = ADDR_VAL;

        // Pack instruction bits
        // dest = 10100 = R20
        // addr = 00110 = R6
        const DEST_REG_IDX: usize = 20;
        const ADDR_REG_IDX: usize = 6;

        // Setup memory
        const MEM_DELAY: u16 = 101;
        const MEM_VALUE: u32 = 567;
        
        // Test decode with register direct
        let mut load_instruction = Load::new(AddrMode::RegisterDirect);
        let mut INSTRUCTION_RD: u32 = 0;
        INSTRUCTION_RD.set_bits(10..=14, (DEST_REG_IDX as u32).get_bits(0..=4));
        INSTRUCTION_RD.set_bits(15..=19, (ADDR_REG_IDX as u32).get_bits(0..=4));
        
        assert_eq!(load_instruction.decode(INSTRUCTION_RD, &regs),
                   SimResult::Wait(0, ()),
                   "register direct, decode() == expected");
        assert_eq!(load_instruction.dest_reg, DEST_REG_IDX,
                   "register direct, .dest_reg == expected");
        assert_eq!(load_instruction.mem_addr, ADDR_VAL,
                   "register direct, .mem_addr == expected");

        // Test decode with immediate
        let mut INSTRUCTION_I: u32 = 0;
        INSTRUCTION_I.set_bits(10..=14, (DEST_REG_IDX as u32).get_bits(0..=4));
        INSTRUCTION_I.set_bits(15..=31, ADDR_VAL.get_bits(0..=16)
                               - (regs[PC] + 1));
        
        load_instruction = Load::new(AddrMode::Immediate);

        assert_eq!(load_instruction.decode(INSTRUCTION_I, &regs),
                   SimResult::Wait(0, ()),
                   "immediate, decode() == expected");
        assert_eq!(load_instruction.dest_reg, DEST_REG_IDX,
                   "immediate, .dest_reg == expected");
        assert_eq!(load_instruction.mem_addr, ADDR_VAL,
                   "immediate, .mem_addr == expected");

        // Test execute
        assert_eq!(load_instruction.execute(), SimResult::Wait(0, ()),
                   "execute() == expected");

        // Test access memory
        scenario.expect(memory_handle.get(ADDR_VAL)
                        .and_return(SimResult::Wait(MEM_DELAY, MEM_VALUE)));
        assert_eq!(load_instruction.access_memory(mem_ref),
                   SimResult::Wait(MEM_DELAY, ()), "access_memory() == expected");
        assert_eq!(load_instruction.value, MEM_VALUE,
                   ".value == expected");

        // Test write back
        let mut expected_wb_regs = regs.clone();
        expected_wb_regs[DEST_REG_IDX] = MEM_VALUE;
        
        assert_eq!(load_instruction.write_back(&mut regs),
                   SimResult::Wait(0, ()), "write_back() == expected");
        assert_eq!(regs, expected_wb_regs,
                   "regs == expected");
    }

    /// Ensures the store instruction functions properly
    #[test]
    fn test_store_instruction() {
        let scenario = Scenario::new();

        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        
        let mut regs = Registers::new();
        let mut store_instruction = Store::new(AddrMode::RegisterDirect);

        // Pack instruction operands
        // src = 00101 = R5
        // addr = 01000 = R8
        const SRC_REG_IDX: usize = 5;
        const ADDR_REG_IDX: usize = 8;
        let mut instruction: u32 = 0;
        instruction.set_bits(15..=19, (SRC_REG_IDX as u32).get_bits(0..=4));
        instruction.set_bits(10..=14, (ADDR_REG_IDX as u32).get_bits(0..=4));

        // Setup registers
        const DEST_ADDR: u32 = 34567;
        const SRC_VAL: u32 = 346;

        regs[SRC_REG_IDX] = SRC_VAL;
        regs[ADDR_REG_IDX] = DEST_ADDR;

        // Test decode
        assert_eq!(store_instruction.decode(instruction, &regs),
                   SimResult::Wait(0, ()), "decode() == expected");
        assert_eq!(store_instruction.value, SRC_VAL, ".value == expected");
        assert_eq!(store_instruction.dest_addr, DEST_ADDR,
                   ".dest_addr == expected");

        // Test execute
        assert_eq!(store_instruction.execute(), SimResult::Wait(0, ()),
                   "execute() == expected");

        // Test access memory
        const MEM_DELAY: u16 = 45;
        scenario.expect(memory_handle.set(DEST_ADDR, SRC_VAL)
                        .and_return(SimResult::Wait(MEM_DELAY, ())));
        assert_eq!(store_instruction.access_memory(mem_ref),
                   SimResult::Wait(MEM_DELAY, ()));

        // Test write back
        let expected_wb_regs = regs.clone();
        
        assert_eq!(store_instruction.write_back(&mut regs),
                   SimResult::Wait(0,()), "write_back == expected");
        assert_eq!(regs, expected_wb_regs, "regs == expected");
    }

    #[test]
    fn test_move_instruction() {
        let scenario = Scenario::new();

        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        
        let mut regs = Registers::new();

        let mut move_instruction = Move::new();

        const SRC: usize = 4;
        const DEST: usize = 5;
        let mut instruction: u32 = 0;
        instruction.set_bits(18..=22, (SRC as u32).get_bits(0..=4));
        instruction.set_bits(13..=17, (DEST as u32).get_bits(0..=4));

        const VAL: u32 = 69;

        regs[SRC] = VAL;

        assert_eq!(move_instruction.decode(instruction, &regs), SimResult::Wait(0, ()), "decode() == expected");
        assert_eq!(move_instruction.value, VAL, "VAL == instr.value");
        assert_eq!(move_instruction.dest, DEST, "DEST = instr.dest");

        assert_eq!(move_instruction.execute(), SimResult::Wait(0, ()), "execute() == expected");
        assert_eq!(move_instruction.access_memory(mem_ref), SimResult::Wait(0, ()), "access_memory() == expected");
        assert_eq!(move_instruction.write_back(&mut regs), SimResult::Wait(0, ()), "write_back() == expected");

        assert_eq!(regs[DEST], VAL);


    }

    #[test]
    fn test_add_reg_dir() {
        let scenario = Scenario::new();

        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        
        let mut regs = Registers::new();

        let mut add = ArithUnsign::new(AddrMode::RegisterDirect, ArithMode::Add);

        const REG1: usize = 10;
        const REG2: usize = 13;
        const DEST: usize = 2;
        let mut instruction: u32 = 0;
        instruction.set_bits(18..=22, (REG1 as u32).get_bits(0..=4));
        instruction.set_bits(23..=27, (REG2 as u32).get_bits(0..=4));
        instruction.set_bits(13..=17, (DEST as u32).get_bits(0..=4));

        const VAL1: u32 = 1;
        const VAL2: u32 = 2;
        const RESULT: u32 = VAL1 + VAL2;

        regs[REG1] = VAL1;
        regs[REG2] = VAL2;

        assert_eq!(add.decode(instruction, &regs), SimResult::Wait(0, ()), "decode() == expected");
        assert_eq!(add.op1, VAL1, "OP1 == instr.op1");
        assert_eq!(add.op2, VAL2, "OP2 == instr.op2");
        assert_eq!(add.dest, DEST, "DEST = instr.dest");
        assert_eq!(add.operation, ArithMode::Add, "operation == instr.operation");

        assert_eq!(add.execute(), SimResult::Wait(0, ()), "execute() == expected");
        assert_eq!(add.result, RESULT, "execute == worked");
        assert_eq!(add.access_memory(mem_ref), SimResult::Wait(0, ()), "access_memory() == expected");
        assert_eq!(add.write_back(&mut regs), SimResult::Wait(0, ()), "write_back() == expected");

        assert_eq!(regs[DEST], RESULT);
    }

    #[test]
    fn test_add_imm() {
        let scenario = Scenario::new();

        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        
        let mut regs = Registers::new();

        let mut add = ArithUnsign::new(AddrMode::Immediate, ArithMode::Add);

        const REG: usize = 10;
        const DEST: usize = 2;
        const VAL1: u32 = 1;
        const VAL2: u32 = 2;
        const RESULT: u32 = VAL1 + VAL2;
        let mut instruction: u32 = 0;
        instruction.set_bits(18..=22, (REG as u32).get_bits(0..=4));
        instruction.set_bits(23..=31, VAL2.get_bits(0..=8));
        instruction.set_bits(13..=17, (DEST as u32).get_bits(0..=4));

        regs[REG] = VAL1;

        assert_eq!(add.decode(instruction, &regs), SimResult::Wait(0, ()), "decode() == expected");
        assert_eq!(add.op1, VAL1, "OP1 == instr.op1");
        assert_eq!(add.op2, VAL2, "OP2 == instr.op2");
        assert_eq!(add.dest, DEST, "DEST = instr.dest");
        assert_eq!(add.operation, ArithMode::Add, "operation == instr.operation");

        assert_eq!(add.execute(), SimResult::Wait(0, ()), "execute() == expected");
        assert_eq!(add.result, RESULT, "execute == worked");
        assert_eq!(add.access_memory(mem_ref), SimResult::Wait(0, ()), "access_memory() == expected");
        assert_eq!(add.write_back(&mut regs), SimResult::Wait(0, ()), "write_back() == expected");

        assert_eq!(regs[DEST], RESULT);
    }

    #[test]
    fn test_comp() {
        let scenario = Scenario::new();

        let (mut memory, memory_handle) = scenario.create_mock_for::<dyn Memory<u32, u32>>();
        let mem_ref = Rc::new(RefCell::new(memory));
        
        let mut regs = Registers::new();

        let mut comp = Comp::new();

        const REG1: usize = 10;
        const REG2: usize = 17;
        const VAL1: u32 = 12;
        const VAL2: u32 = 22;
        let RESULT: u32 = ConditionCodes::LT.value();
        let mut instruction: u32 = 0;
        instruction.set_bits(13..=27, (REG1 as u32).get_bits(0..=4));
        instruction.set_bits(18..=22, (REG2 as u32).get_bits(0..=4));

        regs[REG1] = VAL1;
        regs[REG2] = VAL2;

        assert_eq!(comp.decode(instruction, &regs), SimResult::Wait(0, ()), "decode() == expected");
        assert_eq!(comp.op1, VAL1, "OP1 == instr.op1");
        assert_eq!(comp.op2, VAL2, "OP2 == instr.op2");

        assert_eq!(comp.execute(), SimResult::Wait(0, ()), "execute() == expected");
        assert_eq!(comp.access_memory(mem_ref), SimResult::Wait(0, ()), "access_memory() == expected");
        assert_eq!(comp.write_back(&mut regs), SimResult::Wait(0, ()), "write_back() == expected");

        assert_eq!(regs[STS], RESULT);
    }
}
