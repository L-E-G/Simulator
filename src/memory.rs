#[cfg(test)] use mockers_derive::mocked;

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::{Index,IndexMut};

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
    file: [u32; REGISTERS_SIZE],
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

impl Registers {
    pub fn new() -> Registers {
        Registers{
            file: [0; REGISTERS_SIZE],
        }
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
/// interface purposes. A is the address type.
pub trait InspectableMemory<A> {
    /// Returns a text description of the entire data structure.
    fn inspect_txt(&self) -> Result<String, String>;
    
    /// Returns a text description of an address.
    fn inspect_address_txt(&self, address: A) -> Result<String, String>;
}

/// Simulates the slow DRAM memory.
pub struct DRAM {
    delay: u16,
    data: HashMap<u32, u32>,
}

impl DRAM {
    pub fn new(delay: u16) -> DRAM {
        DRAM{
            delay: delay,
            data: HashMap::new(),
        }
    }
}

impl InspectableMemory<u32> for DRAM {
    fn inspect_txt(&self) -> Result<String, String> {
        let mut out = String::new();

        for (k, v) in &self.data {
            out.push_str(format!("{}: {}\n", k, v).as_str());
        }

        Ok(out)
    }
    
    fn inspect_address_txt(&self, address: u32) -> Result<String, String> {
        match self.data.get(&address) {
            Some(d) => Ok(format!("\
Address: {}
Value  : {}", address, *d)),
            None => Ok(format!("Does not exist")),
        }
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


const DM_CACHE_LINES: usize = 1024;

// Direct mapped cache.
// 10 least significant bits of an address are the index.
// 22 most significant bits of an address are the tag.
pub struct DMCache {
    delay: u16,
    lines: [DMCacheLine; DM_CACHE_LINES],
    base: Rc<RefCell<dyn Memory<u32, u32>>>,
}

#[derive(Copy,Clone)]
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
    pub fn new(delay: u16, base: Rc<RefCell<dyn Memory<u32, u32>>>) -> DMCache {
        let lines = [DMCacheLine::new(); DM_CACHE_LINES];

        DMCache{
            delay: delay,
            lines: lines,
            base: base,
        }
    }

    fn get_address_index(&self, address: u32) -> usize {
        ((address << 22) >> 22) as usize
    }

    fn get_address_tag(&self, address: u32) -> u32 {
        address >> 10
    }
}

impl InspectableMemory<u32> for DMCache {
    fn inspect_txt(&self) -> Result<String, String> {
        let mut out = String::new();

        for line in self.lines.iter() {
            if !line.valid {
                continue;
            }
            
            out.push_str(format!("{} = {} [valid={}, dirty={}]\n",
                line.tag, line.data, line.valid, line.dirty).as_str());
        }

        Ok(out)
    }
        
    fn inspect_address_txt(&self, address: u32) -> Result<String, String> {
        let idx = self.get_address_index(address);

        let line = self.lines[idx];

        Ok(format!("\
Index: {}
Tag  : {}
Data : {}
Valid: {}
Dirty: {}", idx,
                   line.tag, line.data, line.valid, line.dirty))
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
                let old_addr = (u32::from(line.tag) << 10) | (idx as u32);
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
}
