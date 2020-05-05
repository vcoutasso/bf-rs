use std::fs::File;
use std::io::{self, Read, Write};
use std::num::Wrapping;

use Instructions::*;

#[derive(Debug, PartialEq, Clone)]
// The tuple enum variants hold a value that represents how many times the instruction should be repeated. This overcomes the overhead of repeating the same task over and over in the form of 'unit operations'
pub enum Instructions {
    IncrementPointer(usize), // Next pointer
    DecrementPointer(usize), // Previous pointer
    IncrementValue(usize),   // value++
    DecrementValue(usize),   // value--
    BeginLoop,               // Loop start
    EndLoop,                 // Loop end
    ReadChar,                // Read char from stdin
    PrintChar,               // Print value as char to stdout
    // Instructions below this comment do not belong to bf and are here for optimization purposes
    SetZero, // Equivalent to [-] (set current cell to 0), but in one instruction
}

// Translates the code from a string of chars to a Vec of Instructions to be later matched against properly in run(). Returns a vector with the instructions in the order that they appear, but with some optimizations
pub fn parse(program: &str, opt_level: i32) -> Vec<Instructions> {
    // Raw instructions extracted from program
    let mut instructions: Vec<Instructions> = vec![];

    // Extract original instructions
    for ch in program.trim().chars() {
        match ch {
            '>' => instructions.push(IncrementPointer(1)),
            '<' => instructions.push(DecrementPointer(1)),
            '+' => instructions.push(IncrementValue(1)),
            '-' => instructions.push(DecrementValue(1)),
            '[' => instructions.push(BeginLoop),
            ']' => instructions.push(EndLoop),
            ',' => instructions.push(ReadChar),
            '.' => instructions.push(PrintChar),
            // Everything else is regarded as a comment
            _ => continue,
        }
    }

    if opt_level > 0 {
        // Replaces all the occurrences of set_zero for the equivalent and more efficient Instruction::SetZero
        let set_zero = [BeginLoop, DecrementValue(1), EndLoop];

        // This vector contains all instructions in their optimized form (grouped)
        let mut optimized: Vec<Instructions> = vec![];
        // This slice represents the enum variants that can be grouped together
        let groupable = [
            IncrementPointer(1),
            DecrementPointer(1),
            IncrementValue(1),
            DecrementValue(1),
        ];
        // Counter
        let mut i = 0;

        // Keep track of how many repeated groupable instructions are close together for later simplification
        // e.g. ++ => IncrementValue(2)
        while i < instructions.len() {
            let mut acc = 1;

            // If groupable, create an equivalent instruction
            if groupable.contains(&instructions[i]) {
                while Some(&instructions[i]) == instructions[i + acc..].iter().next() {
                    acc += 1;
                }
            }
            // Check if the next 3 instructions are equivalent to SetZero and if so, use it instead
            else if opt_level > 1
            && instructions[i] == BeginLoop
            && i + set_zero.len() < instructions.len() // If the slice is not out of bounds
            && instructions[i..i + set_zero.len()] == set_zero
            // Check if it is equivalent to SetZero
            {
                optimized.push(SetZero);
                i += set_zero.len(); // Skip instructions we don't need anymore
                continue; // All done here, go to next
            }

            // Doesn't look very pretty, but it gets the job done
            match instructions[i] {
                IncrementPointer(_) => optimized.push(IncrementPointer(acc)),
                DecrementPointer(_) => optimized.push(DecrementPointer(acc)),
                IncrementValue(_) => optimized.push(IncrementValue(acc)),
                DecrementValue(_) => optimized.push(DecrementValue(acc)),
                _ => optimized.push(instructions[i].clone()),
            }
            i += acc;
        }
        optimized
    } else {
        instructions
    }
}

// Here's where the magic happens. With the course of action extracted with the parse() function, the only thing that is left to do is to take the appropriate action given an instruction
// Returns the number of executed instructions and the index the pointer points at
pub fn run(inst: &[Instructions], data: &mut [Wrapping<u8>], mut idx: usize) -> (usize, usize) {
    // Variable to keep track of how many instructions were performed
    let mut actions: usize = 0;
    // Counter
    let mut i = 0;

    // Indexes of begin loops to keep track of nested loops. Only used to fill jump
    let mut stack: Vec<usize> = Vec::new();
    // Vec with indexes of jumps to be made during execution (loops)
    let mut jump: Vec<usize> = vec![0; inst.len()];

    // This takes care of nested loops and how the interpreter should deal to them. jump will be filled with the indexes to perform the appropriate jumps at appropriate times
    for i in 0..inst.len() {
        match inst[i] {
            BeginLoop => stack.push(i),
            EndLoop => {
                let index = stack.pop().expect("Could not find matching '['"); // Index of most recent loop
                jump[i] = index; // When code reaches the ith instruction, go to index and continue from there
                jump[index] = i; // When index is reached, go back to the start of the loop
            }
            _ => continue,
        }
    }

    // Loop through all intructions
    while i < inst.len() {
        match inst[i] {
            // If idx is equal to the last position, return to the first
            IncrementPointer(qty) => {
                idx += qty;
                idx %= data.len();
            }
            // If idx is equal to the first position, go to the last
            DecrementPointer(qty) => {
                if qty > idx {
                    idx = data.len() - (qty - idx);
                } else {
                    idx -= qty;
                }
            }
            IncrementValue(qty) => {
                data[idx] += Wrapping(qty as u8);
            }
            DecrementValue(qty) => {
                data[idx] -= Wrapping(qty as u8);
            }
            BeginLoop => {
                if data[idx] == Wrapping(0) {
                    i = jump[i];
                }
            }
            EndLoop => {
                if data[idx] != Wrapping(0) {
                    i = jump[i];
                }
            }
            ReadChar => match io::stdin().bytes().next() {
                Some(res) => {
                    if let Ok(value) = res {
                        data[idx] = Wrapping(value)
                    }
                }
                None => eprintln!("Could not read from stdin"),
            },
            PrintChar => print!("{}", char::from(data[idx].0)),
            SetZero => data[idx] = Wrapping(0),
        }
        actions += 1;
        i += 1;
    }
    (actions, idx)
}

// Dump memory to file
pub fn dump_mem(memory: &[Wrapping<u8>], filename: &str, addr: usize) -> io::Result<()> {
    let mut f = File::create(filename)?;
    let step = 12;

    f.write_all(format!("Pointer pointing at address 0x{:04X}\n\n", addr).as_bytes())?;

    for i in (0..memory.len()).step_by(step) {
        f.write_all(format!("0x{:04X}: \t", i).as_bytes())?;

        for value in memory.iter().skip(i).take(step) {
            f.write_all(format!("0x{:02X} \t", value).as_bytes())?;
        }

        // Format last line properly if it is shorter than the previous ones
        if i + step > memory.len() {
            for _j in 0..(step - (memory.len()%step)) {
                f.write_all(b"\t")?;
            }
        }

        for value in memory.iter().skip(i).take(step) {
            if value.0.is_ascii_graphic() {
                f.write_all(format!("{}", value.0 as char).as_bytes())?;
            } else {
                f.write_all(b".")?;
            }
        }

        f.write_all(b"\n")?;
    }

    Ok(())
}

// Dump instructions to file
pub fn dump_inst(instructions: &[Instructions], filename: &str) -> io::Result<()> {
    let mut f = File::create(filename)?;

    f.write_all(format!("{:#?}", instructions).as_bytes())?;

    Ok(())
}
