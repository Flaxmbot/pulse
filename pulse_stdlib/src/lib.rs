//! Pulse Standard Library - Native Functions

pub mod io;
pub mod json;
pub mod utils;
pub mod testing;
pub mod networking;
pub mod http;
pub mod database;
pub mod uuid;
pub mod regex;
pub mod string_utils;
pub mod time;
pub mod fs;
pub mod process;
pub mod logging;
pub mod numpy;
pub mod pandas;
pub mod linalg;
pub mod stats;
pub mod random;
pub mod plotting;

// Re-export all functions
pub use io::*;
pub use json::*;
pub use utils::*;
pub use testing::*;
pub use networking::*;
pub use http::*;
pub use regex::*;
pub use string_utils::*;

// Time functions - only re-export unique ones
pub use time::current_timestamp_native;
pub use time::current_timestamp_millis_native;
pub use time::current_timestamp_micros_native;
pub use time::sleep_seconds_native;
pub use time::now_native;
pub use time::format_time_native;
pub use time::parse_time_native;
pub use time::duration_create_native;
pub use time::duration_add_native;
pub use time::measure_time_native;
pub use time::unix_to_datetime_native;
pub use time::datetime_to_unix_native;

// FS functions - only re-export unique ones
pub use fs::read_dir_native;
pub use fs::create_dir_native;
pub use fs::remove_dir_native;
pub use fs::remove_file_native;
pub use fs::get_metadata_native;
pub use fs::copy_file_native;
pub use fs::rename_file_native;
pub use fs::list_dir_native;
pub use fs::is_file_native;
pub use fs::is_dir_native;
pub use fs::read_bytes_native;
pub use fs::write_bytes_native;
pub use fs::get_current_dir_native;
pub use fs::set_current_dir_native;

// Process functions
pub use process::spawn_process_native;
pub use process::wait_process_native;
pub use process::kill_process_native;
pub use process::exit_code_native;
pub use process::process_running_native;
pub use process::shell_native;
pub use process::system_info_native;
pub use process::get_env_native;
pub use process::set_env_native;
pub use process::get_args_native;
pub use process::get_pid_native;

// Logging functions
pub use logging::set_log_level_native;
pub use logging::get_log_level_native;
pub use logging::debug_native;
pub use logging::info_native;
pub use logging::warn_native;
pub use logging::error_native;
pub use logging::log_native;
pub use logging::set_log_format_native;
pub use logging::enable_logging_native;
pub use logging::disable_logging_native;
pub use logging::logging_enabled_native;
pub use logging::log_fatal_native;
pub use logging::log_debug_if_native;
pub use logging::log_info_if_native;
pub use logging::trace_native;
pub use logging::log_with_context_native;

// HTTP client functions
pub use http::http_get_native;
pub use http::http_post_native;
pub use http::http_put_native;
pub use http::http_delete_native;
pub use http::http_request_native;
pub use http::http_get_body_native;
pub use http::http_parse_native;
pub use http::http_format_response_native;

// Database functions
pub use database::db_open_native;
pub use database::db_open_memory_native;
pub use database::db_execute_native;
pub use database::db_query_native;
pub use database::db_close_native;
pub use database::db_begin_native;
pub use database::db_commit_native;
pub use database::db_rollback_native;
pub use database::db_tables_native;

// UUID functions
pub use uuid::uuid_generate_native;
pub use uuid::uuid_v4_native;
pub use uuid::uuid_parse_native;
pub use uuid::uuid_to_string_native;
pub use uuid::uuid_is_valid_native;
pub use uuid::uuid_nil_native;
pub use uuid::uuid_namespace_ns_dns_native;
pub use uuid::uuid_namespace_ns_url_native;
pub use uuid::uuid_namespace_ns_oid_native;
pub use uuid::uuid_namespace_x500_native;
pub use uuid::uuid_from_bytes_native;

// NumPy functions - array creation
pub use numpy::array_create_native;
pub use numpy::array_zeros_native;
pub use numpy::array_ones_native;
pub use numpy::array_eye_native;
pub use numpy::array_linspace_native;
pub use numpy::array_arange_native;

// NumPy functions - array operations
pub use numpy::array_shape_native;
pub use numpy::array_reshape_native;
pub use numpy::array_get_native;
pub use numpy::array_set_native;
pub use numpy::array_slice_native;

// NumPy functions - matrix operations
pub use numpy::matmul_native;
pub use numpy::dot_native;
pub use numpy::transpose_native;
pub use numpy::inverse_native;
pub use numpy::determinant_native;

// NumPy functions - element-wise operations
pub use numpy::add_native;
pub use numpy::sub_native;
pub use numpy::mul_native;
pub use numpy::div_native;
pub use numpy::sqrt_native;
pub use numpy::abs_native;
pub use numpy::pow_native;
pub use numpy::sin_native;
pub use numpy::cos_native;
pub use numpy::tan_native;
pub use numpy::exp_native;
pub use numpy::numpy_log_native;
pub use numpy::log10_native;
pub use numpy::floor_native;
pub use numpy::ceil_native;
pub use numpy::round_native;
pub use numpy::negate_native;

// NumPy functions - aggregations
pub use numpy::sum_native;
pub use numpy::mean_native as np_mean_native;
pub use numpy::std_native as np_std_native;
pub use numpy::var_native;
pub use numpy::min_native as np_min_native;
pub use numpy::max_native as np_max_native;
pub use numpy::argmin_native;
pub use numpy::argmax_native;

// NumPy functions - constants and utilities
pub use numpy::pi_native;
pub use numpy::e_native;

// NumPy log function (renamed to avoid conflict with logging::log_native)
// numpy_log_native is exported above

// ============================================================================
// LINALGEBRA LIBRARY
// ============================================================================

// Vector operations
pub use linalg::vector_dot_native;
pub use linalg::vector_cross_native;
pub use linalg::vector_normalize_native;
pub use linalg::vector_magnitude_native;

// Matrix operations
pub use linalg::matrix_multiply_native;
pub use linalg::matrix_transpose_native;
pub use linalg::matrix_inverse_native;
pub use linalg::matrix_determinant_native;

// Matrix decomposition
pub use linalg::matrix_lu_native;
pub use linalg::matrix_qr_native;
pub use linalg::matrix_svd_native;

// Linear system solver
pub use linalg::solve_linear_native;

// Matrix creation
pub use linalg::matrix_identity_native;
pub use linalg::matrix_zeros_native;
pub use linalg::matrix_ones_native;

// ============================================================================
// STATISTICS LIBRARY
// ============================================================================

// Descriptive statistics
pub use stats::mean_native as stats_mean_native;
pub use stats::median_native;
pub use stats::mode_native;
pub use stats::std_native as stats_std_native;
pub use stats::variance_native;
pub use stats::min_native as stats_min_native;
pub use stats::max_native as stats_max_native;
pub use stats::describe_native;

// Probability distributions
pub use stats::normal_pdf_native;
pub use stats::normal_cdf_native;
pub use stats::normal_sample_native;
pub use stats::binomial_pmf_native;
pub use stats::poisson_pmf_native;

// Hypothesis testing
pub use stats::ttest_native;
pub use stats::chisquare_native;

// Correlation and regression
pub use stats::correlation_native;
pub use stats::linear_regression_native;
pub use stats::covariance_native;

// ============================================================================
// RANDOM NUMBER GENERATION LIBRARY
// ============================================================================

// Basic random
pub use random::rand_int_native;
pub use random::rand_int_range_native;
pub use random::rand_float_native;
pub use random::rand_float_range_native;
pub use random::rand_bool_native;

// Seedable RNG
pub use random::seed_rng_native;
pub use random::rng_state_native;

// Distributions
pub use random::uniform_sample_native;
pub use random::normal_sample_native as random_normal_sample_native;
pub use random::exponential_sample_native;
pub use random::poisson_sample_native;

// Sampling
pub use random::choice_native;
pub use random::shuffle_native;
pub use random::sample_native;
pub use random::choices_native;

// Utilities
pub use random::random_bytes_native;
pub use random::random_hex_native;
pub use random::random_string_native;

// ============================================================================
// PLOTTING LIBRARY (ASCII)
// ============================================================================

pub use plotting::bar_chart_native;
pub use plotting::line_chart_native;
pub use plotting::histogram_native;
pub use plotting::box_plot_native;
pub use plotting::scatter_plot_native;
pub use plotting::hbar_chart_native;

// PANDAS - DATAFRAME LIBRARY
// ============================================================================

// Pandas module
pub use pandas::*;

// DataFrame creation
pub use pandas::df_create_native;
pub use pandas::df_from_list_native;
pub use pandas::df_from_csv_native;
pub use pandas::df_from_json_native;

// DataFrame operations
pub use pandas::df_columns_native;
pub use pandas::df_shape_native;
pub use pandas::df_head_native;
pub use pandas::df_tail_native;
pub use pandas::df_select_native;
pub use pandas::df_filter_native;
pub use pandas::df_sort_native;

// Data operations
pub use pandas::df_group_by_native;
pub use pandas::df_aggregate_native;
pub use pandas::df_join_native;
pub use pandas::df_concat_native;

// Column operations
pub use pandas::df_add_column_native;
pub use pandas::df_drop_column_native;
pub use pandas::df_rename_native;

// Statistics
pub use pandas::df_describe_native;
pub use pandas::df_corr_native;
