# LOOP.md
## LLM Execution Protocol for TODO Tasks

You are an autonomous software development agent executing TODO items defined in
`TODO.md`.

Your objective is to **fully implement each task end-to-end**, with clean,
test-covered code and a successful commit. Follow these steps in order for each
unchecked TODO item.

---

### 🟡 Step 1: Interpret the TODO and the git state.

**Goal**: Fully understand what the TODO requires before you code.

- Restate the TODO in your own words.
- Identify the user-facing change (if applicable).
- List key modules, files, or functions that will likely be affected.
- If the TODO is vague, infer reasonable scope.

---

### 📝 Step 2: Create an Implementation Plan

**Goal**: Break the task into exact coding actions.

For example:
- "Add a new helper function `convertCurrency()` in `utils/currency.ts`."
- "Modify the `Checkout` class in `checkout.ts` to call it."

Plan should:
- Be bullet-pointed.
- Include filenames and function/class names.
- Specify if any new files/tests are needed.

---

### 🔨 Step 3: Implement the Feature or Fix

**Goal**: Make clean, minimal, correct code changes.

- Navigate to each file from the plan and apply the changes.
- You can refactor existing code if necessary, but prefer the smallest diff that solves the problem.
- Include inline comments if complex logic is introduced.
- Don't generate tests in this step yet. Only the implementation code.

---

### ✅ Step 4: Write Exhaustive Tests

**Goal**: Guarantee correctness by covering all meaningful cases.

- Write tests in the module itself.
- For the new feature or fix, write test cases that:
  - Cover **all permutations** of input conditions.
  - Include **edge cases** and **failure modes**.
  - Confirm **happy path** success behavior.
- Use the Proptest framework for testing axioms and verifying theorems.

If mocking or stubbing is required, apply appropriate technique per the project's test framework.

---

### 🧪 Step 5: Run the Test Suite

**Goal**: Validate that all tests pass and nothing is broken.

- Run the test suite (e.g. `npm test`, `pytest`, etc.).
- If any tests fail, analyze the failure output.
- Return to Step 3 or Step 4 as needed until all tests pass.

---

### 🧹 Step 6: Self-Review & Clean Up

**Goal**: Ensure that the changes meet quality standards.

Ask yourself:
- Is the code idiomatic and consistent with the rest of the project?
- Are function names, types, and interfaces well-designed?
- Could a reviewer understand this without asking questions?

Apply final cleanups:
- Remove dead code, debug prints, or TODO comments.
- Format the code if needed.

---

### 📚 Step 8: Document the Changes

**Goal**: Ensure the changes are well-documented for future maintenance.

- Generate documentation for the Traits and Types. The Implementations don't get documentation.
- If this is a significant feature, consider updating the README or user guides
- Note any follow-up work that should be captured as new TODOs

### 🟢 Step 7: Commit the Code

**Goal**: Finalize the work, leave the project in a clean and correct state.

1. **Create a Git commit** with a clear message:
   - Start with the TODO item name
   - Include a brief description of what was implemented/fixed
   - Reference any related issues

2. **Mark the TODO as complete** in `TODO.md`:
   - Change `[ ]` to `[x]` for the completed item
   - Add a completion date if applicable

---
