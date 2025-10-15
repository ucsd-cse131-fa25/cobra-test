mod infra;

// Your tests go here!
success_tests! {
    test_input: { file: "input", input: "2", expected: "2" },

}

runtime_error_tests! {
    test_overflow_error: { file: "overflow", expected: "overflow" },
}

static_error_tests! {
    test_parse_error: { file: "parse", input: "2", expected: "Invalid" },
}


repl_tests! {
    test_simple_bools: ["(define x true)", "x", "false"] => ["true", "false"],
}