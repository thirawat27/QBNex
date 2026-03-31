// Centralized text fixture catalog for conformance and release validation.
// Generated from the former .in/.out fixture files so the workspace can stay free of those extensions.

#[allow(dead_code)]
pub fn conformance_expected_output(stem: &str) -> Option<&'static str> {
    match stem {
        "arrays_and_bounds" => Some("1\n4\n5\n10\n15\n99\n"),
        "byval_byref" => Some("6\n6\n8\n8\n11\n8\n"),
        "clear_freefile" => Some("[]\n1\n"),
        "common_shared" => Some("8\n"),
        "common_shared_include" => Some("9\n"),
        "computed_branching" => Some("G1\nG2\nG3\nTARGET\n"),
        "console_input" => Some("Alice\n42\nHello from LINE INPUT\n"),
        "const_and_def_fn" => Some("12.56\n28.26\n"),
        "control_flow" => Some("flow\nselect\n4\n"),
        "data_restore" => Some("1\n2\n3\n4\n"),
        "def_type_coercion" => Some("2\n100000\n"),
        "erase_redim" => Some("1\n2\n0\n0\n"),
        "fixed_length_lset_rset" => Some("[AB  ]\n[   Z]\n"),
        "graphics_modes" => Some("1\n1\n2\n3\n1\n4\n5\n6\n3\n1\n2\n7\n"),
        "graphics_view_window_pmap" => Some("12\n60\n60\n0\n0\n9\n"),
        "logical_comparisons" => Some("AND\nOR\nNOT\nXOR\nEQV\nIMP\nNE\nLE\nGE\n"),
        "loop_controls" => Some("9\n3\n12\n"),
        "mid_assignment" => Some("AxyzE\nAxyz!\n"),
        "numeric_operators" => Some("4\n2\n-3\n-4\n3\n1\n"),
        "on_error" => Some("5\n20\n"),
        "on_play_event" => Some("<BEL>1\n0\n"),
        "on_timer_event" => Some("1\n"),
        "print_using" => Some(" 0.78\nTotal,  12,\nItem: A Qty:  1 Item: B Qty:  2 \n"),
        "procedures_and_def_fn" => Some("25\n30\n10\n"),
        "random_field_io" => Some("XY\n"),
        "screen_text_state" => Some("AAAAAAAAAA\n\n1\n2\n49,50,2\n"),
        "select_case_advanced" => Some("big\n"),
        "sequential_file_io" => Some("line-one\nline-two\ndone\n"),
        "shared_globals" => Some("3\n"),
        "static_state" => Some("1\n2\n3\n1\n2\n3\n"),
        "string_intrinsics" => Some("HE\nLO\nELL\n3\n65\nQBNex\n"),
        "swap_and_clear" => Some("2\n1\n[]\n"),
        "system_memory_io" => Some("123\n123\n77\n"),
        "type_conversions" => Some("4\n4\n42\n321\n123456\n1.5\n2.5\n16\n[ 7]\n"),
        "user_defined_types" => Some("10\n20\n30\n"),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn conformance_input(stem: &str) -> Option<&'static str> {
    match stem {
        "console_input" => Some("Alice\n42\nHello from LINE INPUT\n"),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn release_expected_output(fixture_name: &str) -> Option<&'static str> {
    match fixture_name {
        "release_validation_file_io.bas" => Some("QBNex-123\n"),
        "release_validation_text.bas" => Some("QBNex\n60\n"),
        "release_validation_vm_fallback.bas" => Some("recovered\n"),
        _ => None,
    }
}
