mod de; // Declare the German module

use de::rules::get_default_rules;
use de::methods::IncrementalBuffer;

fn main() {
    // 1. Initialize the rules table (Live throughout the program)
    let rules = get_default_rules();

    // 2. Initialize the buffer
    let mut buffer = IncrementalBuffer::new(&rules);

    // 3. Simulate user typing: "a" then "e"
    println!("Input Key: a");
    buffer.push('a');
    
    println!("Input Next Key: e");
    buffer.push('e');
    // 4. Result
    println!("Result: {}", buffer.view()); // Print: ä

    buffer.clear();

    println!("Input Key: A");
    buffer.push('A');
    
    println!("Input Next Key: E");
    buffer.push('E');

    println!("Result: {}", buffer.view()); // Print: Ä

    buffer.clear();

    println!("Input Key: o");
    buffer.push('o');
    
    println!("Input Next Key: e");
    buffer.push('e');

    println!("Result: {}", buffer.view()); // Print: ö

    buffer.clear();

    println!("Input Key: O");
    buffer.push('O');
    
    println!("Input Next Key: E");
    buffer.push('E');

    println!("Result: {}", buffer.view()); // Print: Ö

    buffer.clear();

    println!("Input Key: u");
    buffer.push('u');
    
    println!("Input Next Key: e");
    buffer.push('e');

    println!("Result: {}", buffer.view()); // Print: ü

    buffer.clear();

    println!("Input Key: U");
    buffer.push('U');
    
    println!("Input Next Key: E");
    buffer.push('E');

    println!("Result: {}", buffer.view()); // Print: Ü

    buffer.clear();

    println!("Input Key: s");
    buffer.push('s');

    println!("Input Next Key: s");
    buffer.push('s');

    println!("Result: {}", buffer.view()); // Print: ß

    buffer.clear();

    println!("Input Key: S");
    buffer.push('S');
    
    println!("Input Next Key: s");
    buffer.push('s');

    println!("Result: {}", buffer.view()); // Print: ß 

    buffer.clear();

    println!("Input Key: S");
    buffer.push('S');
    
    println!("Input Next Key: s");
    buffer.push('S');

    println!("Result: {}", buffer.view()); // Print: ß

    buffer.clear();

}   