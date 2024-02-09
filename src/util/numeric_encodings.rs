

pub fn req_encoding_to_string(n: u16) -> &'static str {
    match n {
        0 => "ENQ_REQ",
        1 => "DEQ_REQ",
        2 => "ENQ_ACK",
        3 => "UNSAFE",
        4 => "SAFE",
        _ => ""
    }
}