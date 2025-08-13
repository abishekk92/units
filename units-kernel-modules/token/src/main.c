#include <stdint.h>
#include <string.h>
#include <stdbool.h>

// RISC-V kernel module for token lifecycle management

// Standard I/O functions for RISC-V environment
extern int read(int fd, void* buf, int count);
extern int write(int fd, const void* buf, int count);
extern void exit(int status);

// Constants
#define STDIN  0
#define STDOUT 1
#define STDERR 2

#define MAX_OBJECTS 32
#define MAX_FUNCTION_NAME 64
#define MAX_DATA_SIZE 65536
#define OBJECT_ID_SIZE 32

// Error codes
#define SUCCESS 0
#define ERROR_INVALID_FUNCTION -1
#define ERROR_INVALID_PARAMS -2
#define ERROR_INSUFFICIENT_BALANCE -3
#define ERROR_UNAUTHORIZED -4
#define ERROR_TOKEN_FROZEN -5
#define ERROR_OVERFLOW -6

// Object types
typedef struct {
    uint8_t id[OBJECT_ID_SIZE];
    uint8_t controller_id[OBJECT_ID_SIZE];
    uint32_t data_len;
    uint8_t data[MAX_DATA_SIZE];
} UnitsObject;

// Instruction structure
typedef struct {
    uint8_t controller_id[OBJECT_ID_SIZE];
    char target_function[MAX_FUNCTION_NAME];
    uint32_t num_objects;
    uint8_t target_objects[MAX_OBJECTS][OBJECT_ID_SIZE];
    uint32_t params_len;
    uint8_t params[MAX_DATA_SIZE];
} Instruction;

// Execution context
typedef struct {
    Instruction instruction;
    uint32_t num_objects;
    UnitsObject objects[MAX_OBJECTS];
} ExecutionContext;

// Object effect
typedef struct {
    uint8_t object_id[OBJECT_ID_SIZE];
    uint32_t data_len;
    uint8_t data[MAX_DATA_SIZE];
} ObjectEffect;

// Token data structure
typedef struct {
    uint64_t total_supply;
    uint8_t decimals;
    char name[64];
    char symbol[16];
    bool is_frozen;
} TokenData;

// Balance data structure
typedef struct {
    uint8_t token_id[OBJECT_ID_SIZE];
    uint8_t owner_id[OBJECT_ID_SIZE];
    uint64_t amount;
} BalanceData;

// Transfer parameters
typedef struct {
    uint64_t amount;
} TransferParams;

// Tokenize parameters
typedef struct {
    uint64_t initial_supply;
    uint8_t decimals;
    char name[64];
    char symbol[16];
} TokenizeParams;

// Mint parameters
typedef struct {
    uint64_t amount;
} MintParams;

// Helper functions
static bool ids_equal(const uint8_t* id1, const uint8_t* id2) {
    return memcmp(id1, id2, OBJECT_ID_SIZE) == 0;
}

static void copy_id(uint8_t* dest, const uint8_t* src) {
    memcpy(dest, src, OBJECT_ID_SIZE);
}

static int read_exact(void* buf, int size) {
    int total = 0;
    while (total < size) {
        int n = read(STDIN, (uint8_t*)buf + total, size - total);
        if (n <= 0) return -1;
        total += n;
    }
    return 0;
}

static int write_exact(const void* buf, int size) {
    int total = 0;
    while (total < size) {
        int n = write(STDOUT, (const uint8_t*)buf + total, size - total);
        if (n <= 0) return -1;
        total += n;
    }
    return 0;
}

// Read execution context from stdin
static int read_execution_context(ExecutionContext* ctx) {
    // Read instruction
    if (read_exact(&ctx->instruction.controller_id, OBJECT_ID_SIZE) < 0) return -1;
    if (read_exact(&ctx->instruction.target_function, MAX_FUNCTION_NAME) < 0) return -1;
    if (read_exact(&ctx->instruction.num_objects, sizeof(uint32_t)) < 0) return -1;
    
    for (uint32_t i = 0; i < ctx->instruction.num_objects; i++) {
        if (read_exact(&ctx->instruction.target_objects[i], OBJECT_ID_SIZE) < 0) return -1;
    }
    
    if (read_exact(&ctx->instruction.params_len, sizeof(uint32_t)) < 0) return -1;
    if (ctx->instruction.params_len > 0) {
        if (read_exact(&ctx->instruction.params, ctx->instruction.params_len) < 0) return -1;
    }
    
    // Read objects
    if (read_exact(&ctx->num_objects, sizeof(uint32_t)) < 0) return -1;
    
    for (uint32_t i = 0; i < ctx->num_objects; i++) {
        if (read_exact(&ctx->objects[i].id, OBJECT_ID_SIZE) < 0) return -1;
        if (read_exact(&ctx->objects[i].controller_id, OBJECT_ID_SIZE) < 0) return -1;
        if (read_exact(&ctx->objects[i].data_len, sizeof(uint32_t)) < 0) return -1;
        if (ctx->objects[i].data_len > 0) {
            if (read_exact(&ctx->objects[i].data, ctx->objects[i].data_len) < 0) return -1;
        }
    }
    
    return 0;
}

// Write object effects to stdout
static int write_effects(ObjectEffect* effects, uint32_t num_effects) {
    if (write_exact(&num_effects, sizeof(uint32_t)) < 0) return -1;
    
    for (uint32_t i = 0; i < num_effects; i++) {
        if (write_exact(&effects[i].object_id, OBJECT_ID_SIZE) < 0) return -1;
        if (write_exact(&effects[i].data_len, sizeof(uint32_t)) < 0) return -1;
        if (effects[i].data_len > 0) {
            if (write_exact(&effects[i].data, effects[i].data_len) < 0) return -1;
        }
    }
    
    return 0;
}

// Find object by ID
static UnitsObject* find_object(ExecutionContext* ctx, const uint8_t* id) {
    for (uint32_t i = 0; i < ctx->num_objects; i++) {
        if (ids_equal(ctx->objects[i].id, id)) {
            return &ctx->objects[i];
        }
    }
    return NULL;
}

// Handle transfer function
static int handle_transfer(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 3) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse parameters
    TransferParams* params = (TransferParams*)ctx->instruction.params;
    
    // Get objects
    UnitsObject* token = find_object(ctx, ctx->instruction.target_objects[0]);
    UnitsObject* from_balance = find_object(ctx, ctx->instruction.target_objects[1]);
    UnitsObject* to_balance = find_object(ctx, ctx->instruction.target_objects[2]);
    
    if (!token || !from_balance || !to_balance) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse token data
    TokenData* token_data = (TokenData*)token->data;
    if (token_data->is_frozen) {
        return ERROR_TOKEN_FROZEN;
    }
    
    // Parse balance data
    BalanceData* from_data = (BalanceData*)from_balance->data;
    BalanceData* to_data = (BalanceData*)to_balance->data;
    
    // Verify token IDs match
    if (!ids_equal(from_data->token_id, token->id) || !ids_equal(to_data->token_id, token->id)) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Check balance
    if (from_data->amount < params->amount) {
        return ERROR_INSUFFICIENT_BALANCE;
    }
    
    // Check for overflow
    if (to_data->amount + params->amount < to_data->amount) {
        return ERROR_OVERFLOW;
    }
    
    // Create effects
    ObjectEffect effects[2];
    
    // Update from balance
    BalanceData new_from_data = *from_data;
    new_from_data.amount -= params->amount;
    copy_id(effects[0].object_id, from_balance->id);
    effects[0].data_len = sizeof(BalanceData);
    memcpy(effects[0].data, &new_from_data, sizeof(BalanceData));
    
    // Update to balance
    BalanceData new_to_data = *to_data;
    new_to_data.amount += params->amount;
    copy_id(effects[1].object_id, to_balance->id);
    effects[1].data_len = sizeof(BalanceData);
    memcpy(effects[1].data, &new_to_data, sizeof(BalanceData));
    
    return write_effects(effects, 2);
}

// Handle tokenize function
static int handle_tokenize(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 2) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse parameters
    TokenizeParams* params = (TokenizeParams*)ctx->instruction.params;
    
    // Create token data
    TokenData token_data = {
        .total_supply = params->initial_supply,
        .decimals = params->decimals,
        .is_frozen = false
    };
    strncpy(token_data.name, params->name, sizeof(token_data.name) - 1);
    strncpy(token_data.symbol, params->symbol, sizeof(token_data.symbol) - 1);
    
    // Create initial balance for creator
    BalanceData creator_balance = {
        .amount = params->initial_supply
    };
    copy_id(creator_balance.token_id, ctx->instruction.target_objects[0]);
    copy_id(creator_balance.owner_id, ctx->instruction.target_objects[1]);
    
    // Create effects
    ObjectEffect effects[2];
    
    // Token object effect
    copy_id(effects[0].object_id, ctx->instruction.target_objects[0]);
    effects[0].data_len = sizeof(TokenData);
    memcpy(effects[0].data, &token_data, sizeof(TokenData));
    
    // Creator balance effect
    copy_id(effects[1].object_id, ctx->instruction.target_objects[1]);
    effects[1].data_len = sizeof(BalanceData);
    memcpy(effects[1].data, &creator_balance, sizeof(BalanceData));
    
    return write_effects(effects, 2);
}

// Handle mint function
static int handle_mint(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 2) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse parameters
    MintParams* params = (MintParams*)ctx->instruction.params;
    
    // Get objects
    UnitsObject* token = find_object(ctx, ctx->instruction.target_objects[0]);
    UnitsObject* balance = find_object(ctx, ctx->instruction.target_objects[1]);
    
    if (!token || !balance) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse data
    TokenData* token_data = (TokenData*)token->data;
    BalanceData* balance_data = (BalanceData*)balance->data;
    
    // Check for overflow
    if (token_data->total_supply + params->amount < token_data->total_supply) {
        return ERROR_OVERFLOW;
    }
    if (balance_data->amount + params->amount < balance_data->amount) {
        return ERROR_OVERFLOW;
    }
    
    // Create effects
    ObjectEffect effects[2];
    
    // Update token total supply
    TokenData new_token_data = *token_data;
    new_token_data.total_supply += params->amount;
    copy_id(effects[0].object_id, token->id);
    effects[0].data_len = sizeof(TokenData);
    memcpy(effects[0].data, &new_token_data, sizeof(TokenData));
    
    // Update balance
    BalanceData new_balance_data = *balance_data;
    new_balance_data.amount += params->amount;
    copy_id(effects[1].object_id, balance->id);
    effects[1].data_len = sizeof(BalanceData);
    memcpy(effects[1].data, &new_balance_data, sizeof(BalanceData));
    
    return write_effects(effects, 2);
}

// Handle burn function
static int handle_burn(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 2) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse parameters
    MintParams* params = (MintParams*)ctx->instruction.params; // Reuse mint params
    
    // Get objects
    UnitsObject* token = find_object(ctx, ctx->instruction.target_objects[0]);
    UnitsObject* balance = find_object(ctx, ctx->instruction.target_objects[1]);
    
    if (!token || !balance) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Parse data
    TokenData* token_data = (TokenData*)token->data;
    BalanceData* balance_data = (BalanceData*)balance->data;
    
    // Check balance
    if (balance_data->amount < params->amount) {
        return ERROR_INSUFFICIENT_BALANCE;
    }
    if (token_data->total_supply < params->amount) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Create effects
    ObjectEffect effects[2];
    
    // Update token total supply
    TokenData new_token_data = *token_data;
    new_token_data.total_supply -= params->amount;
    copy_id(effects[0].object_id, token->id);
    effects[0].data_len = sizeof(TokenData);
    memcpy(effects[0].data, &new_token_data, sizeof(TokenData));
    
    // Update balance
    BalanceData new_balance_data = *balance_data;
    new_balance_data.amount -= params->amount;
    copy_id(effects[1].object_id, balance->id);
    effects[1].data_len = sizeof(BalanceData);
    memcpy(effects[1].data, &new_balance_data, sizeof(BalanceData));
    
    return write_effects(effects, 2);
}

// Handle freeze function
static int handle_freeze(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 1) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Get token object
    UnitsObject* token = find_object(ctx, ctx->instruction.target_objects[0]);
    if (!token) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Update token data
    TokenData* token_data = (TokenData*)token->data;
    TokenData new_token_data = *token_data;
    new_token_data.is_frozen = true;
    
    // Create effect
    ObjectEffect effect;
    copy_id(effect.object_id, token->id);
    effect.data_len = sizeof(TokenData);
    memcpy(effect.data, &new_token_data, sizeof(TokenData));
    
    return write_effects(&effect, 1);
}

// Handle unfreeze function
static int handle_unfreeze(ExecutionContext* ctx) {
    if (ctx->instruction.num_objects < 1) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Get token object
    UnitsObject* token = find_object(ctx, ctx->instruction.target_objects[0]);
    if (!token) {
        return ERROR_INVALID_PARAMS;
    }
    
    // Update token data
    TokenData* token_data = (TokenData*)token->data;
    TokenData new_token_data = *token_data;
    new_token_data.is_frozen = false;
    
    // Create effect
    ObjectEffect effect;
    copy_id(effect.object_id, token->id);
    effect.data_len = sizeof(TokenData);
    memcpy(effect.data, &new_token_data, sizeof(TokenData));
    
    return write_effects(&effect, 1);
}

// Main entry point
int main(void) {
    ExecutionContext ctx;
    
    // Read execution context from stdin
    if (read_execution_context(&ctx) < 0) {
        exit(ERROR_INVALID_PARAMS);
    }
    
    // Dispatch to appropriate handler
    int result;
    if (strcmp(ctx.instruction.target_function, "transfer") == 0) {
        result = handle_transfer(&ctx);
    } else if (strcmp(ctx.instruction.target_function, "tokenize") == 0) {
        result = handle_tokenize(&ctx);
    } else if (strcmp(ctx.instruction.target_function, "mint") == 0) {
        result = handle_mint(&ctx);
    } else if (strcmp(ctx.instruction.target_function, "burn") == 0) {
        result = handle_burn(&ctx);
    } else if (strcmp(ctx.instruction.target_function, "freeze") == 0) {
        result = handle_freeze(&ctx);
    } else if (strcmp(ctx.instruction.target_function, "unfreeze") == 0) {
        result = handle_unfreeze(&ctx);
    } else {
        result = ERROR_INVALID_FUNCTION;
    }
    
    if (result < 0) {
        // Write empty effects on error
        uint32_t num_effects = 0;
        write_exact(&num_effects, sizeof(uint32_t));
    }
    
    exit(result < 0 ? result : SUCCESS);
}