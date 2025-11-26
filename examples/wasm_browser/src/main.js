import init, { calculate_sum, process_numbers, process_string } from '../pkg/wasm_browser.js';

async function main() {
    // Initialize the WASM module
    console.log('Initializing WASM module...');
    await init();
    console.log('WASM module initialized!');
    
    // Example 1: Process numbers
    console.log('\n=== Example 1: Processing Numbers ===');
    const inputNumbers = new Uint32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    const numbers = process_numbers(inputNumbers);
    console.log('Input numbers:', Array.from(inputNumbers));
    console.log('Processed numbers (doubled):', Array.from(numbers));
    
    // Display results in the DOM
    const resultsDiv = document.getElementById('results');
    if (resultsDiv) {
        resultsDiv.innerHTML = `
            <h2>Processing Results</h2>
            <h3>Numbers (doubled):</h3>
            <pre>${JSON.stringify(Array.from(numbers), null, 2)}</pre>
        `;
    }
    
    // Example 2: Process string
    console.log('\n=== Example 2: Processing String ===');
    const testString = "hello, streamweave!";
    const uppercased = process_string(testString);
    console.log('Original:', testString);
    console.log('Uppercased:', uppercased);
    
    // Example 3: Calculate sum
    console.log('\n=== Example 3: Calculating Sum ===');
    const sumNumbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const sum = calculate_sum(sumNumbers);
    console.log('Numbers:', sumNumbers);
    console.log('Sum:', sum);
    
    // Update DOM with all results
    if (resultsDiv) {
        resultsDiv.innerHTML = `
            <h2>StreamWeave WASM Processing Results</h2>
            
            <h3>1. Number Processing</h3>
            <p>Input: [${Array.from(inputNumbers).join(', ')}]</p>
            <p>Output (doubled): [${Array.from(numbers).join(', ')}]</p>
            <pre>${JSON.stringify(Array.from(numbers), null, 2)}</pre>
            
            <h3>2. String Processing</h3>
            <p>Original: "${testString}"</p>
            <p>Uppercased: "${uppercased}"</p>
            
            <h3>3. Sum Calculation</h3>
            <p>Numbers: [${sumNumbers.join(', ')}]</p>
            <p>Sum: ${sum}</p>
        `;
    }
    
    console.log('\n✅ All examples completed successfully!');
}

// Run when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', main);
} else {
    main();
}

