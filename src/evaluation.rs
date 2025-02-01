use crate::jsonpp::JsonPP;

pub(crate) fn evaluate(value: JsonPP) -> serde_json::Value {
    loop {
        if let Ok(out) = value.clone().try_into() {
            return out;
        }
    }
}
