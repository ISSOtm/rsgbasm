extern crate rsgbasm;
use rsgbasm::Assembler;
use rsgbasm::Diagnostic;

fn main() {
    let mut assembler = Assembler::new(&|diag| match diag {
        Diagnostic::Warning(warn) => println!("{:?}", warn),
        Diagnostic::Error(err) => println!("{}", err),
    });
    // TODO: use std::env::args

    match assembler.assemble(std::io::stdin()) {
        Ok(()) => println!("Success!"),
        Err(err) => println!("Error: {}", err),
    }
}
