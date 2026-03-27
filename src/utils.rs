use rust_decimal::Decimal;

pub fn fmt_opt(v: Option<&Decimal>) -> String {
    v.map(|d| format!("{:.5}", d))
        .unwrap_or_else(|| "n/a".into())
}
