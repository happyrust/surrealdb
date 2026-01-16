pub mod email {

	use addr::email::Host;
	use anyhow::Result;

	use crate::val::Value;

	pub fn host((string,): (String,)) -> Result<Value> {
		// Parse the email address
		Ok(match addr::parse_email_address(&string) {
			// Return the host part
			Ok(v) => match v.host() {
				Host::Domain(name) => name.as_str().into(),
				Host::IpAddr(ip_addr) => ip_addr.to_string().into(),
			},
			Err(_) => Value::None,
		})
	}

	pub fn user((string,): (String,)) -> Result<Value> {
		// Parse the email address
		Ok(match addr::parse_email_address(&string) {
			// Return the user part
			Ok(v) => v.user().into(),
			Err(_) => Value::None,
		})
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		#[test]
		fn host() {
			let input = (String::from("john.doe@example.com"),);
			let value = super::host(input).unwrap();
			assert_eq!(value, Value::from("example.com"));
		}

		#[test]
		fn user() {
			let input = (String::from("john.doe@example.com"),);
			let value = super::user(input).unwrap();
			assert_eq!(value, Value::from("john.doe"));
		}
	}
}

pub mod url {

	use anyhow::Result;
	use url::Url;

	use crate::val::Value;

	pub fn domain((string,): (String,)) -> Result<Value> {
		match Url::parse(&string) {
			Ok(v) => match v.domain() {
				Some(v) => Ok(v.into()),
				None => Ok(Value::None),
			},
			Err(_) => Ok(Value::None),
		}
	}

	pub fn fragment((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => match v.fragment() {
				Some(v) => Ok(v.into()),
				None => Ok(Value::None),
			},
			Err(_) => Ok(Value::None),
		}
	}

	pub fn host((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => match v.host_str() {
				Some(v) => Ok(v.into()),
				None => Ok(Value::None),
			},
			Err(_) => Ok(Value::None),
		}
	}

	pub fn path((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => Ok(v.path().into()),
			Err(_) => Ok(Value::None),
		}
	}

	pub fn port((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => match v.port_or_known_default() {
				Some(v) => Ok(v.into()),
				None => Ok(Value::None),
			},
			Err(_) => Ok(Value::None),
		}
	}

	pub fn query((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => match v.query() {
				Some(v) => Ok(v.into()),
				None => Ok(Value::None),
			},
			Err(_) => Ok(Value::None),
		}
	}

	pub fn scheme((string,): (String,)) -> Result<Value> {
		// Parse the URL
		match Url::parse(&string) {
			Ok(v) => Ok(v.scheme().into()),
			Err(_) => Ok(Value::None),
		}
	}

	#[cfg(test)]
	mod tests {
		use crate::val::Value;

		#[test]
		fn port_default_port_specified() {
			let value = super::port(("http://www.google.com:80".to_string(),)).unwrap();
			assert_eq!(value, Value::from(80));
		}

		#[test]
		fn port_nondefault_port_specified() {
			let value = super::port(("http://www.google.com:8080".to_string(),)).unwrap();
			assert_eq!(value, Value::from(8080));
		}

		#[test]
		fn port_no_port_specified() {
			let value = super::port(("http://www.google.com".to_string(),)).unwrap();
			assert_eq!(value, Value::from(80));
		}

		#[test]
		fn port_no_scheme_no_port_specified() {
			let value = super::port(("www.google.com".to_string(),)).unwrap();
			assert_eq!(value, Value::None);
		}
	}
}

pub mod uint64 {

	use anyhow::Result;

	use crate::val::{Array, Value};

	const HI_MAX: u64 = 0x7fff_ffff;
	const LO_MAX: u64 = 0xffff_ffff;
	const I64_MAX_U: u64 = i64::MAX as u64;

	fn strip_wrappers(s: &str) -> &str {
		let s = s.trim_start_matches("pe:");
		s.trim_matches(['`', '⟨', '⟩'])
	}

	fn combine(hi: u64, lo: u64) -> Option<i64> {
		if hi > HI_MAX || lo > LO_MAX {
			return None;
		}
		let v = (hi << 32) | lo;
		if v > I64_MAX_U {
			return None;
		}
		Some(v as i64)
	}

	fn parse_str(s: &str) -> Option<i64> {
		let raw = strip_wrappers(s);
		let mut parts = raw.split(|c| c == '_' || c == '/');
		let hi = parts.next()?.parse::<u64>().ok()?;
		let lo = parts.next()?.parse::<u64>().ok()?;
		combine(hi, lo)
	}

	fn parse_value(v: &Value) -> Option<i64> {
		match v {
			Value::Strand(s) => parse_str(s.as_str()),
			Value::String(s) => parse_str(s.as_str()),
			Value::Array(Array(items)) if items.len() == 2 => {
				let hi = items[0].clone().cast_to::<i64>().ok()?;
				let lo = items[1].clone().cast_to::<i64>().ok()?;
				if hi < 0 || lo < 0 {
					return None;
				}
				combine(hi as u64, lo as u64)
			}
			_ => None,
		}
	}

	pub fn to_u64((val,): (Value,)) -> Result<Value> {
		Ok(parse_value(&val).map(Value::from).unwrap_or(Value::None))
	}

		pub fn to_u64_many((val,): (Value,)) -> Result<Value> {
			match val {
				Value::Array(Array(items)) => Ok(Value::Array(Array(
					items
						.iter()
						.map(|v| parse_value(v).map(Value::from).unwrap_or(Value::None))
						.collect(),
				))),
				_ => Ok(Value::None),
			}
		}

	#[cfg(test)]
	mod tests {
		use super::*;

		#[test]
		fn parse_string_underscore() {
			let v = to_u64((Value::from("24383_73962"),)).unwrap();
			assert_eq!(v, Value::from(((24383u64 << 32) | 73962) as i64));
		}

		#[test]
		fn parse_array_pair() {
			let v = to_u64((Value::Array(Array(vec![Value::from(1), Value::from(2)])),)).unwrap();
			assert_eq!(v, Value::from(((1u64 << 32) | 2) as i64));
		}

		#[test]
		fn parse_many_mixed() {
			let input = Value::Array(Array(vec![
				Value::from("1/2"),
				Value::Array(Array(vec![Value::from(3), Value::from(4)])),
			]));
			let out = to_u64_many((input,)).unwrap();
			assert_eq!(
				out,
				Value::Array(Array(vec![
					Value::from(((1u64 << 32) | 2) as i64),
					Value::from(((3u64 << 32) | 4) as i64),
				]))
			);
		}
	}
}
