#[cfg(test)] use mockers_derive::mocked;

use web_sys::console;
use wasm_bindgen::JsValue;
use bit_field::BitField;

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::{Index,IndexMut};
use std::io::{Read,BufReader};
use std::fs::File;
use std::fmt;

use crate::result::SimResult;

/// The size of the register file.
const REGISTERS_SIZE: usize = 32;

/// Holds all computation registers.
/// Indexes:
/// - [0, 25]: General purpose
/// - 26: Interrupt link return address
/// - 27: Interrupt handler address
/// - 28: Program counter
/// - 29: Status
/// - 30: Stack pointer
/// - 31: Subroutine link return address
#[derive(Clone,Debug,PartialEq)]
pub struct Registers {
    /// Holds register values
    pub file: [u32; REGISTERS_SIZE],
}

/// Interupt link register index
pub const INTLR: usize = 26;

/// Interrupt handler register index
pub const IHDLR: usize = 27;

/// Program counter register index
pub const PC: usize = 28;

/// Status register index
pub const STS: usize = 29;

/// Stack pointer register index
pub const SP: usize = 30;

/// Link register index
pub const LR: usize = 31;


/// Start of the program memory
// pub struct Memory_Start {
//     PROG_MEM_START: u32,
// }

// impl Memory_Start {
//     pub fn new() -> Memory_Start {
//         Memory_Start {
//             PROG_MEM_START: 0,
//         }
//     }
    
// }

impl Registers {
    pub fn new() -> Registers {
        Registers{
            file: [0; REGISTERS_SIZE],
        }
    }
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        
        for i in 0..REGISTERS_SIZE {
            let key = match i {
                INTLR => "INTLR",
                IHDLR => "IHDLR",
                PC => "PC",
                STS => "STS",
                SP => "SP",
                LR => "LR",
                _ => "",
            };
            if key.len() == 0 {
                out.push_str(format!("{:5}", i).as_str());
            } else {
                out.push_str(format!("{:5}", key).as_str());
            }
            
            out.push_str(format!(": {}", self.file[i]).as_str());

            if i + 1 != REGISTERS_SIZE {
                out.push_str("\n");
            }
        }

        write!(f, "{}", out)
    }
}

impl Index<usize> for Registers {
    type Output = u32;
    
    fn index(&self, idx: usize) -> &u32 {
        &self.file[idx]
    }
}

impl IndexMut<usize> for Registers {
    fn index_mut(&mut self, idx: usize) -> &mut u32 {
        &mut self.file[idx]
    }
}

/// Memory provides an interface to access a memory struct, A is the address type,
/// D is the data type.
#[cfg_attr(test, mocked)]
pub trait Memory<A, D> {
    /// Retrieve data at a memory address.
    fn get(&mut self, address: A) -> SimResult<D, String>;

    /// Place data at a memory address.
    fn set(&mut self, address: A, data: D) -> SimResult<(), String>;
}

/// InspectableMemory allows a memory unit to be insepcted for user
/// interface purposes. A is the address type. D is the data type.
pub trait InspectableMemory<A, D> {
    /// Returns a map of all a memory's contents. Where keys are addresses and
    /// values are memory values.
    fn inspect(&self) -> HashMap<A, D>;
    
    /// Returns a text description of an address.
    fn inspect_address_txt(&self, address: A) -> String;
}

/// Simulates the slow DRAM memory.
pub struct DRAM {
    delay: u16,
    data: HashMap<u32, u32>,
}

impl DRAM {
    /// Creates a new DRAM structure.
    pub fn new(delay: u16) -> DRAM {
        DRAM{
            delay: delay,
            data: HashMap::new(),
        }
    }

    /// Loads contents of a file into DRAM.
    /// See load_from_reader() for details about the required format of
    /// this file.
    pub fn load_from_file(&mut self, file_p: &str) -> Result<(), String> {
        // Read file
        let file = match File::open(file_p) {
            Ok(f) => f,
            Err(e) => {
                return Err(format!("Failed to open DRAM file \"{}\": {}",
                                   file_p, e));
            },
        };

        // Load
        self.load_from_reader(file)
    }

    /// Loads contents of a reader into DRAM.
    /// The buffer should be binary. Every 32 bits will be loaded in as a word
    /// in memory. The address in memory will increment by 1 for word loaded.
    pub fn load_from_reader(&mut self, src: impl Read) -> Result<(), String> {
        let mut reader = BufReader::new(src);
        let mut addr: u32 = 0;
        let mut buf: [u8; 4] = [0; 4];

        loop {
            match reader.read(&mut buf) {
                Ok(bytes_read) => {
                    if bytes_read == 0 { // End of file
                        return Ok(());
                    } else if bytes_read != 4 { // Incorrect number of bytes read
                        let mut read_as: Vec<String> = Vec::new();
                        for i in 0..bytes_read {
                            read_as.push(buf[i].to_string());
                        }
                        return Err(format!("Read {} bytes as {:?} from buffer but \
                                            expected 4 bytes, after reading {} \
                                            words successfuly",
                                           bytes_read, read_as, self.data.len()));
                    }

                    let value: u32 = (buf[3] as u32) |
                        (buf[2] as u32) << 8 |
                        (buf[1] as u32) << 16 |
                        (buf[0] as u32) << 24;
                    
                    self.data.insert(addr, value);
                    addr += 1;
                },
                Err(e) => {
                    return Err(format!("Failed to read buffer: {}", e));
                },
            }
        }
    }
}

impl InspectableMemory<u32, u32> for DRAM {
    fn inspect(&self) -> HashMap<u32, u32> {
        self.data.clone()
    }
    
    fn inspect_address_txt(&self, address: u32) -> String {
        match self.data.get(&address) {
            Some(d) => format!("\
Address: {}
Value  : {}", address, *d),
            None => format!("Does not exist"),
        }
    }
}

impl fmt::Display for DRAM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();

        let mut i = 0;
        for (k, v) in &self.data {
            out.push_str(format!("{}: {}", k, v).as_str());

            if i + 1 != self.data.len() {
                out.push_str("\n");
            }

            i += 1;
        }

        write!(f, "{}", out)
    }
}

impl Memory<u32, u32> for DRAM {
    fn get(&mut self, address: u32) -> SimResult<u32, String> {
        match self.data.get(&address) {
            Some(d) => SimResult::Wait(self.delay, *d),
            None => {
                self.data.insert(address, 0);
                SimResult::Wait(self.delay, 0)
            }
        }
    }
    
    fn set(&mut self, address: u32, data: u32) -> SimResult<(), String> {
        self.data.insert(address, data);
        SimResult::Wait(self.delay, ())
    }
}

/// Direct mapped cache.
pub struct DMCache {
    /// Number of cycles it takes to access this cache.
    delay: u16,

    /// Number of lines in the cache.
    num_lines: usize,

    /// Number of least significant bits used for an address's cache line index.
    idx_bits: usize,

    /// Number of most significant bits used for an address's cache line tag.
    tag_bits: usize,

    /// Cache lines.
    lines: Vec<DMCacheLine>,

    /// Underlying memory which will be used to populate the cache on the event
    /// of a cache miss.
    base: Rc<RefCell<dyn Memory<u32, u32>>>,
}

#[derive(Copy,Clone,Debug)]
struct DMCacheLine {
    tag: u32,
    data: u32,
    valid: bool,
    dirty: bool,
}

impl DMCacheLine {
    fn new() -> DMCacheLine {
        DMCacheLine{
            tag: 0,
            data: 0,
            valid: false,
            dirty: false,
        }
    }
}

impl DMCache {
    pub fn new(delay: u16,
               num_lines: usize,
               base: Rc<RefCell<dyn Memory<u32, u32>>>) -> DMCache {
        let mut lines: Vec<DMCacheLine> = vec![];
        for i in 0..num_lines {
            lines.push(DMCacheLine::new());
        }

        let idx_bits = (num_lines as f32).log(2.0).ceil();
        let tag_bits = 32.0 - idx_bits;

        DMCache{
            delay: delay,
            num_lines: num_lines,
            idx_bits: idx_bits as usize,
            tag_bits: tag_bits as usize,
            lines: lines,
            base: base,
        }
    }

    fn get_address_index(&self, address: u32) -> usize {
        address.get_bits(0..=self.idx_bits-1) as usize
        //((address << 22) >> 22) as usize
    }

    fn get_address_tag(&self, address: u32) -> u32 {
        address >> self.idx_bits
        //address >> 10
    }

    fn get_idx_address(&self, idx: usize, tag: u32) -> u32 {
        let mut addr: u32 = 0;
        addr.set_bits(0..=self.tag_bits-1, idx as u32);
        addr.set_bits(self.tag_bits..=31, tag);

        addr
    }

    pub fn inspect_valid(&self) -> HashMap<u32, u32> {
        let mut map: HashMap<u32, u32> = HashMap::new();

        for i in 0..self.num_lines {
            let line = self.lines[i];

            if !line.valid {
                continue
            }
            
            let addr: u32 = self.get_idx_address(i, line.tag);

            map.insert(addr, line.data);
        }

        map
    }

    /// Keys are addresses, values are descriptions of the line.
    pub fn inspect_valid_aliases(&self) -> HashMap<u32, String> {
        let mut map: HashMap<u32, String> = HashMap::new();

        for i in 0..self.num_lines {
            let line = self.lines[i];

            if !line.valid {
                continue
            }
            
            let addr: u32 = self.get_idx_address(i, line.tag);

            let dirty_str = match line.dirty {
                true => " d",
                false => "",
            };

            map.insert(addr, format!("#{} [{}]{}", i, line.tag, dirty_str));
        }

        map
    }
}

impl InspectableMemory<u32, u32> for DMCache {
    fn inspect(&self) -> HashMap<u32, u32> {
        let mut map: HashMap<u32, u32> = HashMap::new();

        for i in 0..self.num_lines {
            let line = self.lines[i];
            
            let addr: u32 = self.get_idx_address(i, line.tag);

            map.insert(addr, line.data);
        }

        map
    }
        
    fn inspect_address_txt(&self, address: u32) -> String {
        let idx = self.get_address_index(address);

        let line = self.lines[idx];

        format!("\
Index: {}
Tag  : {}
Data : {}
Valid: {}
Dirty: {}", idx,
                   line.tag, line.data, line.valid, line.dirty)
    }
}

impl fmt::Display for DMCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();

        let mut i = 0;
        for line in self.lines.iter() {
            out.push_str(format!("{} = {} [valid={}, dirty={}]",
                                 line.tag, line.data, line.valid,
                                 line.dirty).as_str());

            if i + 1 != self.lines.len() {
                out.push_str("\n");
            }

            i += 1;
        }

        write!(f, "{}", out)
    }
}

impl Memory<u32, u32> for DMCache {
    fn get(&mut self, address: u32) -> SimResult<u32, String> {
        // Get line
        let idx = self.get_address_index(address);
        let tag = self.get_address_tag(address);

        let line = self.lines[idx];

        // Check if address in cache
        if line.valid && line.tag == tag {
            SimResult::Wait(self.delay, line.data)
        } else {
            let mut total_wait: u16 = self.delay;
            
            // Evict current line if dirty and there is a conflict
            if line.valid && line.tag != tag && line.dirty {
                // Write to cache layer below
                let evict_res = self.base.borrow_mut().set(address, line.data);

                if let SimResult::Err(e) = evict_res {
                    return SimResult::Err(format!("failed to write out old line value when evicting: {}", e));
                }

                if let SimResult::Wait(c, _r) = evict_res {
                    total_wait += c;
                }
            }

            // Get value from cache layer below
            let get_res = self.base.borrow_mut().get(address);

            let data = match get_res {
                SimResult::Wait(w, d) => {
                    total_wait += w;
                    
                    d
                },
                SimResult::Err(e) => {
                    return SimResult::Err(format!("failed to get line value from base cache: {}", e));
                },
            };

            // Save in cache
            self.lines[idx].valid = true;
            self.lines[idx].dirty = false;
            self.lines[idx].tag = tag;
            self.lines[idx].data = data;

            SimResult::Wait(total_wait, data)
        }
    }
    
    fn set(&mut self, address: u32, data: u32) -> SimResult<(), String> {
        // Get line
        let idx = self.get_address_index(address);
        let tag = self.get_address_tag(address);

        let line = self.lines[idx];

        // If line matches address
        if line.valid && line.tag == tag {
            self.lines[idx].dirty = true;
            self.lines[idx].data = data;

            SimResult::Wait(self.delay, ())
        } else {
            let mut total_wait: u16 = self.delay;
            
            // Evict current line if dirty and there is a conflict
            if line.valid && line.tag != tag && line.dirty {
                // Write to cache layer below
                let old_addr = self.get_idx_address(idx, line.tag);//(u32::from(line.tag) << 10) | (idx as u32);
                let evict_res = self.base.borrow_mut().set(old_addr, line.data);

                if let SimResult::Err(e) = evict_res {
                    return SimResult::Err(format!("failed to write out old line value when evicting: {}", e));
                }

                if let SimResult::Wait(c, _r) = evict_res {
                    total_wait += c;
                }
            }

            // Save in cache
            self.lines[idx].valid = true;
            self.lines[idx].dirty = true;
            self.lines[idx].tag = tag;
            self.lines[idx].data = data;

            SimResult::Wait(total_wait, ())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Tests that the Registers type index trait implementations work.
    #[test]
    fn test_registers_indexable() {
        let mut regs = Registers::new();

        for i in 0..REGISTERS_SIZE {
            regs[i] = (REGISTERS_SIZE - i) as u32;
            assert_eq!(regs[i], (REGISTERS_SIZE - i) as u32,
                       "Registers[{}] failed to set", i);
        }
    }

    /// Tests the DRAM.load_from_file method.
    #[test]
    fn test_dram_load_from_file() {
        let mut dram = DRAM::new(0);

        assert_eq!(dram.load_from_file("./test-data/dram-test.bin"), Ok(()));

        let mut expected: HashMap<u32, u32> = HashMap::new();
        for i in 0..16 {
            expected.insert(i as u32, 15 - (i as u32));
        }

        assert_eq!(dram.inspect(), expected);
    }
}
