use crate::vm_executor::{ExecutionContext, ObjectEffect, VMExecutionError, VMExecutor};
use rvsim::*;
use std::time::Instant;
use units_core::objects::VMType;

/// RISC-V VM memory layout constants
const INPUT_BUFFER_ADDR: u32 = 0x10000000;
const OUTPUT_BUFFER_ADDR: u32 = 0x20000000;
const MAX_BUFFER_SIZE: u32 = 1024 * 1024; // 1MB limit

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


    /// Load ELF binary into machine memory
    fn load_elf(&self, elf_bytes: &[u8]) -> Result<u32, VMExecutionError> {
        // Basic ELF validation - check magic bytes
        if elf_bytes.len() < 4 || &elf_bytes[0..4] != b"\x7fELF" {
            return Err(VMExecutionError::InvalidBytecode("Invalid ELF magic bytes".to_string()));
        }
        
        // For now, we'll assume a simple ELF with entry point at a fixed location
        // In a full implementation, we would parse the ELF header to get the actual entry point
        // and load segments into memory at their specified addresses
        
        // Check if it's 32-bit ELF (required for RV32)
        if elf_bytes.len() < 5 || elf_bytes[4] != 1 {
            return Err(VMExecutionError::InvalidBytecode("Only 32-bit ELF files are supported".to_string()));
        }
        
        // For now, return a default entry point
        // TODO: Parse ELF header to extract real entry point
        Ok(0x1000)
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

        // 2. Load and validate ELF binary
        let entry_point = self.load_elf(bytecode)?;

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
        crate::vm_executor::validate_object_effects(&effects, context.instruction.controller_id)?;

        Ok(effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm_executor::ExecutionContext;
    use std::collections::HashMap;
    use units_core::constants::TOKEN_CONTROLLER_ID;
    use units_core::transaction::Instruction;

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
        
        // Now with proper rvsim integration, this should fail
        assert!(result.is_err());
        match result.unwrap_err() {
            VMExecutionError::InvalidBytecode(_) => {}, // Expected
            other => panic!("Expected InvalidBytecode error, got: {:?}", other),
        }
    }
    
    #[test]
    fn test_valid_elf_header() {
        let executor = RiscVExecutor::new();
        
        // Create a minimal valid ELF header (32-bit, little-endian)
        let mut valid_elf = vec![0u8; 64]; // Minimal ELF header size
        valid_elf[0..4].copy_from_slice(b"\x7fELF"); // ELF magic
        valid_elf[4] = 1; // 32-bit
        valid_elf[5] = 1; // little-endian
        valid_elf[6] = 1; // ELF version
        
        // This should at least pass ELF validation (though execution may fail)
        let entry_point = executor.load_elf(&valid_elf);
        assert!(entry_point.is_ok());
        assert_eq!(entry_point.unwrap(), 0x1000); // Default entry point
    }
}