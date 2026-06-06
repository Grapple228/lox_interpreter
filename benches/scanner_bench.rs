// tests/performance_test.rs
use std::time::Instant;

use interpreter::scanner::{init_scanner, scan_token};

fn generate_large_file(size_mb: usize) -> String {
    let mut source = String::with_capacity(size_mb * 1024 * 1024);

    let template = r#"
// This is a comment
var x = 123.456;
var y = "string literal";
if (x > 100) {
    print("x is large");
    for (var i = 0; i < 10; i = i + 1) {
        print(i);
    }
}
while (y != nil) {
    y = nil;
}
fun fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
class Animal {
    speak() {
        print("Animal speaks");
    }
}
"#;

    while source.len() < size_mb * 1024 * 1024 {
        source.push_str(template);
        source.push('\n');
    }

    source.truncate(size_mb * 1024 * 1024);
    source
}

fn benchmark_scan(mb: usize) {
    println!("\n=== Scanning {} MB file ===", mb);

    let source = generate_large_file(mb);
    let source_cstring = std::ffi::CString::new(source).unwrap();
    let source_ptr = source_cstring.as_ptr() as *const u8;

    init_scanner(source_ptr);

    // Warmup
    for _ in 0..3 {
        loop {
            let token = scan_token();
            if token.typ == interpreter::token::TokenType::EOF {
                break;
            }
        }
    }

    // Real benchmark
    let start = Instant::now();
    let mut token_count = 0;
    init_scanner(source_ptr);

    loop {
        let token = scan_token();
        token_count += 1;
        if token.typ == interpreter::token::TokenType::EOF {
            break;
        }
    }

    let duration = start.elapsed();
    let mb_per_sec = mb as f64 / duration.as_secs_f64();
    let tokens_per_sec = token_count as f64 / duration.as_secs_f64();

    println!("  Time: {:?}", duration);
    println!("  Tokens: {}", token_count);
    println!("  Speed: {:.2} MB/s", mb_per_sec);
    println!("  Tokens/sec: {:.0}", tokens_per_sec);
}

fn benchmark_individual_tokens() {
    println!("\n=== Individual Token Performance ===");

    let test_cases = vec![
        ("ident", "hello_world_123456789"),
        ("number", "12345.6789"),
        ("string", "\"this is a long string literal\""),
        ("keyword", "while"),
        ("operator", "<=>="),
    ];

    for (name, source) in test_cases {
        let source_cstring = std::ffi::CString::new(source).unwrap();
        let source_ptr = source_cstring.as_ptr() as *const u8;

        init_scanner(source_ptr);

        // Warmup
        for _ in 0..1000 {
            scan_token();
        }

        let start = Instant::now();
        let iterations = 1_000_000;

        for _ in 0..iterations {
            init_scanner(source_ptr);
            std::hint::black_box(scan_token());
        }

        let duration = start.elapsed();
        let per_token = duration / iterations;

        println!("  {}: {:?} per token", name, per_token);
    }
}

fn main() {
    println!("Scanner Performance Test");
    println!("========================");

    // Тест на разных размерах файлов
    benchmark_scan(1); // 1 MB
    benchmark_scan(10); // 10 MB

    // Тест отдельных токенов
    benchmark_individual_tokens();
}
