use std::{
    env::args,
    io::{stdin, stdout, Write},
    process::exit,
};

use interpreter::{
    vm::{InterpretResult, ValueStack, Vm},
    Result,
};

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        interpreter::init()?;
    }

    let mut vm = Vm::new();

    let args: Vec<String> = args().collect();

    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&mut vm, &args[1]),
        _ => {
            println!("Usage: interpreted [path]");
            exit(64);
        }
    }
}

fn run_file(vm: &mut Vm, filename: &str) -> Result<()> {
    use std::fs::File;
    use std::io::Read;

    let mut file = match File::open(&filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Could not open file \"{}\".", filename);
            exit(74);
        }
    };

    let mut source = String::new();
    if let Err(_) = file.read_to_string(&mut source) {
        eprintln!("Could not read file \"{}\".", filename);
        exit(74);
    }

    let source_cstring = std::ffi::CString::new(source).unwrap();
    let source_ptr = source_cstring.as_ptr() as *const u8;

    let mut stack = ValueStack::new();
    vm.init(&mut stack);

    match vm.interpret(&mut stack, source_ptr) {
        InterpretResult::Ok => (),
        InterpretResult::CompileError => exit(65),
        InterpretResult::RuntimeError => exit(70),
    }

    Ok(())
}

fn repl(vm: &mut Vm) -> Result<()> {
    let stdin = stdin();
    let mut stdout = stdout();
    let mut line = String::new();

    _ = writeln!(stdout, "Lox interpleter in REPL mode");

    loop {
        _ = write!(stdout, "> ");
        _ = stdout.flush();

        let Ok(_) = stdin.read_line(&mut line) else {
            print!("\n");
            break;
        };

        let source_cstring = std::ffi::CString::new(line.as_bytes()).unwrap();
        let source_ptr = source_cstring.as_ptr() as *const u8;

        let mut stack = ValueStack::new();
        vm.init(&mut stack);

        match vm.interpret(&mut stack, source_ptr) {
            InterpretResult::Ok => (),
            InterpretResult::CompileError => exit(65),
            InterpretResult::RuntimeError => exit(70),
        }

        line.clear();
    }

    Ok(())
}
