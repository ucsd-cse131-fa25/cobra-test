// Add paste crate for macro identifier concatenation
use std::{
    fs::File, io::{prelude::*, BufReader, Write}, path::{Path, PathBuf}, process::{Command, Stdio}, sync::mpsc, thread, time::Duration
};

#[derive(Debug)]
pub enum SnekError {
    Aot(String),
    Jit(String),
    Run(String),
}

pub(crate) enum TestKind {
    Success,
    RuntimeError,
    StaticError,
}

#[macro_export]
macro_rules! success_tests {
    ($($tt:tt)*) => { $crate::tests!(Success => $($tt)*); }
}

#[macro_export]
macro_rules! runtime_error_tests {
    ($($tt:tt)*) => { $crate::tests!(RuntimeError => $($tt)*); }
}

#[macro_export]
macro_rules! static_error_tests {
    ($($tt:tt)*) => { $crate::tests!(StaticError => $($tt)*); }
}

#[macro_export]
macro_rules! tests {
    // Accept test cases as identifier: { file: ..., ... }
    ($kind:ident => $( $name:ident : { file: $file:literal, $(input: $input:literal,)? expected: $expected:literal $(,)? } ),* $(,)? ) => {
        $(
            #[test]
            fn $name() {
                #[allow(unused_assignments, unused_mut)]
                let mut input = None;
                $(input = Some($input);)?
                let kind = $crate::infra::TestKind::$kind;
                $crate::infra::run_test(stringify!($name), $file, input, $expected, kind);
            }
        )*
    };
}

pub(crate) fn run_test(
    name: &str,
    file: &str,
    input: Option<&str>,
    expected: &str,
    kind: TestKind,
) {
    match kind {
        TestKind::Success => run_success_test(name, file, expected, input),
        TestKind::RuntimeError => run_runtime_error_test(name, file, expected, input),
        TestKind::StaticError => run_static_error_test(name, file, expected),
    }
}

fn run_runtime_error_test(name: &str, file: &str, expected: &str, input: Option<&str>) {
    match compile(name, file, input) {
        Err(SnekError::Aot(err)) => {
            panic!("expected a successful compilation, but got an AOT error: `{}`", err);
        }
        Err(err) => {
            check_error_msg(&err, expected);
            return;
        }
        Ok((out1, out2)) => {
            panic!("expected a runtime failure, but program succeeded: `{}` `{}`", out1, out2);
        }
    }
}

fn run_static_error_test(name: &str, file: &str, expected: &str) {
    match compile(name, file, None) {
        Ok((e1,e2)) => panic!("expected a failure, but compilation succeeded"),
        Err(err) => check_error_msg(&err, expected),

    }
}

fn check_error_msg(found: &SnekError, expected: &str) {
    match found {
        SnekError::Aot(err) => assert!( err.contains(expected.trim()), "Compile error message does not match {}", err),
        SnekError::Jit(err) => assert!( err.contains(expected.trim()), "JIT error message does not match {}", err),
        SnekError::Run(err) => assert!( err.contains(expected.trim()), "AOT runtime error message does not match {}", err),
    }
}

fn mk_path(name: &str, ext: Ext) -> PathBuf {
    Path::new("tests").join(format!("{name}.{ext}"))
}

#[derive(Copy, Clone)]
enum Ext {
    Snek,
    Asm,
    Run,
}


impl std::fmt::Display for Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ext::Snek => write!(f, "snek"),
            Ext::Asm => write!(f, "s"),
            Ext::Run => write!(f, "run"),
        }
    }
}

fn compile(name: &str, file: &str, input: Option<&str>) -> Result<(String, String), SnekError> {
    // Run the compiler
    let boa_path = if cfg!(target_os = "macos") {
        PathBuf::from("target/x86_64-apple-darwin/debug/cobra")
    } else {
        PathBuf::from("target/debug/cobra")
    };
    let output_c = Command::new(&boa_path)
        .arg("-c")
        .arg(&mk_path(file, Ext::Snek))
        .arg(&mk_path(name, Ext::Asm))
        .output()
        .expect("could not run the compiler");
    if !output_c.status.success() {
        return Err(SnekError::Aot(String::from_utf8(output_c.stderr).unwrap()));
    }

    let mut cmd_e = Command::new(&boa_path);
    cmd_e.arg("-e").arg(&mk_path(file, Ext::Snek));
    if let Some(inp) = input {
        cmd_e.arg(inp);
    }
    let output_e = cmd_e.output().expect("could not run the compiler");
    if !output_e.status.success() {
        return Err(SnekError::Jit(String::from_utf8(output_e.stderr).unwrap()));
    }
    let jit_stdout = String::from_utf8(output_e.stdout).unwrap();

    eprintln!("JIT result: {}", jit_stdout);

    // Assemble and link
    let output = Command::new("make")
        .arg(&mk_path(name, Ext::Run))
        .output()
        .expect("could not run make");
    assert!(output.status.success(), "linking failed");

    // Run produced program and capture stdout
    let output_run = run(name, input)
        .map_err(|e| SnekError::Run(e))?
        .into_bytes();
    let run_stdout = String::from_utf8(output_run).unwrap();

    Ok((jit_stdout, run_stdout))
}


fn run(name: &str, input: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new(&mk_path(name, Ext::Run));
    if let Some(input) = input {
        cmd.arg(input);
    }
    let output = cmd.output().unwrap();
    if output.status.success() {
        Ok(String::from_utf8(output.stdout).unwrap().trim().to_string())
    } else {
        Err(String::from_utf8(output.stderr).unwrap().trim().to_string())
    }
}


pub(crate) fn run_success_test(name: &str, file: &str, expected: &str, input: Option<&str>) {
    let (jit_out, run_out) = match compile(name, file, input) {
        Ok((jit, run)) => (jit, run),
        Err(SnekError::Aot(err)) => panic!("expected a successful compilation, but got an AOT error: `{}`", err),
        Err(SnekError::Jit(err)) => panic!("expected a successful compilation, but got a JIT error: `{}`", err),
        Err(SnekError::Run(err)) => panic!("expected a successful run, but got a runtime error: `{}`", err),
    };

    let expected_trim = expected.trim();

    let jit_trim = jit_out.trim();
    let run_trim = run_out.trim();

    let mut failed_flags = Vec::new();

    if expected_trim != jit_trim {
        failed_flags.push(("-e", jit_trim.to_string(), jit_out));
    }
    if expected_trim != run_trim {
        failed_flags.push(("-c", run_trim.to_string(), run_out));
    }

    if !failed_flags.is_empty() {
        for (flag, actual_trim, raw) in &failed_flags {
            eprintln!("Flag {} unexpected output:\n{}", flag, prettydiff::diff_lines(raw, expected_trim));
        }
        panic!("test failed: outputs did not match expected value for flags: {:?}", failed_flags.iter().map(|(f,_,_)| *f).collect::<Vec<_>>());
    }
}


#[macro_export]
macro_rules! repl_tests {
    ($($name:ident: [$($command:literal),*] => [$($expected:literal),*]),* $(,)?) => {
        $(
        #[test]
        fn $name() {
            let commands = vec![$($command),*];
            let expected_outputs = vec![$($expected),*];
            $crate::infra::run_repl_sequence_test(stringify!($name), &commands, &expected_outputs);
        }
        )*
    }
}

pub(crate) fn run_repl_sequence_test(name: &str, commands: &[&str], expected_outputs: &[&str]) {
    let actual_outputs = run_repl_with_timeout(commands, 3000);

    let mut current_pos = 0;
    let mut found_outputs = Vec::new();
    
    for expected in expected_outputs {
        let expected_subs: Vec<&str> = expected.split(',').map(|s| s.trim()).collect();
        
        // Linear scan
        let remaining = &actual_outputs[current_pos..];
        let mut search_pos = 0;
        let mut match_start = None;
        let mut match_end = None;
        
        let mut all_found = true;
        for (i, sub) in expected_subs.iter().enumerate() {
            if let Some(pos) = remaining[search_pos..].find(sub) {
                let absolute_pos = search_pos + pos;
                if i == 0 {
                    match_start = Some(absolute_pos);
                }
                search_pos = absolute_pos + sub.len();
                if i == expected_subs.len() - 1 {
                    match_end = Some(search_pos);
                }
            } else {
                all_found = false;
                break;
            }
        }
        
        if all_found {
            if let (Some(start), Some(end)) = (match_start, match_end) {
                let matched_content = remaining[start..end].trim().to_string();
                found_outputs.push(matched_content);
                current_pos = current_pos + end;
            } else {
                eprintln!("[repl_test] Internal error extracting match for {:?}\nFull output:\n{}", expected_subs, actual_outputs);
                panic!("Test '{}' failed: internal error extracting match", name);
            }
        } else {
            let expected_str = format!("{:?}", expected_outputs);
            let actual_str = format!("{:?}", found_outputs);
            let expected_joined = expected_outputs.join("\n");
            let actual_joined = found_outputs.join("\n");
            eprintln!("\n[repl_test] MISMATCH\nExpected vector: {}\nActual vector:{}\n\nString diff:\n{}\n\nFull output:\n{}\n",
                expected_str,
                actual_str,
                prettydiff::diff_lines(&actual_joined, &expected_joined),
                actual_outputs
            );
            panic!("Test '{}' failed: expected substrings {:?} not found in order in output", name, expected_subs);
        }
    }
    println!("[repl_test] Success!\nExpected vector: {:?}\nActual vector:   {:?}\n", expected_outputs, found_outputs);
}





fn run_repl_with_timeout(commands: &[&str], timeout_ms: u64) -> String {
    // Probably dont need this for autograder
    let boa_path = if cfg!(target_os = "macos") {
        "target/x86_64-apple-darwin/debug/cobra"
    } else {
        "target/debug/cobra"
    };

    let mut child = Command::new(boa_path)
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start repl");

    {
        let stdin = child.stdin.as_mut().expect("failed to open stdin");
        
        for command in commands {
            writeln!(stdin, "{}", command).unwrap();
            stdin.flush().unwrap();
            thread::sleep(Duration::from_millis(100));
        }
        
        // kill REPL
        writeln!(stdin, "").unwrap();
        stdin.flush().unwrap();
    }
    
    // Processing
    thread::sleep(Duration::from_millis(timeout_ms));
    
    let _ = child.kill();
    
    let output = child.wait_with_output().expect("failed to read output");
    String::from_utf8_lossy(&output.stdout).to_string()
}

