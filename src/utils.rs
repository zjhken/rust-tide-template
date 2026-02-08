use anyhow_ext::{Context, Result};
use async_std::{io::WriteExt, task_local};
use tracing::{Instrument, debug, error, info, info_span};


const RELAY_BUFFER_SIZE: usize = 65536; // 64KB for traffic relay

task_local! {
	pub static REQ_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

const A2Z: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub fn gen_n_random_str(n: u8) -> String {
	(0..n)
		.map(|_| {
			let idx = rand::random::<u8>() % (A2Z.len() as u8);
			A2Z.chars().nth(idx as usize).unwrap()
		})
		.collect()
}

pub(crate) fn set_req_id() {
	REQ_ID.with(|s| {
		let mut ss = s.borrow_mut();
		ss.clear();
		ss.push_str(&gen_n_random_str(7));
	});
}

pub(crate) fn get_req_id() -> String {
	let mut id = String::new();
	REQ_ID.with(|s| {
		id.push_str(s.borrow().as_str());
	});
	return id;
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_gen_random_str_length() {
		let result = gen_n_random_str(7);
		assert_eq!(result.len(), 7, "Generated string should have length 7");
	}

	#[test]
	fn test_gen_random_str_characters() {
		let result = gen_n_random_str(7);
		let valid_chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

		for byte in result.as_bytes() {
			assert!(
				valid_chars.contains(byte),
				"Invalid character '{}' in generated string",
				*byte as char
			);
		}
	}

	#[test]
	fn test_gen_random_str_deterministic() {
		// Test that function works and returns valid string
		let result1 = gen_n_random_str(7);
		assert_eq!(result1.len(), 7);

		// Test that function can be called multiple times
		let result2 = gen_n_random_str(7);
		assert_eq!(result2.len(), 7);

		// Results should likely be different due to time difference
		// (though theoretically they could be the same)
	}

	#[test]
	fn test_gen_random_str_format() {
		let result = gen_n_random_str(7);
		println!("Generated random string: {}", result);
		assert_eq!(result.len(), 7);
		assert!(!result.is_empty());
	}
}
