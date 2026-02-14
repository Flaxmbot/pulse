use std::collections::HashMap;
use pulse_core::{Value, NativeFn};
use pulse_core::object::Object;
use pulse_vm::VM;

pub fn load_std_module(name: &str, vm: &mut VM) -> Option<pulse_core::object::ObjHandle> {
    let mut exports = HashMap::new();
    
    match name {
        "std/math" => {
            // Need to implement math natives or reuse existing ones
            // For now, let's just add abs from utils
            add_native("abs", pulse_stdlib::utils::abs_native, &mut exports, vm);
        }
        "std/io" => {
            add_native_async("read", pulse_stdlib::io::read_file_native, &mut exports, vm);
            add_native_async("write", pulse_stdlib::io::write_file_native, &mut exports, vm);
            add_native_async("exists", pulse_stdlib::io::file_exists_native, &mut exports, vm);
            add_native_async("delete", pulse_stdlib::io::delete_file_native, &mut exports, vm);
        }
        "std/net" => {
             add_native_async("tcp_connect", pulse_stdlib::networking::tcp_connect_native, &mut exports, vm);
             add_native_async("tcp_listen", pulse_stdlib::networking::tcp_listen_native, &mut exports, vm);
             add_native_async("tcp_accept", pulse_stdlib::networking::tcp_accept_native, &mut exports, vm);
             add_native_async("tcp_send", pulse_stdlib::networking::tcp_send_native, &mut exports, vm);
             add_native_async("tcp_receive", pulse_stdlib::networking::tcp_receive_native, &mut exports, vm);
             add_native_async("udp_bind", pulse_stdlib::networking::socket_create_native, &mut exports, vm); // socket_create is bind?
             add_native_async("dns_resolve", pulse_stdlib::networking::dns_resolve_native, &mut exports, vm);
        }
        "std/http" => {
             add_native_async("get", pulse_stdlib::networking::http_get_native, &mut exports, vm);
             add_native_async("post", pulse_stdlib::networking::http_post_native, &mut exports, vm);
        }
        "std/json" => {
            add_native("parse", pulse_stdlib::json::json_parse_native, &mut exports, vm);
            add_native("stringify", pulse_stdlib::json::json_stringify_native, &mut exports, vm);
        }
        "std/pandas" => {
            // DataFrame creation
            add_native("df_create", pulse_stdlib::pandas::df_create_native, &mut exports, vm);
            add_native("df_from_list", pulse_stdlib::pandas::df_from_list_native, &mut exports, vm);
            add_native("df_from_csv", pulse_stdlib::pandas::df_from_csv_native, &mut exports, vm);
            add_native("df_from_json", pulse_stdlib::pandas::df_from_json_native, &mut exports, vm);
            
            // DataFrame operations
            add_native("df_columns", pulse_stdlib::pandas::df_columns_native, &mut exports, vm);
            add_native("df_shape", pulse_stdlib::pandas::df_shape_native, &mut exports, vm);
            add_native("df_head", pulse_stdlib::pandas::df_head_native, &mut exports, vm);
            add_native("df_tail", pulse_stdlib::pandas::df_tail_native, &mut exports, vm);
            add_native("df_select", pulse_stdlib::pandas::df_select_native, &mut exports, vm);
            add_native("df_filter", pulse_stdlib::pandas::df_filter_native, &mut exports, vm);
            add_native("df_sort", pulse_stdlib::pandas::df_sort_native, &mut exports, vm);
            
            // Data operations
            add_native("df_group_by", pulse_stdlib::pandas::df_group_by_native, &mut exports, vm);
            add_native("df_aggregate", pulse_stdlib::pandas::df_aggregate_native, &mut exports, vm);
            add_native("df_join", pulse_stdlib::pandas::df_join_native, &mut exports, vm);
            add_native("df_concat", pulse_stdlib::pandas::df_concat_native, &mut exports, vm);
            
            // Column operations
            add_native("df_add_column", pulse_stdlib::pandas::df_add_column_native, &mut exports, vm);
            add_native("df_drop_column", pulse_stdlib::pandas::df_drop_column_native, &mut exports, vm);
            add_native("df_rename", pulse_stdlib::pandas::df_rename_native, &mut exports, vm);
            
            // Statistics
            add_native("df_describe", pulse_stdlib::pandas::df_describe_native, &mut exports, vm);
            add_native("df_corr", pulse_stdlib::pandas::df_corr_native, &mut exports, vm);
        }
        "std/linalg" => {
            // Vector operations
            add_native("vector_dot", pulse_stdlib::linalg::vector_dot_native, &mut exports, vm);
            add_native("vector_cross", pulse_stdlib::linalg::vector_cross_native, &mut exports, vm);
            add_native("vector_normalize", pulse_stdlib::linalg::vector_normalize_native, &mut exports, vm);
            add_native("vector_magnitude", pulse_stdlib::linalg::vector_magnitude_native, &mut exports, vm);
            
            // Matrix operations
            add_native("matrix_multiply", pulse_stdlib::linalg::matrix_multiply_native, &mut exports, vm);
            add_native("matrix_transpose", pulse_stdlib::linalg::matrix_transpose_native, &mut exports, vm);
            add_native("matrix_inverse", pulse_stdlib::linalg::matrix_inverse_native, &mut exports, vm);
            add_native("matrix_determinant", pulse_stdlib::linalg::matrix_determinant_native, &mut exports, vm);
            
            // Matrix decomposition
            add_native("matrix_lu", pulse_stdlib::linalg::matrix_lu_native, &mut exports, vm);
            add_native("matrix_qr", pulse_stdlib::linalg::matrix_qr_native, &mut exports, vm);
            add_native("matrix_svd", pulse_stdlib::linalg::matrix_svd_native, &mut exports, vm);
            
            // Linear system
            add_native("solve_linear", pulse_stdlib::linalg::solve_linear_native, &mut exports, vm);
            
            // Matrix creation
            add_native("matrix_identity", pulse_stdlib::linalg::matrix_identity_native, &mut exports, vm);
            add_native("matrix_zeros", pulse_stdlib::linalg::matrix_zeros_native, &mut exports, vm);
            add_native("matrix_ones", pulse_stdlib::linalg::matrix_ones_native, &mut exports, vm);
        }
        "std/stats" => {
            // Descriptive statistics
            add_native("mean", pulse_stdlib::stats::mean_native, &mut exports, vm);
            add_native("median", pulse_stdlib::stats::median_native, &mut exports, vm);
            add_native("mode", pulse_stdlib::stats::mode_native, &mut exports, vm);
            add_native("std", pulse_stdlib::stats::std_native, &mut exports, vm);
            add_native("variance", pulse_stdlib::stats::variance_native, &mut exports, vm);
            add_native("min", pulse_stdlib::stats::min_native, &mut exports, vm);
            add_native("max", pulse_stdlib::stats::max_native, &mut exports, vm);
            add_native("describe", pulse_stdlib::stats::describe_native, &mut exports, vm);
            
            // Probability distributions
            add_native("normal_pdf", pulse_stdlib::stats::normal_pdf_native, &mut exports, vm);
            add_native("normal_cdf", pulse_stdlib::stats::normal_cdf_native, &mut exports, vm);
            add_native("normal_sample", pulse_stdlib::stats::normal_sample_native, &mut exports, vm);
            add_native("binomial_pmf", pulse_stdlib::stats::binomial_pmf_native, &mut exports, vm);
            add_native("poisson_pmf", pulse_stdlib::stats::poisson_pmf_native, &mut exports, vm);
            
            // Hypothesis testing
            add_native("ttest", pulse_stdlib::stats::ttest_native, &mut exports, vm);
            add_native("chisquare", pulse_stdlib::stats::chisquare_native, &mut exports, vm);
            
            // Correlation and regression
            add_native("correlation", pulse_stdlib::stats::correlation_native, &mut exports, vm);
            add_native("linear_regression", pulse_stdlib::stats::linear_regression_native, &mut exports, vm);
            add_native("covariance", pulse_stdlib::stats::covariance_native, &mut exports, vm);
        }
        "std/random" => {
            // Basic random
            add_native("rand_int", pulse_stdlib::random::rand_int_native, &mut exports, vm);
            add_native("rand_int_range", pulse_stdlib::random::rand_int_range_native, &mut exports, vm);
            add_native("rand_float", pulse_stdlib::random::rand_float_native, &mut exports, vm);
            add_native("rand_float_range", pulse_stdlib::random::rand_float_range_native, &mut exports, vm);
            add_native("rand_bool", pulse_stdlib::random::rand_bool_native, &mut exports, vm);
            
            // Seedable RNG
            add_native("seed_rng", pulse_stdlib::random::seed_rng_native, &mut exports, vm);
            add_native("rng_state", pulse_stdlib::random::rng_state_native, &mut exports, vm);
            
            // Distributions
            add_native("uniform_sample", pulse_stdlib::random::uniform_sample_native, &mut exports, vm);
            add_native("normal_sample", pulse_stdlib::random::normal_sample_native, &mut exports, vm);
            add_native("exponential_sample", pulse_stdlib::random::exponential_sample_native, &mut exports, vm);
            add_native("poisson_sample", pulse_stdlib::random::poisson_sample_native, &mut exports, vm);
            
            // Sampling
            add_native("choice", pulse_stdlib::random::choice_native, &mut exports, vm);
            add_native("shuffle", pulse_stdlib::random::shuffle_native, &mut exports, vm);
            add_native("sample", pulse_stdlib::random::sample_native, &mut exports, vm);
            add_native("choices", pulse_stdlib::random::choices_native, &mut exports, vm);
            
            // Utilities
            add_native("random_bytes", pulse_stdlib::random::random_bytes_native, &mut exports, vm);
            add_native("random_hex", pulse_stdlib::random::random_hex_native, &mut exports, vm);
            add_native("random_string", pulse_stdlib::random::random_string_native, &mut exports, vm);
        }
        "std/plotting" => {
            add_native("bar_chart", pulse_stdlib::plotting::bar_chart_native, &mut exports, vm);
            add_native("line_chart", pulse_stdlib::plotting::line_chart_native, &mut exports, vm);
            add_native("histogram", pulse_stdlib::plotting::histogram_native, &mut exports, vm);
            add_native("box_plot", pulse_stdlib::plotting::box_plot_native, &mut exports, vm);
            add_native("scatter_plot", pulse_stdlib::plotting::scatter_plot_native, &mut exports, vm);
            add_native("hbar_chart", pulse_stdlib::plotting::hbar_chart_native, &mut exports, vm);
        }
        _ => return None,
    }
    
    let handle = vm.heap.alloc(Object::Module(exports));
    Some(handle)
}

fn add_native(name: &str, func: pulse_core::value::SyncNativeFn, exports: &mut HashMap<String, Value>, vm: &mut VM) {
    let native = NativeFn { name: name.to_string(), func: pulse_core::value::NativeFunctionKind::Sync(func) };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}

fn add_native_async(name: &str, func: pulse_core::value::AsyncNativeFn, exports: &mut HashMap<String, Value>, vm: &mut VM) {
    let native = NativeFn { name: name.to_string(), func: pulse_core::value::NativeFunctionKind::Async(func) };
    let handle = vm.heap.alloc(Object::NativeFn(native));
    exports.insert(name.to_string(), Value::Obj(handle));
}
