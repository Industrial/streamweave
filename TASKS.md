# Task Management for Parallel AI Workers

## How to Update This File

**IMPORTANT: Multiple AI agents will be working on this task list in parallel. Use `sed` commands to make atomic updates to avoid conflicts.**

### Updating Task Status

Use `sed` to mark tasks as in-progress or completed:

```bash
# Mark a task as in-progress
sed -i 's/- \[ \] 1.1.1/- [x] 1.1.1/' TASKS.md

# Mark a task as completed
sed -i 's/- [x] 1.1.1/- [x] 1.1.1/' TASKS.md

# Mark multiple tasks at once
sed -i 's/- \[ \] 1.1.1/- [x] 1.1.1/' TASKS.md && sed -i 's/- \[ \] 1.1.2/- [x] 1.1.2/' TASKS.md
```

### Best Practices for Parallel Work

1. **Always use `sed` for status updates** - This ensures atomic file modifications
2. **Work on different task numbers** - Check the file before starting to avoid conflicts
3. **Mark tasks in-progress immediately** - Use `- [o]` when you start working on a task
4. **Mark tasks completed when done** - Use `- [x]` when the task is finished
5. **Update parent tasks** - When all subtasks are done, mark the parent task as completed too

### Task Status Format

- `- [ ]` = Not started (available for work)
- `- [o]` = In progress (currently being worked on)
- `- [x]` = Completed (finished)

---

# General-Purpose FBP Node Implementation Tasks

## 0. Standardize Existing Nodes (HIGHEST PRIORITY - DO FIRST)

**Context:** All nodes must follow the standard port pattern: `configuration` input port and `error` output port for consistency, composability, and future flexibility.

**Standard Port Pattern:**
- **Input Ports:** `configuration` (optional but should exist), plus data input ports (`in`, `in1`, `in2`, etc.)
- **Output Ports:** Data output ports (`out`, `true`, `false`, etc.), plus `error`

### 0.1 Fix Boolean Logic Nodes

- [x] 0.1 All boolean logic nodes standardized

- [x] 0.1.1 Add `configuration` port to AndNode
  - **File:** `src/graph/nodes/boolean_logic/and_node.rs`
  - **Context:** Add `configuration` input port for consistency (even if unused)
  - **Current Ports:** `in1`, `in2` → `out`, `error`
  - **New Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added to `input_port_names`
    - Port handling in `execute()` method (can ignore config for now)
    - `has_input_port()` updated
    - Tests updated to verify port exists
    - All existing tests still pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.2 Add `configuration` port to OrNode
  - **File:** `src/graph/nodes/boolean_logic/or_node.rs`
  - **Context:** Add `configuration` input port for consistency
  - **Current Ports:** `in1`, `in2` → `out`, `error`
  - **New Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added
    - Port handling updated
    - Tests updated
    - All existing tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.3 Add `configuration` port to NotNode
  - **File:** `src/graph/nodes/boolean_logic/not_node.rs`
  - **Context:** Add `configuration` input port for consistency
  - **Current Ports:** `in` → `out`, `error`
  - **New Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added
    - Port handling updated
    - Tests updated
    - All existing tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.4 Add `configuration` port to XorNode
  - **File:** `src/graph/nodes/boolean_logic/xor_node.rs`
  - **Context:** Add `configuration` input port for consistency
  - **Current Ports:** `in1`, `in2` → `out`, `error`
  - **New Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added
    - Port handling updated
    - Tests updated
    - All existing tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.5 Add `configuration` port to NandNode
  - **File:** `src/graph/nodes/boolean_logic/nand_node.rs`
  - **Context:** Add `configuration` input port for consistency
  - **Current Ports:** `in1`, `in2` → `out`, `error`
  - **New Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added
    - Port handling updated
    - Tests updated
    - All existing tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.6 Add `configuration` port to NorNode
  - **File:** `src/graph/nodes/boolean_logic/nor_node.rs`
  - **Context:** Add `configuration` input port for consistency
  - **Current Ports:** `in1`, `in2` → `out`, `error`
  - **New Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - `configuration` port added
    - Port handling updated
    - Tests updated
    - All existing tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 0.1.7 Update boolean logic module documentation
  - **File:** `src/graph/nodes/boolean_logic/mod.rs`
  - **Context:** Document the standard port pattern
  - **Acceptance Criteria:**
    - Documentation mentions `configuration` port
    - Examples show standard pattern
    - All examples compile
    - `bin/pre-commit` succeeds
    - Git commit is made

## 1. Core Control Flow Nodes (HIGHEST PRIORITY)

### 1.1 Pattern Matching & Conditional Routing

- [x] 1.1.1 Implement MatchNode - Pattern matching with multiple branches
  - **File:** `src/graph/nodes/match_node.rs`
  - **Ports:** `configuration`, `in` → `out_0`, `out_1`, ..., `out_n`, `default`, `error`
  - **Context:** Routes items based on pattern matching (enum variants, ranges, regex, etc.)
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple pattern branches
    - Supports default branch
    - Handles pattern matching errors
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.1.2 Create MatchConfig trait for pattern definitions
  - **File:** `src/graph/nodes/match_node.rs`
  - **Context:** Define trait for pattern matching functions
  - **Acceptance Criteria:**
    - Trait allows custom pattern matching
    - Supports common patterns (enum, range, regex)
    - Type-safe pattern matching
    - Documentation with examples
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.1.3 Add MatchNode tests
  - **File:** `src/graph/nodes/match_node_test.rs`
  - **Acceptance Criteria:**
    - Tests for enum variant matching
    - Tests for range matching
    - Tests for regex matching
    - Tests for default branch
    - Tests for error handling
    - `bin/pre-commit` succeeds
    - Git commit is made

### 1.2 Loops & Iteration

- [x] 1.2.1 Implement ForEachNode - Iterate over collections
  - **File:** `src/graph/nodes/for_each_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Context:** Expands collections into streams
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Extracts collections from input items
    - Emits each collection item as separate output
    - Handles empty collections
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.2.2 Implement WhileLoopNode - While loop with condition
  - **File:** `src/graph/nodes/while_loop_node.rs`
  - **Ports:** `configuration`, `in`, `condition` → `out`, `break`, `error`
  - **Context:** Repeats processing until condition is false
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Evaluates condition for each iteration
    - Supports break signal
    - Handles infinite loop prevention
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.2.3 Implement RangeNode - Generate number ranges
  - **File:** `src/graph/nodes/range_node.rs`
  - **Ports:** `configuration`, `start`, `end`, `step` → `out`, `error`
  - **Context:** Produces ranges for iteration
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Generates number sequences
    - Supports configurable step size
    - Handles invalid ranges
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.2.4 Add loop node tests
  - **File:** `src/graph/nodes/for_each_node_test.rs`, `src/graph/nodes/while_loop_node_test.rs`, `src/graph/nodes/range_node_test.rs`
  - **Acceptance Criteria:**
    - Tests for each loop type
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 1.3 Error Handling

- [x] 1.3.1 Implement ErrorBranchNode - Route Result<T, E> to success/error paths
  - **File:** `src/graph/nodes/error_branch_node.rs`
  - **Ports:** `configuration`, `in` → `success`, `error`
  - **Context:** Splits `Result` streams
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Routes `Ok` values to success port
    - Routes `Err` values to error port
    - Preserves error context
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.3.2 Add ErrorBranchNode tests
  - **File:** `src/graph/nodes/error_branch_node_test.rs`
  - **Acceptance Criteria:**
    - Tests for success routing
    - Tests for error routing
    - Tests for error context preservation
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 1.4 Variables & State

- [x] 1.4.1 Implement VariableNode - Graph-level variable storage
  - **File:** `src/graph/nodes/variable_node.rs`
  - **Ports:** `configuration`, `read`, `write`, `value` → `error`
  - **Context:** Shared state between nodes
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Thread-safe variable storage
    - Supports read and write operations
    - Handles concurrent access
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.4.2 Implement ReadVariableNode - Read variable value
  - **File:** `src/graph/nodes/read_variable_node.rs`
  - **Ports:** `configuration`, `name` → `out`, `error`
  - **Context:** Read variable from graph state
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Reads variable by name
    - Handles missing variables
    - Type-safe variable access
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.4.3 Implement WriteVariableNode - Write variable value
  - **File:** `src/graph/nodes/write_variable_node.rs`
  - **Ports:** `configuration`, `name`, `value` → `out`, `error`
  - **Context:** Write variable to graph state
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Writes variable by name
    - Handles type mismatches
    - Thread-safe writes
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.4.4 Add variable node tests
  - **File:** `src/graph/nodes/variable_node_test.rs`, `src/graph/nodes/read_variable_node_test.rs`, `src/graph/nodes/write_variable_node_test.rs`
  - **Acceptance Criteria:**
    - Tests for each variable operation
    - Tests for concurrent access
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 1.5 Synchronization

- [x] 1.5.1 Implement SyncNode - Wait for all inputs before proceeding
  - **File:** `src/graph/nodes/sync_node.rs`
  - **Ports:** `configuration`, `in_0`, `in_1`, ..., `in_n` → `out`, `error`
  - **Context:** Barrier synchronization
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Waits for all inputs
    - Emits combined output when all ready
    - Handles timeout scenarios
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.5.2 Implement JoinNode - Join two streams on keys
  - **File:** `src/graph/nodes/join_node.rs`
  - **Ports:** `configuration`, `left`, `right` → `out`, `error`
  - **Context:** Inner/outer/left/right joins
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple join strategies
    - Key-based joining
    - Handles unmatched keys
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 1.5.3 Add synchronization node tests
  - **File:** `src/graph/nodes/sync_node_test.rs`, `src/graph/nodes/join_node_test.rs`
  - **Acceptance Criteria:**
    - Tests for each synchronization type
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 2. Arithmetic & Math Operations (HIGH PRIORITY)

### 2.1 Basic Arithmetic Operations

- [x] 2.1.1 Implement AddNode - Addition
  - **File:** `src/graph/nodes/arithmetic/add_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles overflow
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.2 Implement SubtractNode - Subtraction
  - **File:** `src/graph/nodes/arithmetic/subtract_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles underflow
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.3 Implement MultiplyNode - Multiplication
  - **File:** `src/graph/nodes/arithmetic/multiply_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles overflow
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.4 Implement DivideNode - Division
  - **File:** `src/graph/nodes/arithmetic/divide_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles division by zero
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.5 Implement ModuloNode - Modulo operation
  - **File:** `src/graph/nodes/arithmetic/modulo_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports integer types
    - Handles division by zero
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.6 Implement PowerNode - Exponentiation
  - **File:** `src/graph/nodes/arithmetic/power_node.rs`
  - **Ports:** `configuration`, `base`, `exponent` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles edge cases
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.1.7 Add arithmetic node tests
  - **File:** `src/graph/nodes/arithmetic/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 2.2 Comparison Operations

- [x] 2.2.1 Implement EqualNode - Equality comparison
  - **File:** `src/graph/nodes/comparison/equal_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.2 Implement NotEqualNode - Inequality comparison
  - **File:** `src/graph/nodes/comparison/not_equal_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.3 Implement GreaterThanNode - Greater than
  - **File:** `src/graph/nodes/comparison/greater_than_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.4 Implement GreaterThanOrEqualNode - Greater than or equal
  - **File:** `src/graph/nodes/comparison/greater_than_or_equal_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.5 Implement LessThanNode - Less than
  - **File:** `src/graph/nodes/comparison/less_than_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.6 Implement LessThanOrEqualNode - Less than or equal
  - **File:** `src/graph/nodes/comparison/less_than_or_equal_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.2.7 Add comparison node tests
  - **File:** `src/graph/nodes/comparison/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each comparison
    - Tests for edge cases
    - Tests for type compatibility
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 2.3 Math Functions

- [x] 2.3.1 Implement AbsNode - Absolute value
  - **File:** `src/graph/nodes/math/abs_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.2 Implement MinNode - Minimum of two values
  - **File:** `src/graph/nodes/math/min_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.3 Implement MaxNode - Maximum of two values
  - **File:** `src/graph/nodes/math/max_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports comparable types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.4 Implement RoundNode - Rounding
  - **File:** `src/graph/nodes/math/round_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports floating point types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.5 Implement FloorNode - Floor
  - **File:** `src/graph/nodes/math/floor_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports floating point types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.6 Implement CeilNode - Ceiling
  - **File:** `src/graph/nodes/math/ceil_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports floating point types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.7 Implement SqrtNode - Square root
  - **File:** `src/graph/nodes/math/sqrt_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles negative inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.8 Implement LogNode - Logarithm
  - **File:** `src/graph/nodes/math/log_node.rs`
  - **Ports:** `configuration`, `in`, `base` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles invalid inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.9 Implement ExpNode - Exponential
  - **File:** `src/graph/nodes/math/exp_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports numeric types
    - Handles overflow
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.10 Implement SinNode, CosNode, TanNode - Trigonometric functions
  - **File:** `src/graph/nodes/math/trigonometric_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports floating point types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 2.3.11 Add math function node tests
  - **File:** `src/graph/nodes/math/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each function
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 2.4 Advanced Math Functions

- [ ] 2.4.1 Implement TrigonometricNode - Trigonometric functions
  - **File:** `src/graph/nodes/math/trigonometric_node.rs`
  - **Ports:** `configuration`, `in`, `function_type` → `out`, `error`
  - **Context:** sin, cos, tan, asin, acos, atan functions
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports sin, cos, tan operations
    - Supports inverse functions (asin, acos, atan)
    - Handles domain errors
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 2.4.2 Implement HyperbolicNode - Hyperbolic functions
  - **File:** `src/graph/nodes/math/hyperbolic_node.rs`
  - **Ports:** `configuration`, `in`, `function_type` → `out`, `error`
  - **Context:** sinh, cosh, tanh functions
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports sinh, cosh, tanh operations
    - Handles overflow cases
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 2.4.3 Implement RandomNode - Random number generation
  - **File:** `src/graph/nodes/math/random_node.rs`
  - **Ports:** `configuration`, `min`, `max` → `out`, `error`
  - **Context:** Generate random numbers within a range
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Generates random numbers in specified range
    - Supports integer and floating point types
    - Handles invalid ranges
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 2.4.4 Add advanced math function tests
  - **File:** `src/graph/nodes/math/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each function
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 3. String Operations (HIGH PRIORITY)

### 3.1 Basic String Operations

- [x] 3.1.1 Implement StringConcatNode - Concatenate strings
  - **File:** `src/graph/nodes/string/concat_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Concatenates two strings
    - Handles non-string inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.2 Implement StringLengthNode - Get string length
  - **File:** `src/graph/nodes/string/length_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns character count
    - Handles non-string inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.3 Implement StringSliceNode - Extract substring
  - **File:** `src/graph/nodes/string/slice_node.rs`
  - **Ports:** `configuration`, `in`, `start`, `end` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Extracts substring by indices
    - Handles invalid indices
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.4 Implement StringReplaceNode - Replace substrings
  - **File:** `src/graph/nodes/string/replace_node.rs`
  - **Ports:** `configuration`, `in`, `pattern`, `replacement` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Replaces all occurrences
    - Handles regex patterns
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.5 Implement StringSplitNode - Split string into array
  - **File:** `src/graph/nodes/string/split_node.rs`
  - **Ports:** `configuration`, `in`, `delimiter` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Splits by delimiter
    - Returns array of strings
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.6 Implement StringJoinNode - Join array into string
  - **File:** `src/graph/nodes/string/join_node.rs`
  - **Ports:** `configuration`, `in`, `delimiter` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Joins array elements
    - Uses delimiter
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.1.7 Add basic string operation tests
  - **File:** `src/graph/nodes/string/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 3.2 String Predicates

- [x] 3.2.1 Implement StringContainsNode - Check if contains substring
  - **File:** `src/graph/nodes/string/contains_node.rs`
  - **Ports:** `configuration`, `in`, `substring` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Case-sensitive option
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.2.2 Implement StringStartsWithNode - Check if starts with
  - **File:** `src/graph/nodes/string/starts_with_node.rs`
  - **Ports:** `configuration`, `in`, `prefix` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.2.3 Implement StringEndsWithNode - Check if ends with
  - **File:** `src/graph/nodes/string/ends_with_node.rs`
  - **Ports:** `configuration`, `in`, `suffix` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.2.4 Implement StringMatchNode - Regex matching
  - **File:** `src/graph/nodes/string/match_node.rs`
  - **Ports:** `configuration`, `in`, `pattern` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean or matches
    - Handles invalid regex
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.2.5 Implement StringEqualNode - String equality
  - **File:** `src/graph/nodes/string/equal_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Case-sensitive option
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.2.6 Add string predicate tests
  - **File:** `src/graph/nodes/string/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each predicate
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 3.3 String Transformations

- [x] 3.3.1 Implement StringCaseNode - Case conversion (upper/lower/title)
  - **File:** `src/graph/nodes/string/case_node.rs`
  - **Ports:** `configuration`, `in`, `case_type` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple case types
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.3.2 Implement StringTrimNode - Trim whitespace
  - **File:** `src/graph/nodes/string/trim_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Trims leading/trailing whitespace
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.3.3 Implement StringPadNode - Pad strings
  - **File:** `src/graph/nodes/string/pad_node.rs`
  - **Ports:** `configuration`, `in`, `length`, `padding` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Pads to specified length
    - Supports left/right/center padding
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.3.4 Implement StringReverseNode - Reverse string
  - **File:** `src/graph/nodes/string/reverse_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Reverses character order
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 3.3.5 Add string transformation tests
  - **File:** `src/graph/nodes/string/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each transformation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 3.4 Additional String Operations

- [ ] 3.4.1 Implement StringIndexOfNode - Find index of substring
  - **File:** `src/graph/nodes/string/index_of_node.rs`
  - **Ports:** `configuration`, `in`, `substring` → `out`, `error`
  - **Context:** Finds the index of the first occurrence of a substring
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns index of substring or -1 if not found
    - Supports optional start index parameter
    - Handles empty strings
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.2 Implement StringRepeatNode - Repeat string N times
  - **File:** `src/graph/nodes/string/repeat_node.rs`
  - **Ports:** `configuration`, `in`, `count` → `out`, `error`
  - **Context:** Repeats a string a specified number of times
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Repeats string N times
    - Handles zero and negative counts
    - Handles non-string inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.3 Implement StringCharNode - Character operations
  - **File:** `src/graph/nodes/string/char_node.rs`
  - **Ports:** `configuration`, `in`, `index` → `out`, `error`
  - **Context:** Get character at index or character code operations
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Gets character at specified index
    - Supports char code conversion
    - Handles out-of-bounds indices
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.4 Implement StringPredicateNode - String predicate checks
  - **File:** `src/graph/nodes/string/predicate_node.rs`
  - **Ports:** `configuration`, `in`, `predicate_type` → `out`, `error`
  - **Context:** Check string properties (is_empty, is_whitespace, is_digit, is_alpha, etc.)
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports multiple predicate types
    - Returns boolean result
    - Handles non-string inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.5 Implement StringSearchNode - Search with regex
  - **File:** `src/graph/nodes/string/search_node.rs`
  - **Ports:** `configuration`, `in`, `pattern` → `out`, `error`
  - **Context:** Search for regex pattern matches in string
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Finds regex matches
    - Returns match positions or matched text
    - Handles invalid regex patterns
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.6 Implement StringSplitLinesNode - Split by newlines
  - **File:** `src/graph/nodes/string/split_lines_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Context:** Split string into array of lines
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Splits by newline characters
    - Handles different line ending formats (LF, CRLF, CR)
    - Returns array of strings
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.7 Implement StringSplitWordsNode - Split by words
  - **File:** `src/graph/nodes/string/split_words_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Context:** Split string into array of words
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Splits by whitespace
    - Handles multiple consecutive spaces
    - Returns array of strings
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 3.4.8 Add additional string operation tests
  - **File:** `src/graph/nodes/string/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each new operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 4. Array/Collection Operations (HIGH PRIORITY)

### 4.1 Array Access

- [x] 4.1.1 Implement ArrayLengthNode - Get array length
  - **File:** `src/graph/nodes/array/length_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns array length
    - Handles non-array inputs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.1.2 Implement ArrayIndexNode - Access by index
  - **File:** `src/graph/nodes/array/index_node.rs`
  - **Ports:** `configuration`, `in`, `index` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Accesses element by index
    - Handles out-of-bounds
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.1.3 Implement ArraySliceNode - Extract slice
  - **File:** `src/graph/nodes/array/slice_node.rs`
  - **Ports:** `configuration`, `in`, `start`, `end` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Extracts array slice
    - Handles invalid indices
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.1.4 Implement ArrayContainsNode - Check if contains value
  - **File:** `src/graph/nodes/array/contains_node.rs`
  - **Ports:** `configuration`, `in`, `value` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.1.5 Implement ArrayIndexOfNode - Find index of value
  - **File:** `src/graph/nodes/array/index_of_node.rs`
  - **Ports:** `configuration`, `in`, `value` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns index or -1
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.1.6 Add array access tests
  - **File:** `src/graph/nodes/array/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 4.2 Array Transformations

- [x] 4.2.1 Implement ArrayConcatNode - Concatenate arrays
  - **File:** `src/graph/nodes/array/concat_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Concatenates two arrays
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.2.2 Implement ArrayReverseNode - Reverse array
  - **File:** `src/graph/nodes/array/reverse_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Reverses element order
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.2.3 Implement ArraySortNode - Sort array
  - **File:** `src/graph/nodes/array/sort_node.rs`
  - **Ports:** `configuration`, `in`, `order` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Sorts elements
    - Supports ascending/descending
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.2.4 Implement ArrayFilterNode - Filter array elements
  - **File:** `src/graph/nodes/array/filter_node.rs`
  - **Ports:** `configuration`, `in`, `predicate` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Filters by predicate
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.2.5 Implement ArrayMapNode - Map over array
  - **File:** `src/graph/nodes/array/map_node.rs`
  - **Ports:** `configuration`, `in`, `function` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Applies function to each element
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.2.6 Add array transformation tests
  - **File:** `src/graph/nodes/array/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each transformation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 4.3 Array Operations

- [x] 4.3.1 Implement ArrayJoinNode - Join array elements
  - **File:** `src/graph/nodes/array/join_node.rs`
  - **Ports:** `configuration`, `in`, `delimiter` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Joins elements with delimiter
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.3.2 Implement ArraySplitNode - Split into chunks
  - **File:** `src/graph/nodes/array/split_node.rs`
  - **Ports:** `configuration`, `in`, `chunk_size` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Splits into chunks
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.3.3 Implement ArrayFlattenNode - Flatten nested arrays
  - **File:** `src/graph/nodes/array/flatten_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Flattens one level
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.3.4 Implement ArrayUniqueNode - Remove duplicates
  - **File:** `src/graph/nodes/array/unique_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Removes duplicate elements
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 4.3.5 Add array operation tests
  - **File:** `src/graph/nodes/array/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 4.4 Additional Array Operations

- [ ] 4.4.1 Implement ArrayFindNode - Find element matching predicate
  - **File:** `src/graph/nodes/array/find_node.rs`
  - **Ports:** `configuration`, `in`, `predicate` → `out`, `error`
  - **Context:** Find first element matching a predicate function
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Finds first matching element
    - Returns element or null if not found
    - Supports custom predicate functions
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 4.4.2 Implement ArrayModifyNode - Modify array elements in-place
  - **File:** `src/graph/nodes/array/modify_node.rs`
  - **Ports:** `configuration`, `in`, `index`, `value` → `out`, `error`
  - **Context:** Modify array element at specified index
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Modifies element at index
    - Handles out-of-bounds indices
    - Returns modified array
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 4.4.3 Add additional array operation tests
  - **File:** `src/graph/nodes/array/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each new operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 5. Object/Map Operations (MEDIUM PRIORITY)

### 5.1 Object Access

- [x] 5.1.1 Implement ObjectKeysNode - Get object keys
  - **File:** `src/graph/nodes/object/keys_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns array of keys
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.1.2 Implement ObjectValuesNode - Get object values
  - **File:** `src/graph/nodes/object/values_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns array of values
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.1.3 Implement ObjectEntriesNode - Get key-value pairs
  - **File:** `src/graph/nodes/object/entries_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns array of pairs
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.1.4 Implement ObjectPropertyNode - Get property value
  - **File:** `src/graph/nodes/object/property_node.rs`
  - **Ports:** `configuration`, `in`, `key` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns property value
    - Handles missing properties
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.1.5 Implement ObjectHasPropertyNode - Check if property exists
  - **File:** `src/graph/nodes/object/has_property_node.rs`
  - **Ports:** `configuration`, `in`, `key` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.1.6 Add object access tests
  - **File:** `src/graph/nodes/object/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 5.2 Object Transformations

- [x] 5.2.1 Implement ObjectMergeNode - Merge objects
  - **File:** `src/graph/nodes/object/merge_node.rs`
  - **Ports:** `configuration`, `in1`, `in2` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Merges two objects
    - Handles key conflicts
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.2.2 Implement ObjectSetPropertyNode - Set property
  - **File:** `src/graph/nodes/object/set_property_node.rs`
  - **Ports:** `configuration`, `in`, `key`, `value` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Sets property value
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.2.3 Implement ObjectDeletePropertyNode - Delete property
  - **File:** `src/graph/nodes/object/delete_property_node.rs`
  - **Ports:** `configuration`, `in`, `key` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Removes property
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 5.2.4 Add object transformation tests
  - **File:** `src/graph/nodes/object/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each transformation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 5.3 Additional Object Operations

- [ ] 5.3.1 Implement ObjectRandomMemberNode - Get random key-value pair
  - **File:** `src/graph/nodes/object/random_member_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Context:** Get a random key-value pair from object
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns random key-value pair
    - Handles empty objects
    - Returns consistent format (key, value tuple)
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 5.3.2 Add additional object operation tests
  - **File:** `src/graph/nodes/object/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for random member operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 6. Aggregation & Reduction (MEDIUM PRIORITY)

### 6.1 Aggregation

- [x] 6.1.1 Implement SumNode - Sum values
  - **File:** `src/graph/nodes/aggregation/sum_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Sums numeric values
    - Handles empty streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.1.2 Implement CountNode - Count items
  - **File:** `src/graph/nodes/aggregation/count_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Counts items in stream
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.1.3 Implement AverageNode - Calculate average
  - **File:** `src/graph/nodes/aggregation/average_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Calculates mean
    - Handles empty streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.1.4 Implement MinAggregateNode - Minimum value
  - **File:** `src/graph/nodes/aggregation/min_aggregate_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Finds minimum
    - Handles empty streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.1.5 Implement MaxAggregateNode - Maximum value
  - **File:** `src/graph/nodes/aggregation/max_aggregate_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Finds maximum
    - Handles empty streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.1.6 Add aggregation tests
  - **File:** `src/graph/nodes/aggregation/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each aggregation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 6.2 Reduction

- [x] 6.2.1 Implement ReduceNode - General reduction
  - **File:** `src/graph/nodes/reduction/reduce_node.rs`
  - **Ports:** `configuration`, `in`, `initial`, `function` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Applies reduction function
    - Supports initial value
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.2.2 Implement GroupByNode - Group by key
  - **File:** `src/graph/nodes/reduction/group_by_node.rs`
  - **Ports:** `configuration`, `in`, `key_function` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Groups items by key
    - Returns grouped results
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.2.3 Implement AggregateNode - General aggregation
  - **File:** `src/graph/nodes/reduction/aggregate_node.rs`
  - **Ports:** `configuration`, `in`, `aggregator` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Supports custom aggregators
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 6.2.4 Add reduction tests
  - **File:** `src/graph/nodes/reduction/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each reduction
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 7. Time & Delay Operations (MEDIUM PRIORITY)

### 7.1 Time Operations

- [x] 7.1.1 Implement DelayNode - Delay items by duration
  - **File:** `src/graph/nodes/time/delay_node.rs`
  - **Ports:** `configuration`, `in`, `duration` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Delays each item
    - Supports configurable duration
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 7.1.2 Implement TimeoutNode - Apply timeout
  - **File:** `src/graph/nodes/time/timeout_node.rs`
  - **Ports:** `configuration`, `in`, `timeout` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Times out after duration
    - Returns error on timeout
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 7.1.3 Implement TimerNode - Generate periodic events
  - **File:** `src/graph/nodes/time/timer_node.rs`
  - **Ports:** `configuration`, `interval` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Generates periodic events
    - Supports configurable interval
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 7.1.4 Implement TimestampNode - Add timestamps
  - **File:** `src/graph/nodes/time/timestamp_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Adds timestamp to items
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 7.1.5 Add time operation tests
  - **File:** `src/graph/nodes/time/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for timing accuracy
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 8. Stream Control (MEDIUM PRIORITY)

### 8.1 Stream Control

- [x] 8.1.1 Implement TakeNode - Take first N items
  - **File:** `src/graph/nodes/stream/take_node.rs`
  - **Ports:** `configuration`, `in`, `count` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Takes first N items
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.1.2 Implement SkipNode - Skip first N items
  - **File:** `src/graph/nodes/stream/skip_node.rs`
  - **Ports:** `configuration`, `in`, `count` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Skips first N items
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.1.3 Implement LimitNode - Limit stream size
  - **File:** `src/graph/nodes/stream/limit_node.rs`
  - **Ports:** `configuration`, `in`, `max_size` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Limits stream to max size
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.1.4 Implement DropNode - Drop items
  - **File:** `src/graph/nodes/stream/drop_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Drops all items
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.1.5 Implement SampleNode - Sample items
  - **File:** `src/graph/nodes/stream/sample_node.rs`
  - **Ports:** `configuration`, `in`, `rate` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Samples items by rate
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.1.6 Add stream control tests
  - **File:** `src/graph/nodes/stream/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 8.2 Stream Combination

- [x] 8.2.1 Implement ZipNode - Zip multiple streams
  - **File:** `src/graph/nodes/stream/zip_node.rs`
  - **Ports:** `configuration`, `in_0`, `in_1`, ..., `in_n` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Zips multiple streams
    - Handles different stream lengths
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.2.2 Implement InterleaveNode - Interleave streams
  - **File:** `src/graph/nodes/stream/interleave_node.rs`
  - **Ports:** `configuration`, `in_0`, `in_1`, ..., `in_n` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Interleaves items from streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.2.3 Implement MergeNode - Merge streams
  - **File:** `src/graph/nodes/stream/merge_node.rs`
  - **Ports:** `configuration`, `in_0`, `in_1`, ..., `in_n` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Merges multiple streams
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 8.2.4 Add stream combination tests
  - **File:** `src/graph/nodes/stream/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 8.3 Advanced Stream Operations

- [ ] 8.3.1 Implement PartitionNode - Partition stream into multiple streams
  - **File:** `src/graph/nodes/stream/partition_node.rs`
  - **Ports:** `configuration`, `in`, `predicate` → `out_true`, `out_false`, `error`
  - **Context:** Split stream into two streams based on predicate
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Routes items to true/false streams based on predicate
    - Supports custom predicate functions
    - Handles predicate errors
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 8.3.2 Implement SplitAtNode - Split stream at index
  - **File:** `src/graph/nodes/stream/split_at_node.rs`
  - **Ports:** `configuration`, `in`, `index` → `out_before`, `out_after`, `error`
  - **Context:** Split stream into two streams at specified index
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Splits stream at specified index
    - Returns items before and after index
    - Handles index out of bounds
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 8.3.3 Implement OrderedMergeNode - Merge streams maintaining order
  - **File:** `src/graph/nodes/stream/ordered_merge_node.rs`
  - **Ports:** `configuration`, `in_0`, `in_1`, ..., `in_n`, `key_function` → `out`, `error`
  - **Context:** Merge multiple sorted streams maintaining sort order
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Merges streams while maintaining sort order
    - Supports custom key extraction function
    - Handles unsorted input gracefully
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 8.3.4 Implement BatchNode - Batch items into groups
  - **File:** `src/graph/nodes/stream/batch_node.rs`
  - **Ports:** `configuration`, `in`, `batch_size` → `out`, `error`
  - **Context:** Group items into batches of specified size
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Groups items into batches
    - Supports configurable batch size
    - Handles partial batches at end
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 8.3.5 Add advanced stream operation tests
  - **File:** `src/graph/nodes/stream/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each new operation
    - Tests for edge cases
    - Tests for error handling
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 9. Type Operations (LOW-MEDIUM PRIORITY)

### 9.1 Type Checking

- [x] 9.1.1 Implement TypeOfNode - Get type of value
  - **File:** `src/graph/nodes/type_ops/type_of_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns type name
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.2 Implement IsNumberNode - Check if number
  - **File:** `src/graph/nodes/type_ops/is_number_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.3 Implement IsStringNode - Check if string
  - **File:** `src/graph/nodes/type_ops/is_string_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.4 Implement IsBooleanNode - Check if boolean
  - **File:** `src/graph/nodes/type_ops/is_boolean_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.5 Implement IsArrayNode - Check if array
  - **File:** `src/graph/nodes/type_ops/is_array_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.6 Implement IsObjectNode - Check if object
  - **File:** `src/graph/nodes/type_ops/is_object_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.7 Implement IsNullNode - Check if null
  - **File:** `src/graph/nodes/type_ops/is_null_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Returns boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.1.8 Add type checking tests
  - **File:** `src/graph/nodes/type_ops/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each type check
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 9.2 Type Conversion

- [x] 9.2.1 Implement ToStringNode - Convert to string
  - **File:** `src/graph/nodes/type_ops/to_string_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Converts to string
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.2.2 Implement ToNumberNode - Convert to number
  - **File:** `src/graph/nodes/type_ops/to_number_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Converts to number
    - Handles invalid conversions
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.2.3 Implement ToBooleanNode - Convert to boolean
  - **File:** `src/graph/nodes/type_ops/to_boolean_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Converts to boolean
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.2.4 Implement ToArrayNode - Convert to array
  - **File:** `src/graph/nodes/type_ops/to_array_node.rs`
  - **Ports:** `configuration`, `in` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Converts to array
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 9.2.5 Add type conversion tests
  - **File:** `src/graph/nodes/type_ops/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each conversion
    - Tests for invalid conversions
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 10. Advanced Control Flow (LOW PRIORITY)

### 10.1 Advanced Loops

- [x] 10.1.1 Implement RepeatNode - Repeat N times
  - **File:** `src/graph/nodes/advanced/repeat_node.rs`
  - **Ports:** `configuration`, `in`, `count` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Repeats items N times
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.1.2 Implement BreakNode - Break from loop
  - **File:** `src/graph/nodes/advanced/break_node.rs`
  - **Ports:** `configuration`, `in`, `signal` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Sends break signal
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.1.3 Implement ContinueNode - Continue loop iteration
  - **File:** `src/graph/nodes/advanced/continue_node.rs`
  - **Ports:** `configuration`, `in`, `signal` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Sends continue signal
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.1.4 Add advanced loop tests
  - **File:** `src/graph/nodes/advanced/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

### 10.2 Advanced Control

- [x] 10.2.1 Implement SwitchNode - Multi-way switch
  - **File:** `src/graph/nodes/advanced/switch_node.rs`
  - **Ports:** `configuration`, `in`, `value` → `out_0`, `out_1`, ..., `out_n`, `default`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Routes based on value
    - Supports default case
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.2.2 Implement TryCatchNode - Try-catch error handling
  - **File:** `src/graph/nodes/advanced/try_catch_node.rs`
  - **Ports:** `configuration`, `in`, `try`, `catch` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Handles errors gracefully
    - Routes to catch on error
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.2.3 Implement RetryNode - Retry on failure
  - **File:** `src/graph/nodes/advanced/retry_node.rs`
  - **Ports:** `configuration`, `in`, `max_retries` → `out`, `error`
  - **Acceptance Criteria:**
    - Node implements `Node` trait
    - Retries on failure
    - Supports exponential backoff
    - Comprehensive tests
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 10.2.4 Add advanced control tests
  - **File:** `src/graph/nodes/advanced/*_test.rs`
  - **Acceptance Criteria:**
    - Tests for each operation
    - Tests for edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

## 11. Module Organization & Integration

### 11.1 Module Structure

- [x] 11.1.1 Create module directories for node categories
  - **Context:** Organize nodes into logical modules
  - **Acceptance Criteria:**
    - `src/graph/nodes/arithmetic/` exists
    - `src/graph/nodes/comparison/` exists
    - `src/graph/nodes/string/` exists
    - `src/graph/nodes/array/` exists
    - `src/graph/nodes/object/` exists
    - `src/graph/nodes/aggregation/` exists
    - `src/graph/nodes/reduction/` exists
    - `src/graph/nodes/time/` exists
    - `src/graph/nodes/stream/` exists
    - `src/graph/nodes/type_ops/` exists
    - `src/graph/nodes/advanced/` exists
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 11.1.2 Update mod.rs files for each module
  - **Context:** Export all nodes from modules
  - **Acceptance Criteria:**
    - Each module has mod.rs
    - All nodes are exported
    - Tests are properly organized
    - All code compiles
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 11.1.3 Update main nodes/mod.rs to export all categories
  - **Context:** Central export point for all nodes
  - **Acceptance Criteria:**
    - All node categories exported
    - Clear organization
    - Documentation updated
    - All code compiles
    - `bin/pre-commit` succeeds
    - Git commit is made

### 11.2 Documentation

- [x] 11.2.1 Add module-level documentation
  - **Context:** Document each node category
  - **Acceptance Criteria:**
    - Each module has documentation
    - Examples provided
    - Usage patterns explained
    - All examples compile
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 11.2.2 Add node-level documentation
  - **Context:** Document each node implementation
  - **Acceptance Criteria:**
    - Each node has rustdoc
    - Ports documented
    - Examples provided
    - Error handling explained
    - `bin/pre-commit` succeeds
    - Git commit is made

- [x] 11.2.3 Create usage examples
  - **Context:** Provide real-world examples
  - **Acceptance Criteria:**
    - Examples for each category
    - Complex examples showing composition
    - All examples runnable
    - Examples in examples/ directory
    - `bin/pre-commit` succeeds
    - Git commit is made

### 11.3 Testing Infrastructure

- [ ] 11.3.1 Ensure all nodes have tests
  - **Context:** Verify test coverage
  - **Acceptance Criteria:**
    - Every node has test file
    - Tests cover happy path
    - Tests cover error cases
    - Tests cover edge cases
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 11.3.2 Add integration tests
  - **Context:** Test node composition
  - **Acceptance Criteria:**
    - Tests for common patterns
    - Tests for complex graphs
    - All tests pass
    - `bin/pre-commit` succeeds
    - Git commit is made

- [ ] 11.3.3 Add performance benchmarks
  - **Context:** Measure node performance
  - **Acceptance Criteria:**
    - Benchmarks for critical nodes
    - Performance documented
    - Benchmarks runnable
    - `bin/pre-commit` succeeds
    - Git commit is made
