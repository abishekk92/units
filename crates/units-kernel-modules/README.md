# UNITS Kernel Modules

This directory contains kernel modules for the UNITS system. Kernel modules are executable objects that control the lifecycle and behavior of data objects in the system.

## Architecture

Kernel modules in UNITS:
- Are compiled to RISC-V bytecode and run in a sandboxed VM environment
- Have exclusive control over objects they manage (controller pattern)
- Process instructions and return object effects (state changes)
- Cannot directly access system resources - all I/O goes through stdin/stdout

## Token Module

The token module (`token/`) manages the complete lifecycle of tokens in the UNITS system.

### Functions

- **tokenize**: Create a new token with initial supply
- **transfer**: Transfer tokens between balance objects
- **mint**: Increase token supply
- **burn**: Decrease token supply
- **freeze**: Prevent all transfers of a token
- **unfreeze**: Re-enable transfers of a token

### Data Structures

**TokenData**: Core token information
- `total_supply`: Total amount of tokens in existence
- `decimals`: Number of decimal places
- `name`: Human-readable token name
- `symbol`: Token ticker symbol
- `is_frozen`: Whether transfers are allowed

**BalanceData**: Token balance for an owner
- `token_id`: ID of the token
- `owner_id`: ID of the balance owner
- `amount`: Token balance

### Building

To build the RISC-V kernel module:
```bash
cd token
CARGO_FEATURE_BUILD_RISCV=1 RISCV_PREFIX=riscv64-unknown-elf cargo build
```

Note: You need a RISC-V toolchain installed. The `RISCV_PREFIX` environment variable should point to your toolchain prefix.

### Testing

Run the test suite:
```bash
cd token
cargo test --features test-harness
```

## Integration with UNITS

Kernel modules integrate with the UNITS system through:

1. **Registration**: The module is stored as an executable object with a well-known ID (e.g., `TOKEN_CONTROLLER_ID`)
2. **Execution**: When a transaction targets the module, the VM loads and executes it
3. **State Changes**: The module returns `ObjectEffect`s that describe how objects should be updated
4. **Security**: Only the module can modify objects it controls, ensuring consistency

## Development Guidelines

When creating new kernel modules:

1. **Use C for RISC-V**: Write the core logic in C for compilation to RISC-V
2. **Rust for Types**: Define data structures and serialization in Rust
3. **Stdin/Stdout Protocol**: All communication uses a binary protocol through standard I/O
4. **Deterministic Execution**: Modules must be deterministic - same inputs always produce same outputs
5. **No External Dependencies**: Modules cannot access files, network, or other system resources
6. **Error Handling**: Return appropriate error codes and write empty effects on errors