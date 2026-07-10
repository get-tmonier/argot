# ID: crates/printer/src/util.rs:425
fn build_decimal_formatter(mut value: u64) -> DecimalFormatter {
    let mut buf = [0; DecimalFormatter::MAX_U64_LEN];
    let mut start = buf.len();
    loop {
        start -= 1;
        let digit = u8::try_from(value % 10).unwrap();
        value /= 10;
        buf[start] = b'0' + digit;
        if value == 0 {
            break;
        }
    }
    DecimalFormatter { buf, start }
}
