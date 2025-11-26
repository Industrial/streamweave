import init, { process_numbers, process_string, calculate_sum } from '../pkg/wasm_browser.js';

async function main() {
    // Initialize the WASM module
    console.log('Initializing WASM module...');
    await init();
    console.log('WASM module initialized!');
    
    // Example 1: Process numbers
    console.log('\n=== Example 1: Processing Numbers ===');
    const numbers = process_numbers(1, 15);
    console.log('Processed numbers:', numbers);
    
    // Display results in the DOM
    const resultsDiv = document.getElementById('results');
    if (resultsDiv) {
        resultsDiv.innerHTML = `
            <h2>Processing Results</h2>
            <h3>Numbers (1-14, doubled, filtered > 10):</h3>
            <pre>${JSON.stringify(numbers, null, 2)}</pre>
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
            <p>Processed numbers from 1 to 14:</p>
            <pre>${JSON.stringify(numbers, null, 2)}</pre>
            
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

