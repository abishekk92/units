use crate::vm_executor::{ExecutionContext, ObjectEffect, VMExecutionError, VMExecutor};
use rvsim::{CpuState, Elf, Machine, Memory};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
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

    /// Create and configure a RISC-V machine
    fn create_machine(&self) -> Result<Machine, VMExecutionError> {
        // Create memory with configured size
        let memory = Memory::new(self.config.memory_limit);
        
        // Create CPU state
        let cpu_state = CpuState::new();
        
        // Create machine
        let machine = Machine::new(memory, cpu_state);
        
        Ok(machine)
    }

    /// Load ELF binary into machine memory
    fn load_elf(&self, machine: &mut Machine, elf_bytes: &[u8]) -> Result<u32, VMExecutionError> {
        // Parse ELF
        let mut cursor = Cursor::new(elf_bytes);
        let elf = Elf::from_reader(&mut cursor)
            .map_err(|e| VMExecutionError::InvalidBytecode(format!("ELF parsing failed: {}", e)))?;

        // Load ELF sections into memory
        let entry_point = elf.load_into_memory(machine.memory_mut())
            .map_err(|e| VMExecutionError::InvalidBytecode(format!("ELF loading failed: {}", e)))?;

        Ok(entry_point)
    }

    /// Serialize context and write to input buffer
    fn setup_input_buffer(
        &self, 
        machine: &mut Machine, 
        context: &ExecutionContext
    ) -> Result<(), VMExecutionError> {
        // Serialize the execution context
        let context_bytes = bincode::serialize(context)
            .map_err(|e| VMExecutionError::SerializationError(format!("Context serialization failed: {}", e)))?;

        // Check size limit
        if context_bytes.len() > MAX_BUFFER_SIZE as usize {
            return Err(VMExecutionError::SerializationError(
                "Context too large for input buffer".to_string()
            ));
        }

        // Write context size first (4 bytes little-endian)
        let size_bytes = (context_bytes.len() as u32).to_le_bytes();
        machine.memory_mut().write(INPUT_BUFFER_ADDR, &size_bytes)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to write context size: {}", e)))?;

        // Write context data
        machine.memory_mut().write(INPUT_BUFFER_ADDR + 4, &context_bytes)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to write context: {}", e)))?;

        Ok(())
    }

    /// Read and deserialize effects from output buffer
    fn read_output_buffer(&self, machine: &Machine) -> Result<Vec<ObjectEffect>, VMExecutionError> {
        // Read output size first (4 bytes little-endian)
        let mut size_bytes = [0u8; 4];
        machine.memory().read(OUTPUT_BUFFER_ADDR, &mut size_bytes)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to read output size: {}", e)))?;
        
        let output_size = u32::from_le_bytes(size_bytes) as usize;
        
        // Check size limit
        if output_size > MAX_BUFFER_SIZE as usize {
            return Err(VMExecutionError::SerializationError(
                "Output too large".to_string()
            ));
        }

        if output_size == 0 {
            return Ok(Vec::new());
        }

        // Read output data
        let mut output_bytes = vec![0u8; output_size];
        machine.memory().read(OUTPUT_BUFFER_ADDR + 4, &mut output_bytes)
            .map_err(|e| VMExecutionError::ExecutionFailed(format!("Failed to read output: {}", e)))?;

        // Deserialize effects
        let effects: Vec<ObjectEffect> = bincode::deserialize(&output_bytes)
            .map_err(|e| VMExecutionError::SerializationError(format!("Effects deserialization failed: {}", e)))?;

        Ok(effects)
    }

    /// Execute the RISC-V program
    fn execute_program(
        &self, 
        machine: &mut Machine, 
        entry_point: u32
    ) -> Result<i32, VMExecutionError> {
        // Set program counter to entry point
        machine.cpu_state_mut().set_pc(entry_point);

        // Execute with instruction limit
        let mut instruction_count = 0;
        let start_time = std::time::Instant::now();

        loop {
            // Check instruction limit
            if instruction_count >= self.config.instruction_limit {
                return Err(VMExecutionError::InstructionLimitExceeded);
            }

            // Check timeout
            if start_time.elapsed().as_millis() > self.config.timeout_ms as u128 {
                return Err(VMExecutionError::TimeoutExceeded);
            }

            // Execute one instruction
            match machine.step() {
                Ok(true) => {
                    instruction_count += 1;
                    continue;
                }
                Ok(false) => {
                    // Program terminated normally
                    // Get exit code from register a0 (x10)
                    let exit_code = machine.cpu_state().register(10) as i32;
                    return Ok(exit_code);
                }
                Err(e) => {
                    return Err(VMExecutionError::ExecutionFailed(format!("Execution error: {}", e)));
                }
            }
        }
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
        // 1. Create and configure RISC-V machine
        let mut machine = self.create_machine()?;

        // 2. Load ELF binary into VM memory space
        let entry_point = self.load_elf(&mut machine, bytecode)?;

        // 3. Set up input buffer with serialized ExecutionContext
        self.setup_input_buffer(&mut machine, context)?;

        // 4. Execute the program
        let exit_code = self.execute_program(&mut machine, entry_point)?;

        // 5. Check exit code
        if exit_code != 0 {
            return Err(VMExecutionError::ExecutionFailed(format!("Program exited with code: {}", exit_code)));
        }

        // 6. Read and deserialize ObjectEffects from output buffer
        let effects = self.read_output_buffer(&machine)?;

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
    use units_core::id::UnitsObjectId;
    use units_core::objects::UnitsObject;
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
    fn test_machine_creation() {
        let executor = RiscVExecutor::new();
        let machine = executor.create_machine();
        assert!(machine.is_ok());
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
        
        // Try to execute invalid bytecode
        let invalid_elf = vec![0x00, 0x01, 0x02, 0x03]; // Not a valid ELF
        let result = executor.load_and_execute(&invalid_elf, &context);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            VMExecutionError::InvalidBytecode(_) => {}, // Expected
            other => panic!("Expected InvalidBytecode error, got: {:?}", other),
        }
    }
}