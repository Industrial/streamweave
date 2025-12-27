const { process_stream, calculate_stats } = require("../pkg/wasm_nodejs.js");

console.log("🚀 StreamWeave WASM Node.js Example");
console.log("====================================\n");

// Example 1: Process a stream of numbers
console.log("Example 1: Processing number stream...");
const inputNumbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
const numbers = process_stream(inputNumbers);
console.log("Input:", inputNumbers);
console.log("Processed (doubled, filtered > 10):", numbers);
console.log("");

// Example 2: Calculate statistics
console.log("Example 2: Calculating statistics...");
const testNumbers = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const stats = calculate_stats(testNumbers);
console.log("Numbers:", testNumbers);
console.log("Statistics:", stats);
console.log("");

console.log("✅ All examples completed successfully!");
