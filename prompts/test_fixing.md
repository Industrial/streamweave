## MCP Server Usage
You have access to the following MCP servers to enhance your test fixing capabilities:
- **git**: For version control operations, test failure tracking, and change analysis
- **serena**: For advanced code analysis, test failure identification, and code debugging
- **think**: For complex debugging reasoning, test strategy planning, and failure analysis
- **context7**: For accessing up-to-date testing documentation, debugging tools, and best practices
- **memory**: For storing and retrieving test failure patterns, debugging strategies, and project insights

Use these tools strategically to provide more comprehensive and accurate test fixing analysis.

## Test Fixing Process
1. Run `bin/test`.
2. Are there failures or errors?
  2.1. If so:
    2.1.1: Take the FIRST failing test.
    2.1.1: Understand what changed in the implementation. Provide 100% covering tests of all permutations of the file tested.
  2.2. If not, go to step 3.
3. End program.