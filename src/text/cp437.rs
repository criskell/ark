pub fn normalize_to_cp437(character: char) -> u8 {
    match character {
        'A' => 0x41,
        'B' => 0x42,
        'C' => 0x43,
        'á' => 0xA0,
        'à' => 0x85,
        'â' => 0x83,
        'ã' => 0xA3,
        'é' => 0x82,
        'ê' => 0x88,
        'í' => 0x8A,
        'ó' => 0xA2,
        'õ' => 0xA5,
        'ú' => 0xA4,
        'ç' => 0x87,
        ' '..'~' | '\n' | '\t' | '\r' => character as u8,
        _ => 0xfe,
    }
}
