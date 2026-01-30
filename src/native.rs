use crate::value::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn clock(_arg_count: usize, _args: &[Value]) -> Value {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Value::Number(duration.as_secs_f64())
}
