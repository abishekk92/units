# UNITS Execution Environment Alignment Tasks

## Architecture Reference

**📋 Complete Architecture Specification**: See [ARCHITECTURE_SPEC.md](./ARCHITECTURE_SPEC.md)

**Key Design**: Unified object model where everything is a UnitsObject with immutable controllers. Kernel modules are RISC-V ELF shared objects that execute in sandboxed VMs to control object mutations.

## Implementation Status

✅ **Design Complete**: Unified object architecture with VM extensibility  
⚠️ **Implementation Pending**: Core object model and RISC-V execution

## 🎯 Implementation Roadmap

### Phase 1.1: Core Object Model
- [ ] **NEW UnitsObject struct** - Replace current enum-based system
  - Add controller_id and object_type fields (ObjectType enum with VMType)
  - Implement VMType non-exhaustive enum and ObjectType enum
  - Update object creation/access APIs  
  - **Files**: `units-core/src/objects.rs`

- [ ] **System constants** - Bootstrap foundation
  - Define hardcoded controller IDs (SYSTEM_LOADER_ID, etc.)
  - Add validation for system controllers
  - **Files**: `units-core/src/constants.rs` (new)

### Phase 1.2: RISC-V Execution
- [ ] **VM executor trait** - Pluggable VM architecture
  - Define VMExecutor trait for multiple VM types
  - Implement ExecutionContext and ObjectEffect structs
  - **Files**: `units-runtime/src/vm_executor.rs` (new)

- [ ] **RISC-V integration** - ELF execution sandbox
  - Research RISC-V crate (riscv-vm, riscv-emu, etc.)
  - Implement RiscVExecutor with ELF loading
  - Add memory/instruction limits and sandboxing
  - **Files**: `units-runtime/src/riscv_executor.rs` (new)

### Phase 1.3: Transaction Pipeline
- [ ] **Instruction validation** - Single-controller enforcement
  - Implement target object validation  
  - Reserve space for future cross-controller support
  - **Files**: `units-core/src/transaction.rs` (update)

- [ ] **Complete execution flow** - End-to-end pipeline
  - Integrate VM execution with transaction processing
  - Update TransactionReceipt to include ObjectEffects
  - Test with simple RISC-V controller
  - **Files**: `units-runtime/src/transaction_manager.rs` (update)


## 📋 Success Criteria

**MVP Goals:**
- [ ] All existing tests pass with new object model
- [ ] Simple RISC-V controller executes and modifies objects  
- [ ] TransactionReceipt includes ObjectEffects
- [ ] Clean migration path from current system

## 🔮 Future Phases (Post-MVP)

### Phase 2: Attestation & Security
- TEE attestation integration with TransactionReceipt
- Advanced resource limits and quota enforcement
- Enhanced sandboxing and isolation

### Phase 3: Extensions  
- **Cross-controller communication** (design space reserved)
- **Additional VM types** (WASM, eBPF via VMExecutor trait)
- **Distributed execution** and consensus integration

## 🚀 Next Steps

1. **Start Phase 1.1**: Implement new UnitsObject struct in `units-core/src/objects.rs`
2. **Research RISC-V crates**: Evaluate riscv-vm, riscv-emu, or similar for Phase 1.2
3. **Incremental testing**: Each phase should maintain backward compatibility

**Ready to begin implementation!**
