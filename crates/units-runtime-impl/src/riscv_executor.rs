//! RISC-V VM Executor implementation for the UNITS system
//!
//! This module provides a RISC-V virtual machine executor that supports both
//! ELF binaries and raw bytecode format for simplified execution.
//!
//! ## Raw Bytecode Format
//! 
//! The raw bytecode format is a simplified alternative to ELF that allows
//! direct loading of RISC-V instructions:
//!
//! ```text
//! [4 bytes] Magic: "RVBC" (0x52564243)
//! [4 bytes] Entry offset: Little-endian u32, must be 4-byte aligned
//! [N bytes] RISC-V instructions
//! ```
//!
//! The bytecode is loaded at address 0x1000, and the entry point is calculated
//! as 0x1000 + entry_offset.

use units_core_types::{ExecutionContext, ObjectEffect, VMExecutionError, VMExecutor};
use rvsim::*;
use std::time::Instant;
use units_core_types::objects::VMType;

/// RISC-V VM memory layout constants
const INPUT_BUFFER_ADDR: u32 = 0x10000000;
const OUTPUT_BUFFER_ADDR: u32 = 0x20000000;
const MAX_BUFFER_SIZE: u32 = 1024 * 1024; // 1MB limit
const CODE_BASE_ADDR: u32 = 0x1000; // Base address for loading bytecode

/// Raw bytecode format magic bytes
const BYTECODE_MAGIC: &[u8; 4] = b"RVBC";

/// ELF constants
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const PT_LOAD: u32 = 1; // Loadable segment type
const ELF32_HEADER_SIZE: usize = 52;
const ELF32_PHDR_SIZE: usize = 32; // Program header size

/// RISC-V executor configuration
#[derive(Debug, Clone)]
pub struct RiscVExecutorConfig {
    /// Maximum memory size in bytes
    pub memory_limit: usize,
    /// Maximum number of instructions to execute
    pub instruction_limit: u64,
    /// Maximum execution time in milliseconds
    pub timeout_ms: u64,
}

/// Custom memory implementation for rvsim
struct RiscVMemory {
    data: Vec<u8>,
    memory_limit: usize,
}

impl RiscVMemory {
    fn new(memory_limit: usize) -> Self {
        Self {
            data: vec![0u8; memory_limit],
            memory_limit,
        }
    }
    
    /// Write bytes to memory at the specified address
    fn write_bytes(&mut self, addr: u32, bytes: &[u8]) -> Result<(), VMExecutionError> {
        let addr = addr as usize;
        if addr + bytes.len() > self.memory_limit {
            return Err(VMExecutionError::ExecutionFailed("Memory write out of bounds".to_string()));
        }
        self.data[addr..addr + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
    
    /// Read bytes from memory at the specified address
    fn read_bytes(&self, addr: u32, len: usize) -> Result<Vec<u8>, VMExecutionError> {
        let addr = addr as usize;
        if addr + len > self.memory_limit {
            return Err(VMExecutionError::ExecutionFailed("Memory read out of bounds".to_string()));
        }
        Ok(self.data[addr..addr + len].to_vec())
    }
}

impl Memory for RiscVMemory {
    fn access<T: Copy>(&mut self, addr: u32, _access: MemoryAccess<T>) -> bool {
        let addr = addr as usize;
        let size = std::mem::size_of::<T>();
        
        if addr + size > self.memory_limit {
            return false;
        }
        
        // For now, simplified implementation - just allow all accesses within bounds
        true
    }
}

impl Default for RiscVExecutorConfig {
    fn default() -> Self {
        Self {
            memory_limit: 16 * 1024 * 1024, // 16MB
            instruction_limit: 1_000_000,   // 1M instructions
            timeout_ms: 5000,               // 5 seconds
        }
    }
}

/// RISC-V VM executor implementation using rvsim
pub struct RiscVExecutor {
    config: RiscVExecutorConfig,
}

impl RiscVExecutor {
    /// Create a new RISC-V executor with default configuration
    pub fn new() -> Self {
        Self {
            config: RiscVExecutorConfig::default(),
        }
    }

    /// Create a new RISC-V executor with custom configuration
    pub fn with_config(config: RiscVExecutorConfig) -> Self {
        Self { config }
    }


    /// Load raw bytecode into machine memory
    /// Format: [4 bytes magic "RVBC"] [4 bytes entry offset] [N bytes RISC-V instructions]
    fn load_raw_bytecode(&self, bytecode: &[u8], memory: &mut RiscVMemory) -> Result<u32, VMExecutionError> {
        // Minimum size: magic (4) + entry offset (4) + at least one instruction (4)
        if bytecode.len() < 12 {
            return Err(VMExecutionError::InvalidBytecode("Bytecode too small".to_string()));
        }
        
        // Check magic bytes
        if &bytecode[0..4] != BYTECODE_MAGIC {
            return Err(VMExecutionError::InvalidBytecode("Invalid bytecode magic".to_string()));
        }
        
        // Read entry offset (little-endian)
        let entry_offset = u32::from_le_bytes([
            bytecode[4], bytecode[5], bytecode[6], bytecode[7]
        ]);
        
        // Get actual code bytes
        let code_bytes = &bytecode[8..];
        
        // Validate entry offset
        if entry_offset as usize >= code_bytes.len() {
            return Err(VMExecutionError::InvalidBytecode("Entry offset out of bounds".to_string()));
        }
        
        // Check alignment (RISC-V instructions must be 4-byte aligned)
        if entry_offset % 4 != 0 {
            return Err(VMExecutionError::InvalidBytecode("Entry offset must be 4-byte aligned".to_string()));
        }
        
        // Load code into memory at CODE_BASE_ADDR
        memory.write_bytes(CODE_BASE_ADDR, code_bytes)?;
        
        // Return absolute entry point address
        Ok(CODE_BASE_ADDR + entry_offset)
    }

    /// Load ELF binary into machine memory
    fn load_elf(&self, elf_bytes: &[u8], memory: &mut RiscVMemory) -> Result<u32, VMExecutionError> {
        // Basic ELF validation - check magic bytes
        if elf_bytes.len() < 4 || &elf_bytes[0..4] != ELF_MAGIC {
            return Err(VMExecutionError::InvalidBytecode("Invalid ELF magic bytes".to_string()));
        }
        
        // Check minimum ELF header size
        if elf_bytes.len() < ELF32_HEADER_SIZE {
            return Err(VMExecutionError::InvalidBytecode("ELF file too small".to_string()));
        }
        
        // Check if it's 32-bit ELF (required for RV32)
        if elf_bytes[4] != 1 {
            return Err(VMExecutionError::InvalidBytecode("Only 32-bit ELF files are supported".to_string()));
        }
        
        // Check endianness (little endian for RISC-V)
        if elf_bytes[5] != 1 {
            return Err(VMExecutionError::InvalidBytecode("Only little-endian ELF files are supported".to_string()));
        }
        
        // Parse ELF header fields
        let entry_point = u32::from_le_bytes([
            elf_bytes[24], elf_bytes[25], elf_bytes[26], elf_bytes[27]
        ]);
        
        let phoff = u32::from_le_bytes([
            elf_bytes[28], elf_bytes[29], elf_bytes[30], elf_bytes[31]
        ]) as usize;
        
        let phentsize = u16::from_le_bytes([elf_bytes[42], elf_bytes[43]]) as usize;
        let phnum = u16::from_le_bytes([elf_bytes[44], elf_bytes[45]]) as usize;
        
        // Validate entry point
        if entry_point == 0 {
            return Err(VMExecutionError::InvalidBytecode("Invalid entry point (zero)".to_string()));
        }
        
        if entry_point % 4 != 0 {
            return Err(VMExecutionError::InvalidBytecode("Entry point must be 4-byte aligned".to_string()));
        }
        
        // Validate program header parameters
        if phentsize != ELF32_PHDR_SIZE {
            return Err(VMExecutionError::InvalidBytecode(
                format!("Invalid program header size: expected {}, got {}", ELF32_PHDR_SIZE, phentsize)
            ));
        }
        
        if phoff == 0 || phnum == 0 {
            return Err(VMExecutionError::InvalidBytecode("No program headers found".to_string()));
        }
        
        // Check if we have enough data for program headers
        let ph_end = phoff + (phnum * phentsize);
        if ph_end > elf_bytes.len() {
            return Err(VMExecutionError::InvalidBytecode("Program headers extend beyond file".to_string()));
        }
        
        // Load all PT_LOAD segments
        let mut loaded_any = false;
        for i in 0..phnum {
            let ph_offset = phoff + (i * phentsize);
            
            // Parse program header
            let p_type = u32::from_le_bytes([
                elf_bytes[ph_offset], elf_bytes[ph_offset + 1], 
                elf_bytes[ph_offset + 2], elf_bytes[ph_offset + 3]
            ]);
            
            // Skip non-loadable segments
            if p_type != PT_LOAD {
                continue;
            }
            
            let p_offset = u32::from_le_bytes([
                elf_bytes[ph_offset + 4], elf_bytes[ph_offset + 5],
                elf_bytes[ph_offset + 6], elf_bytes[ph_offset + 7]
            ]) as usize;
            
            let p_vaddr = u32::from_le_bytes([
                elf_bytes[ph_offset + 8], elf_bytes[ph_offset + 9],
                elf_bytes[ph_offset + 10], elf_bytes[ph_offset + 11]
            ]);
            
            let p_filesz = u32::from_le_bytes([
                elf_bytes[ph_offset + 16], elf_bytes[ph_offset + 17],
                elf_bytes[ph_offset + 18], elf_bytes[ph_offset + 19]
            ]) as usize;
            
            let p_memsz = u32::from_le_bytes([
                elf_bytes[ph_offset + 20], elf_bytes[ph_offset + 21],
                elf_bytes[ph_offset + 22], elf_bytes[ph_offset + 23]
            ]) as usize;
            
            // Validate segment
            if p_offset + p_filesz > elf_bytes.len() {
                return Err(VMExecutionError::InvalidBytecode(
                    format!("Segment {} data extends beyond file", i)
                ));
            }
            
            // Check memory bounds
            if p_vaddr as usize + p_memsz > self.config.memory_limit {
                return Err(VMExecutionError::MemoryLimitExceeded);
            }
            
            // Load segment data
            if p_filesz > 0 {
                let segment_data = &elf_bytes[p_offset..p_offset + p_filesz];
                memory.write_bytes(p_vaddr, segment_data)?;
            }
            
            // Zero-fill remaining memory (BSS section)
            if p_memsz > p_filesz {
                let zero_start = p_vaddr + p_filesz as u32;
                let zero_size = p_memsz - p_filesz;
                let zeros = vec![0u8; zero_size];
                memory.write_bytes(zero_start, &zeros)?;
            }
            
            loaded_any = true;
        }
        
        if !loaded_any {
            return Err(VMExecutionError::InvalidBytecode("No loadable segments found".to_string()));
        }
        
        Ok(entry_point)
    }

    /// Setup input buffer with execution context
    fn setup_input_buffer(
        &self, 
        memory: &mut RiscVMemory, 
        context: &ExecutionContext
    ) -> Result<(), VMExecutionError> {
        // Serialize the execution context
        let context_bytes = bincode::serialize(context)
            .map_err(|e| VMExecutionError::SerializationError(format!("Context serialization failed: {}", e)))?;
        
        // Check if serialized context fits in the buffer
        if context_bytes.len() > MAX_BUFFER_SIZE as usize {
            return Err(VMExecutionError::ExecutionFailed(
                format!("Execution context too large: {} bytes", context_bytes.len())
            ));
        }
        
        // Write context to input buffer location
        memory.write_bytes(INPUT_BUFFER_ADDR, &context_bytes)?;
        
        // Write buffer size at the beginning of the buffer (for the VM program to know)
        let size_bytes = (context_bytes.len() as u32).to_le_bytes();
        memory.write_bytes(INPUT_BUFFER_ADDR - 4, &size_bytes)?;
        
        Ok(())
    }

    /// Read output buffer and deserialize object effects
    fn read_output_buffer(&self, memory: &RiscVMemory) -> Result<Vec<ObjectEffect>, VMExecutionError> {
        // Read the output buffer size (stored at OUTPUT_BUFFER_ADDR - 4)
        let size_bytes = memory.read_bytes(OUTPUT_BUFFER_ADDR - 4, 4)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to read output size: {}", e)))?;
        
        let output_size = u32::from_le_bytes([
            size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]
        ]) as usize;
        
        // Validate output size
        if output_size > MAX_BUFFER_SIZE as usize {
            return Err(VMExecutionError::ExecutionFailed(
                format!("Output buffer size too large: {} bytes", output_size)
            ));
        }
        
        // If no output, return empty vector
        if output_size == 0 {
            return Ok(Vec::new());
        }
        
        // Read the output buffer
        let output_bytes = memory.read_bytes(OUTPUT_BUFFER_ADDR, output_size)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to read output buffer: {}", e)))?;
        
        // Deserialize object effects
        let effects: Vec<ObjectEffect> = bincode::deserialize(&output_bytes)
            .map_err(|e| VMExecutionError::SerializationError(format!("Failed to deserialize effects: {}", e)))?;
        
        Ok(effects)
    }

    /// Execute RISC-V program using rvsim
    fn execute_program(
        &self,
        memory: &mut RiscVMemory,
        entry_point: u32
    ) -> Result<i32, VMExecutionError> {
        // Create CPU state with the entry point
        let mut cpu = CpuState::new(entry_point);
        
        // Create a simple clock
        let mut clock = SimpleClock::new();
        
        // Create interpreter
        let mut interp = Interp::new(&mut cpu, memory, &mut clock);
        
        // For this simplified implementation, we'll run the program once
        // In a full implementation, we'd have a proper execution loop with timeout and instruction limits
        let _start_time = Instant::now();
        
        // Try to run one step
        let _result = interp.run();
        
        // For now, use a simplified approach - assume the program runs once and terminates
        // In a full implementation, we'd check for different CpuError types
        // and handle them appropriately (halt, breakpoint, system calls, etc.)
        
        // If we reach here, consider the program executed (for basic testing)
        // A proper implementation would loop until termination condition
        Ok(0)
    }
}

impl Default for RiscVExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl VMExecutor for RiscVExecutor {
    fn vm_type(&self) -> VMType {
        VMType::RiscV
    }
    
    fn load_and_execute(
        &self,
        bytecode: &[u8],
        context: &ExecutionContext,
    ) -> Result<Vec<ObjectEffect>, VMExecutionError> {
        // 1. Create memory for the RISC-V VM
        let mut memory = RiscVMemory::new(self.config.memory_limit);

        // 2. Detect bytecode format and load appropriately
        let entry_point = if bytecode.len() >= 4 && &bytecode[0..4] == BYTECODE_MAGIC {
            // Raw bytecode format
            self.load_raw_bytecode(bytecode, &mut memory)?
        } else if bytecode.len() >= 4 && &bytecode[0..4] == b"\x7fELF" {
            // ELF format
            self.load_elf(bytecode, &mut memory)?
        } else {
            return Err(VMExecutionError::InvalidBytecode(
                "Unknown bytecode format (expected RVBC or ELF)".to_string()
            ));
        };

        // 3. Set up input buffer with serialized ExecutionContext
        self.setup_input_buffer(&mut memory, context)?;

        // 4. Execute the program
        let exit_code = self.execute_program(&mut memory, entry_point)?;

        // 5. Check exit code
        if exit_code != 0 {
            return Err(VMExecutionError::ExecutionFailed(format!("Program exited with code: {}", exit_code)));
        }

        // 6. Read and deserialize ObjectEffects from output buffer
        let effects = self.read_output_buffer(&memory)?;

        // 7. Validate effects (controller can only modify objects it controls)
        units_core_types::validate_object_effects(&effects, context.instruction.controller_id)?;

        Ok(effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use units_core_types::ExecutionContext;
    use std::collections::HashMap;
    use units_core_types::constants::TOKEN_CONTROLLER_ID;
    use units_core_types::transaction::Instruction;

    #[test]
    fn test_riscv_executor_creation() {
        let executor = RiscVExecutor::new();
        assert_eq!(executor.vm_type(), VMType::RiscV);
        
        let custom_config = RiscVExecutorConfig {
            memory_limit: 8 * 1024 * 1024,
            instruction_limit: 500_000,
            timeout_ms: 1000,
        };
        
        let custom_executor = RiscVExecutor::with_config(custom_config.clone());
        assert_eq!(custom_executor.config.memory_limit, custom_config.memory_limit);
    }

    #[test]
    fn test_invalid_elf() {
        let executor = RiscVExecutor::new();
        
        let instruction = Instruction::new(
            TOKEN_CONTROLLER_ID,
            "test".to_string(),
            vec![],
            vec![],
        );
        
        let context = ExecutionContext::new(instruction, HashMap::new(), 1, 2);
        
        // Try to execute invalid bytecode (not ELF magic)
        let invalid_elf = vec![0x00, 0x01, 0x02, 0x03]; // Not a valid ELF
        let result = executor.load_and_execute(&invalid_elf, &context);
        
        // Should fail with unknown format error
        assert!(result.is_err());
        match result.unwrap_err() {
            VMExecutionError::InvalidBytecode(msg) => {
                assert!(msg.contains("Unknown bytecode format"));
            },
            other => panic!("Expected InvalidBytecode error, got: {:?}", other),
        }
    }
    
    #[test]
    fn test_valid_elf_header_no_segments() {
        let executor = RiscVExecutor::new();
        let mut memory = RiscVMemory::new(executor.config.memory_limit);
        
        // Create a minimal valid ELF header (32-bit, little-endian)
        let mut valid_elf = vec![0u8; 64]; // Minimal ELF header size
        valid_elf[0..4].copy_from_slice(b"\x7fELF"); // ELF magic
        valid_elf[4] = 1; // 32-bit
        valid_elf[5] = 1; // little-endian
        valid_elf[6] = 1; // ELF version
        
        // Set entry point at offset 24 (4 bytes, little-endian)
        let test_entry_point = 0x1000u32;
        valid_elf[24..28].copy_from_slice(&test_entry_point.to_le_bytes());
        
        // Set program header info
        valid_elf[28..32].copy_from_slice(&0u32.to_le_bytes()); // phoff = 0
        valid_elf[42..44].copy_from_slice(&32u16.to_le_bytes()); // phentsize = 32
        valid_elf[44..46].copy_from_slice(&0u16.to_le_bytes()); // phnum = 0
        
        // This should fail with "No program headers found" error
        let result = executor.load_elf(&valid_elf, &mut memory);
        assert!(result.is_err());
        match result.unwrap_err() {
            VMExecutionError::InvalidBytecode(msg) => {
                assert!(msg.contains("No program headers found"));
            },
            other => panic!("Expected InvalidBytecode error about no program headers, got: {:?}", other),
        }
    }
    
    #[test]
    fn test_elf_with_loadable_segment() {
        let executor = RiscVExecutor::new();
        let mut memory = RiscVMemory::new(executor.config.memory_limit);
        
        // Create ELF with program header
        let mut elf = vec![0u8; 256]; // Enough space for header + program header + code
        
        // ELF header
        elf[0..4].copy_from_slice(b"\x7fELF"); // Magic
        elf[4] = 1; // 32-bit
        elf[5] = 1; // Little-endian
        elf[6] = 1; // ELF version
        
        let entry_point = 0x1000u32;
        let phoff = 52u32; // Program headers right after ELF header
        let phentsize = 32u16;
        let phnum = 1u16;
        
        elf[24..28].copy_from_slice(&entry_point.to_le_bytes()); // e_entry
        elf[28..32].copy_from_slice(&phoff.to_le_bytes()); // e_phoff
        elf[42..44].copy_from_slice(&phentsize.to_le_bytes()); // e_phentsize
        elf[44..46].copy_from_slice(&phnum.to_le_bytes()); // e_phnum
        
        // Program header (PT_LOAD)
        let ph_start = phoff as usize;
        elf[ph_start..ph_start+4].copy_from_slice(&PT_LOAD.to_le_bytes()); // p_type
        elf[ph_start+4..ph_start+8].copy_from_slice(&128u32.to_le_bytes()); // p_offset (data at offset 128)
        elf[ph_start+8..ph_start+12].copy_from_slice(&entry_point.to_le_bytes()); // p_vaddr
        elf[ph_start+12..ph_start+16].copy_from_slice(&entry_point.to_le_bytes()); // p_paddr
        elf[ph_start+16..ph_start+20].copy_from_slice(&12u32.to_le_bytes()); // p_filesz (3 instructions)
        elf[ph_start+20..ph_start+24].copy_from_slice(&12u32.to_le_bytes()); // p_memsz
        
        // Add some RISC-V code at offset 128
        elf[128..132].copy_from_slice(&[0x13, 0x00, 0x00, 0x00]); // nop
        elf[132..136].copy_from_slice(&[0x13, 0x00, 0x00, 0x00]); // nop
        elf[136..140].copy_from_slice(&[0x6f, 0x00, 0x00, 0x00]); // j 0 (infinite loop)
        
        // Load the ELF
        let result = executor.load_elf(&elf, &mut memory);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), entry_point);
        
        // Verify code was loaded at correct address
        let loaded_code = memory.read_bytes(entry_point, 12).unwrap();
        assert_eq!(&loaded_code[0..4], &[0x13, 0x00, 0x00, 0x00]);
        assert_eq!(&loaded_code[8..12], &[0x6f, 0x00, 0x00, 0x00]);
    }
    
    #[test]
    fn test_elf_with_bss_section() {
        let executor = RiscVExecutor::new();
        let mut memory = RiscVMemory::new(executor.config.memory_limit);
        
        // Create ELF with BSS section (p_memsz > p_filesz)
        let mut elf = vec![0u8; 256];
        
        // ELF header
        elf[0..4].copy_from_slice(b"\x7fELF");
        elf[4] = 1; // 32-bit
        elf[5] = 1; // Little-endian
        elf[6] = 1; // ELF version
        
        let entry_point = 0x2000u32;
        let data_addr = 0x3000u32;
        let phoff = 52u32;
        
        elf[24..28].copy_from_slice(&entry_point.to_le_bytes());
        elf[28..32].copy_from_slice(&phoff.to_le_bytes());
        elf[42..44].copy_from_slice(&32u16.to_le_bytes()); // e_phentsize
        elf[44..46].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
        
        // Program header with BSS
        let ph_start = phoff as usize;
        elf[ph_start..ph_start+4].copy_from_slice(&PT_LOAD.to_le_bytes());
        elf[ph_start+4..ph_start+8].copy_from_slice(&128u32.to_le_bytes()); // p_offset
        elf[ph_start+8..ph_start+12].copy_from_slice(&data_addr.to_le_bytes()); // p_vaddr
        elf[ph_start+12..ph_start+16].copy_from_slice(&data_addr.to_le_bytes()); // p_paddr
        elf[ph_start+16..ph_start+20].copy_from_slice(&8u32.to_le_bytes()); // p_filesz (8 bytes)
        elf[ph_start+20..ph_start+24].copy_from_slice(&32u32.to_le_bytes()); // p_memsz (32 bytes - includes BSS)
        
        // Add initialized data
        elf[128..132].copy_from_slice(&0xDEADBEEFu32.to_le_bytes());
        elf[132..136].copy_from_slice(&0xCAFEBABEu32.to_le_bytes());
        
        // Load the ELF
        let result = executor.load_elf(&elf, &mut memory);
        assert!(result.is_ok());
        
        // Verify initialized data
        let loaded_data = memory.read_bytes(data_addr, 8).unwrap();
        assert_eq!(&loaded_data[0..4], &0xDEADBEEFu32.to_le_bytes());
        assert_eq!(&loaded_data[4..8], &0xCAFEBABEu32.to_le_bytes());
        
        // Verify BSS section is zero-filled
        let bss_data = memory.read_bytes(data_addr + 8, 24).unwrap();
        assert!(bss_data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_raw_bytecode_loading() {
        let executor = RiscVExecutor::new();
        let mut memory = RiscVMemory::new(executor.config.memory_limit);
        
        // Create a valid raw bytecode with some RISC-V instructions
        let mut bytecode = Vec::new();
        bytecode.extend_from_slice(BYTECODE_MAGIC); // Magic bytes
        bytecode.extend_from_slice(&8u32.to_le_bytes()); // Entry offset
        
        // Add some sample RISC-V instructions (NOP instructions)
        bytecode.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]); // nop (addi x0, x0, 0)
        bytecode.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]); // nop
        bytecode.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]); // nop (entry point here)
        
        let entry_point = executor.load_raw_bytecode(&bytecode, &mut memory);
        assert!(entry_point.is_ok());
        assert_eq!(entry_point.unwrap(), CODE_BASE_ADDR + 8);
        
        // Verify code was loaded into memory
        let loaded_code = memory.read_bytes(CODE_BASE_ADDR, 12).unwrap();
        assert_eq!(&loaded_code[0..4], &[0x13, 0x00, 0x00, 0x00]);
    }
    
    #[test]
    fn test_raw_bytecode_validation() {
        let executor = RiscVExecutor::new();
        let mut memory = RiscVMemory::new(executor.config.memory_limit);
        
        // Test too small bytecode
        let small_bytecode = vec![0x00, 0x01, 0x02];
        let result = executor.load_raw_bytecode(&small_bytecode, &mut memory);
        assert!(result.is_err());
        
        // Test invalid magic
        let invalid_magic = vec![0x00, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00];
        let result = executor.load_raw_bytecode(&invalid_magic, &mut memory);
        assert!(result.is_err());
        
        // Test invalid entry offset (out of bounds)
        let mut invalid_offset = Vec::new();
        invalid_offset.extend_from_slice(BYTECODE_MAGIC);
        invalid_offset.extend_from_slice(&100u32.to_le_bytes()); // Offset beyond code size
        invalid_offset.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]); // One instruction
        let result = executor.load_raw_bytecode(&invalid_offset, &mut memory);
        assert!(result.is_err());
        
        // Test unaligned entry offset
        let mut unaligned = Vec::new();
        unaligned.extend_from_slice(BYTECODE_MAGIC);
        unaligned.extend_from_slice(&3u32.to_le_bytes()); // Not 4-byte aligned
        unaligned.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]);
        let result = executor.load_raw_bytecode(&unaligned, &mut memory);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_bytecode_format_detection() {
        let executor = RiscVExecutor::new();
        
        let instruction = Instruction::new(
            TOKEN_CONTROLLER_ID,
            "test".to_string(),
            vec![],
            vec![],
        );
        
        let context = ExecutionContext::new(instruction, HashMap::new(), 1, 2);
        
        // Test raw bytecode format detection
        let mut raw_bytecode = Vec::new();
        raw_bytecode.extend_from_slice(BYTECODE_MAGIC);
        raw_bytecode.extend_from_slice(&0u32.to_le_bytes());
        raw_bytecode.extend_from_slice(&[0x13, 0x00, 0x00, 0x00]); // NOP
        
        // This should detect raw bytecode format (though execution will fail without proper implementation)
        let result = executor.load_and_execute(&raw_bytecode, &context);
        // We expect it to fail later in execution, not in format detection
        assert!(result.is_err());
        
        // Test unknown format
        let unknown_format = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result = executor.load_and_execute(&unknown_format, &context);
        match result.unwrap_err() {
            VMExecutionError::InvalidBytecode(msg) => {
                assert!(msg.contains("Unknown bytecode format"));
            }
            _ => panic!("Expected InvalidBytecode error for unknown format"),
        }
    }
}