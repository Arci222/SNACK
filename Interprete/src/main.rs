use std::{env, fs, process};

use lalrpop_util::lalrpop_mod;

mod ast;

lalrpop_mod!(pub parser);

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Uso: Interprete <file.spln>");
        process::exit(2);
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("Impossibile leggere '{path}': {error}");
        process::exit(2);
    });

    let program = parser::ProgramParser::new()
        .parse(&source)
        .unwrap_or_else(|error| {
            eprintln!("Errore di sintassi in '{path}': {error}");
            process::exit(1);
        });

    let output = program.run().unwrap_or_else(|error| {
        eprintln!("Errore semantico in '{path}': {error}");
        process::exit(1);
    });

    for line in output {
        println!("{line}");
    }
}
