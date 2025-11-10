mod infra;

// Your tests go here!
success_tests! {
    test_input: { file: "input", input: "2", expected: "2" },
    test_input_tc: { file: "input", input: "false", expected: "false", typecheck: true },

}

runtime_error_tests! {
    test_overflow_error: { file: "overflow", expected: "overflow" },
}

static_error_tests! {
    test_parse_error: { file: "parse", input: "2", expected: "Invalid" },
}


repl_tests! {
    test_simple_bools: ["(define x true)", "x", "false"] => ["true", "false"],
    test_define_and_use: { commands: ["(define a 10)", "(define b (+ a 5))", "(+ a b)"], expected: ["25"], typecheck: true },

}
