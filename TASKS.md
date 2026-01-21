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

- [x] 1. Fix Node Examples
  - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
  - Acceptance Criteria:
    - Example created in examples/nodes/ using Graph API (no direct node execution)
    - Example added to Cargo.toml [[example]] section
    - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
    - git commit made for each completed example (you may not use `--no-verify`).
  
  - [x] 1.1 BreakNode (examples/nodes/break_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.2 ConditionNode (examples/nodes/condition_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.3 ContinueNode (examples/nodes/continue_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.4 ErrorBranchNode (examples/nodes/error_branch_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.5 JoinNode (examples/nodes/join_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.6 RangeNode (examples/nodes/range_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.7 ReadVariableNode (examples/nodes/read_variable_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 1.8 SyncNode (examples/nodes/sync_node.rs)
    - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
    - Acceptance Criteria:
      - Example must ALWAYS use graph API to explain solution.
      - Example may NEVER use node without graph.
      - Example added to Cargo.toml [[example]] section
      - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
      - git commit made for each completed example (you may not use `--no-verify`).

- [x] 2. Create Node Examples
  - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
  - Acceptance Criteria:
    - Example created in examples/nodes/ using Graph API (no direct node execution)
    - Example added to Cargo.toml [[example]] section
    - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
    - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.1 Advanced Control Flow Nodes
    - [x] 2.1.1 break_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.1.2 continue_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.1.3 repeat_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.1.4 retry_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.1.5 switch_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.1.6 try_catch_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.2 Aggregation Nodes
    - [x] 2.2.1 average_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.2.2 count_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.2.3 max_aggregate_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.2.4 min_aggregate_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.2.5 sum_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.3 Arithmetic Nodes
    - [x] 2.3.1 add_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.3.2 divide_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.3.3 modulo_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.3.4 multiply_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.3.5 power_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.3.6 subtract_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.4 Array Nodes
    - [x] 2.4.1 concat_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.2 contains_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.3 flatten_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.4 index_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.5 index_of_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.6 length_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.7 reverse_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.8 slice_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.9 sort_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.10 split_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.4.11 unique_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.5 Boolean Logic Nodes
    - [x] 2.5.1 and_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.5.2 nand_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.5.3 nor_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.5.4 not_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.5.5 or_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.5.6 xor_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.6 Comparison Nodes
    - [x] 2.6.1 equal_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.6.2 greater_than_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.6.3 greater_than_or_equal_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.6.4 less_than_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.6.5 less_than_or_equal_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.6.6 not_equal_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [x] 2.7 Math Nodes
    - [x] 2.7.1 abs_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.2 ceil_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.3 exp_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.4 floor_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.5 log_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.6 max_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.7 min_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.8 round_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.7.9 sqrt_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.8 Object Nodes
    - [x] 2.8.1 get_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.8.2 has_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [x] 2.8.3 keys_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [o] 2.8.4 merge_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.8.5 set_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.8.6 size_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.8.7 values_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.9 Reduction Nodes
    - [ ] 2.9.1 fold_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.9.2 reduce_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.9.3 scan_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.10 Stream Nodes
    - [ ] 2.10.1 buffer_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.2 debounce_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.3 distinct_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.4 distinct_until_changed_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.5 drop_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.6 filter_map_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.7 first_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.8 last_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.9 merge_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.10 sample_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.11 skip_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.12 take_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.13 throttle_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.14 window_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.10.15 zip_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.11 String Nodes
    - [ ] 2.11.1 append_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.2 capitalize_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.3 char_at_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.4 concat_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.5 contains_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.6 ends_with_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.7 format_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.8 index_of_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.9 last_index_of_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.10 length_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.11 lowercase_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.12 prepend_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.13 replace_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.14 split_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.15 starts_with_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.16 substring_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.17 trim_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.11.18 uppercase_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.12 Time Nodes
    - [ ] 2.12.1 current_time_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.12.2 format_time_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.12.3 parse_time_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.12.4 timestamp_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

  - [ ] 2.13 Type Operations Nodes
    - [ ] 2.13.1 as_bool_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.2 as_float_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.3 as_int_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.4 as_string_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.5 is_array_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.6 is_bool_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.7 is_float_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.8 is_int_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.9 is_null_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.10 is_object_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.11 is_string_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).

    - [ ] 2.13.12 to_json_node
      - Context: Many nodes in src/nodes/ do not have example programs demonstrating their usage. Each example should use the Graph API and no node may be used by itself.
      - Acceptance Criteria:
        - Example created in examples/nodes/ using Graph API (no direct node execution)
        - Example added to Cargo.toml [[example]] section
        - /test-example runs successfully (time timeout 10 cargo run --example <example_name>)
        - git commit made for each completed example (you may not use `--no-verify`).
