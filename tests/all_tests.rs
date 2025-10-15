mod infra;

// Your tests go here!
success_tests! {
    test_false_val: { file: "false_val", expected: "false" },
    test_input: { file: "input", input: "2", expected: "2" },

}

runtime_error_tests! {
    test_overflow: { file: "overflow", expected: "overflow" },
}

static_error_tests! {
    test_parse: { file: "parse", expected: "Invalid" },
}


repl_tests! {
    test_simple_bools: ["(define x true)", "x", "false"] => ["true", "false"],
}