use tillerd_custom_macro::ErrorCode;

#[derive(ErrorCode)]
enum Missing {
    #[error_code("present")]
    Present,
    Absent,
}

fn main() {}
