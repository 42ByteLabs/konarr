//! # Random utilities
use rand::Rng;

pub(crate) const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";

pub(crate) fn generate_random_string(len: usize) -> String {
    let mut rng = rand::rng();
    let mut random_string = String::new();
    for _ in 0..len {
        let random_char = CHARSET[rng.random_range(0..CHARSET.len())] as char;
        random_string.push(random_char);
    }

    random_string
}
