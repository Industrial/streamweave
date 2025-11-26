const { process_stream, process_json, calculate_stats } = require('../pkg/wasm_nodejs.js');

console.log('🚀 StreamWeave WASM Node.js Example');
console.log('====================================\n');

// Example 1: Process a stream of numbers
console.log('Example 1: Processing number stream...');
const numbers = process_stream(1, 20);
console.log('Input range: 1-19');
console.log('Processed (doubled, filtered > 10):', numbers);
console.log('');

// Example 2: Process JSON
console.log('Example 2: Processing JSON...');
const inputJson = '{"name": "test", "value": 42}';
const processedJson = process_json(inputJson);
console.log('Input:', inputJson);
console.log('Processed:', processedJson);
console.log('');

// Example 3: Calculate statistics
console.log('Example 3: Calculating statistics...');
const testNumbers = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
const stats = calculate_stats(testNumbers);
console.log('Numbers:', testNumbers);
console.log('Statistics:', stats);
console.log('');

console.log('✅ All examples completed successfully!');

