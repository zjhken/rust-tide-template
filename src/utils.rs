use async_std::task_local;

task_local! {
	static REQ_ID: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}
const TOKEN: u32 = 0x60db1e55;
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

macro_rules! retry_http {
	($f:expr, $maxTries:expr, $interval:expr, $retry_http_codes:expr) => {{
		let mut tries = 0;
		let result = loop {
			let result = $f;
			tries += 1;
			match result {
				Ok(ref resp) => {
					let status = resp.status();
					if ($retry_http_codes.contains(&(status as u16))) {
						tracing::warn!(
							"({}/{}) retry: bad status code. {}",
							tries,
							$maxTries,
							status
						);
						if tries >= $maxTries {
							tracing::error!("exceed maxTries");
							break result;
						}
						async_std::task::sleep(std::time::Duration::from_millis($interval)).await
					} else {
						break result;
					}
				}
				Err(ref e) => {
					tracing::warn!("({}/{}) retry: error. {}", tries, $maxTries, e);
					if tries >= $maxTries {
						tracing::error!("exceed maxTries");
						break result;
					}
					async_std::task::sleep(std::time::Duration::from_millis($interval)).await
				}
			}
		};
		result
	}};
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
