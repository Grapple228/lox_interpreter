use std::{
    env::args,
    io::{stdin, stdout, Write},
    process::exit,
    time::Instant,
};

use interpreter::{
    common::{
        Chunk,
        OpCode::{self, OP_CONSTANT},
        Value,
    },
    vm::{InterpretResult, Vm},
    Error, Result, Stack,
};
use tracing::debug;

fn main() -> Result<()> {
    interpreter::init()?;

    // const TOTAL: usize = 5;
    // println!("Adding {TOTAL} constants...");

    // let start = Instant::now();

    // let mut chunk = Chunk::new();
    // chunk.write_constant(Value::Number(2.0), 123);

    // chunk.write_constant(Value::Number(4.0), 123);

    // chunk.write(OpCode::OP_ADD, 123);

    // chunk.write_constant(Value::Number(2.0), 123);

    // chunk.write(OpCode::OP_DIVIDE, 123);
    // chunk.write(OpCode::OP_NEGATE, 123);

    // chunk.write(OpCode::OP_RETURN, 124);

    // let end = start.elapsed();

    // println!("Chunk created per {:?}", end);

    // let start = Instant::now();
    let mut vm = Vm::new();
    // vm.interpret(&chunk);
    // let end = start.elapsed();

    // println!("Interpreted per {:?}", end);

    // debug!("Chunk: {:?}, Content: {:?}", chunk, chunk.dump_content());

    let args: Vec<String> = args().collect();

    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&mut vm, args[1].clone()),
        other => {
            println!("Usage: interpreted [path]");
            exit(64);
        }
    }
}

fn run_file(vm: &mut Vm, filename: String) -> Result<()> {
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
    let source_ptr = source_cstring.as_ptr();

    match vm.interpret_new(source_ptr as *const u8) {
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

    writeln!(stdout, "Lox interpleter in REPL mode");

    loop {
        write!(stdout, "> ");
        stdout.flush();

        let Ok(len) = stdin.read_line(&mut line) else {
            print!("\n");
            break;
        };

        let source = line.trim_end_matches('\n');
        vm.interpret_new(source.as_ptr());

        line.clear();
    }

    Ok(())
}
