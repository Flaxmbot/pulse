; ModuleID = 'pulse_aot_module'
source_filename = "pulse_aot_module"
target triple = "x86_64-pc-windows-msvc"

@str_const = private unnamed_addr constant [2 x i8] c"\0A\00", align 1

declare void @pulse_print_int(i64)

declare void @pulse_print_float(i64)

declare void @pulse_print_bool(i64)

declare void @pulse_print_newline()

declare void @pulse_print_string(ptr, i64)

define i64 @pulse_main() {
entry:
  %global_tag_0 = alloca i64, align 8
  %global_val_0 = alloca i64, align 8
  store i64 0, ptr %global_tag_0, align 4
  store i64 0, ptr %global_val_0, align 4
  store i64 2, ptr %global_tag_0, align 4
  store i64 42, ptr %global_val_0, align 4
  %global_tag_2 = alloca i64, align 8
  %global_val_2 = alloca i64, align 8
  store i64 0, ptr %global_tag_2, align 4
  store i64 0, ptr %global_val_2, align 4
  %gtag = load i64, ptr %global_tag_2, align 4
  %gval = load i64, ptr %global_val_2, align 4
  call void @pulse_print_int(i64 %gval)
  call void @pulse_print_int(i64 ptrtoint (ptr @str_const to i64))
  ret i64 0
}

define i32 @main() {
entry:
  %result = call i64 @pulse_main()
  %exit_code = trunc i64 %result to i32
  ret i32 %exit_code
}
