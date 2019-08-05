
use unicode_segmentation::UnicodeSegmentation;
use chrono::prelude::*;
use chrono::Duration;
extern crate unicode_segmentation;
extern crate chrono;
extern crate serde_yaml;
extern crate serde;


pub mod prelude {
	pub use super::{process_command, execute_command, lg_types};
	pub use super::log::*;
}

// //// Log //// //

pub mod log {
	use super::lg_types::*;
	use unicode_segmentation::UnicodeSegmentation;

	#[derive(Debug, Eq, Clone, serde::Serialize, serde::Deserialize)]
	pub struct LogEntry {
		pub time: chrono::DateTime<chrono::FixedOffset>,
		pub data: String,
		pub kind: String,
		pub note: String,
	}
	// span, data

	impl LogEntry {
		pub fn new(time: chrono::DateTime<chrono::FixedOffset>, kind: &str, data: &str, note: &str) -> LogEntry {
			LogEntry { time, kind: kind.to_string(), data: data.to_string(), note: note.to_string() }
		}
		pub fn make(time: chrono::DateTime<chrono::FixedOffset>, data: ValidData) -> LogEntry {
			LogEntry { time, kind: data.kind, data: data.data, note: data.note }
		}
	}

	impl Ord for LogEntry {
		fn cmp(&self, other: &Self) -> std::cmp::Ordering {
			self.time.cmp(&other.time)
		}
	}
	impl PartialOrd for LogEntry {
		fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
			self.time.partial_cmp(&other.time)
		}
	}
	//impl Eq for LogEntry {}
	impl PartialEq for LogEntry {
		fn eq(&self, other: &Self) -> bool {
			self.time == other.time
		}
	}

	#[derive(Debug, serde::Serialize, serde::Deserialize)]
	pub struct Log {
		// For space efficiency, might consider making LogEntry `String`s into
		// `&str` and hosting them all in a table.
		// str_table: Vec<String>;
		first: LogEntry,
		pub vec: Vec<LogEntry>,
	}

	impl Log {
		// Running iter() should guarantee datetime-sorted results.
		pub fn new() -> Log {
			Log {
				first: LogEntry::new(chrono::DateTime::parse_from_rfc3339("1900-01-01T00:00:00-07:00").unwrap(), "", "", ""),
				vec: Vec::<LogEntry>::new(),
			}
		}

		pub fn iter(&self) -> std::slice::Iter<LogEntry> {
			self.vec.iter()
		}

		pub fn add(&mut self, time: chrono::DateTime<chrono::FixedOffset>, data: String, kind: String, note: String) {
			self.push( LogEntry {
				time, data, kind, note
			});
		}

		pub fn push(&mut self, entry: LogEntry) {
			let current = self.task_at(entry.time);
			if (current.kind == entry.kind)
			& (current.data == entry.data)
			& (current.note == entry.note) {
				return
			}
			self.vec.push(entry);
			self.vec.sort();
		}

		pub fn remove(&mut self, time: chrono::DateTime<chrono::FixedOffset>) -> LogEntry {
			for (i, entry) in self.vec.iter().enumerate() {
				if entry.time >= time {
					return self.vec.remove(i)
				}
			}
			self.vec.remove(self.vec.len())
		}

		pub fn first(&self) -> &LogEntry {
			// Shall, later, return the actual first entry in the list.
			// Which _will be_ a guaranteed nil entry starting at the earliest
			// possible moment--the big bang. Or whatever is the minimal date our format allows.
			&self.first
		}

		pub fn task_at(&self, time: chrono::DateTime<chrono::FixedOffset>) -> &LogEntry {
			// Identify the singular task that overlaps with the given datetime.
			let mut task = self.first();
			for entry in self.iter() {
				if entry.time <= time {
					task = entry;
				} else {
					break;
				}
			}
			task
		}

		pub fn as_string(&self, scale: f32, delimit: String) -> (String, Vec<&str>) {
			// NOTE
			// len shouldn't be capable of being negative--all dates are sorted in the log-generating stage.
			// Nonetheless, we might want to convert logs from timestamps into durations for safety purposes.
			// Similarly, we ought to generate log outputs in a more data-oriented than string-oriented way
			// so we can produce better, cleaner output. Move this complexity to... something else.

			fn trim_pad(data: &str, len: usize) -> String {
				// Trim or pad a string with ' ' to make it fit the desired len.
				match data.len() {
					// Pad data if short.
					x if x <= len => [
							//delimit,
							data,
							" ".repeat(len - x).as_str()
						].concat(),
					// Trim data if long.
					_ => UnicodeSegmentation::graphemes(data, true)
						 .collect::<Vec<&str>>()[..len]
						 .to_vec()
						 .join(""),
				}
			}
			let mut key_set = Vec::<&str>::new();
			let mut result = String::new();
			let mut iter = self.vec.iter();

			// Initialize loop and, if empty,
			// simply an empty string.
			let mut entry = match iter.next() {
				Some(e) => {
					key_set.push(&e.kind);
					e
				},
				None => return ("".to_string(), key_set),
			};

			// Initialize loop and, only one entry,
			// return simply the first result.
			let mut next_entry = match iter.next() {
				Some(e) => {
					key_set.push(&e.kind);
					e
				},
				None => return ([&entry.data, " ..."].concat(), key_set),
			};

			// Compute available width.
			let len = next_entry.time
								.signed_duration_since(entry.time)
								.num_minutes();
			let len = (scale * len as f32) as usize;// - delimit.len();
			let len = match len >= delimit.len() {
				true => len - delimit.len(),
				false => 0,
			};

			// Finish initializing the loop with the first entry, trimmed according
			// to its length--dictated by the followup entry's position.
			let data: String = trim_pad(&entry.data, len);
			result.push_str(data.as_str());
			// Loops over all subsequent entries.
			loop {
				entry = next_entry;
				next_entry = match iter.next() {
					Some(e) => {
						if ! key_set.contains(&e.kind.as_str()) {
							key_set.push(&e.kind);
						}
						e
					},
					None => {
						// Exit condition.
						// The final task lasts eternity so no need to trim it.
						result.push_str(&delimit);
						result.push_str(entry.data.as_str());
						//result.push_str(" ...");
						break
					},
				};
				// Compute available width.
				let len = next_entry.time
									.signed_duration_since(entry.time)
									.num_minutes();
				let len = (scale * len as f32) as usize;
				let value: String = match len >= delimit.len() {
					true => [delimit.as_str(), trim_pad(&entry.data, len - delimit.len()).as_str()].concat(),
					false => delimit.graphemes(true)
								.collect::<Vec<&str>>()[..len]
								.to_vec()
								.join(""),
				};
				result.push_str(value.as_str());
			}
			(result, key_set)
		}
	}
}


// //// Time //// //

fn now() -> chrono::DateTime<chrono::FixedOffset> {
	//chrono::DateTime::<chrono::FixedOffset>::from_utc(Utc::now().naive_utc(), chrono::FixedOffset)
	Utc::now().with_timezone(&chrono::FixedOffset::west(7 * 3600))
	//Local::now().with_timezone(&Utc)
}

fn parse_time_str(time_str: &str) -> Vec<i64> {
	// Convert user input into numerals.
	// 0:30 => 0, 30
	// 1: => 1, 0
	let mut time_units = Vec::<i64>::new();
	for unit in time_str.split(":") {
		// Parse the substrings into numbers.
		let unit = match unit {
			"" => Ok(0),
			_ => unit.parse::<i64>(),
		};

		let unit = match unit {
			Ok(unit) => unit,
			Err(err) => panic!("Invalid timestamp--must be of `1:`, `:30`, or `30` format. {}", err),
		};
		time_units.push(unit);
	}
	time_units
}

fn parse_time(time_str: &str) -> NaiveTime {
	//chrono::DateTime::parse_from_str("2019 14 1:46 +0000", "%Y %j %H:%M %z").expect("T475")
	let time_units: Vec<u32> = parse_time_str(time_str)
		.into_iter()
		.map(|n| n as u32)
		.collect();
	// Convert into minute totals.
	let minutes = match time_units.len() {
		2 => NaiveTime::from_num_seconds_from_midnight_opt( time_units[0]*3600 + time_units[1]*60, 0),
		1 => NaiveTime::from_num_seconds_from_midnight_opt( time_units[0]*60, 0),
		_ => panic!("Invalid timestamp--must be of `0:30` or `30` format.")
	};
	match minutes {
		Some(m) => m,
		None => panic!("Not a valid timestamp! Must be of range 0:00-23:59. {}", time_str)
	}
}

fn parse_duration(time_str: &str) -> chrono::Duration {
	let time_units = parse_time_str(time_str);
	// Convert into minute totals.
	let minutes = match time_units.len() {
		2 => time_units[0]*60 + time_units[1],
		1 => time_units[0],
		_ => panic!("Invalid timestamp--must be of `0:30` or `30` format.")
	};
	chrono::Duration::minutes(minutes)
}

fn map_time_after_datetime(time: NaiveTime, date_frame: chrono::DateTime<chrono::FixedOffset>) -> chrono::DateTime<chrono::FixedOffset> {
	// Input time and date and output the earliest datetime matching that time.
	// I.e. `3:00AM` and `Wed 5:00PM` output `Tue, 3:00AM`.
	let time = date_frame.date().and_time(time).expect("Invalid date + time!");
	match date_frame.time() < time.time() {
		true => time,
		false => time + Duration::days(1),
	}
}


// //// Types //// //

pub mod lg_types {
	use super::*;
	use super::log::*;
	#[derive(Debug, PartialEq, Clone)]
	pub enum RawInit {
		Now,
		Retcon,
		Time(String),
	}

	#[derive(Debug, PartialEq, Clone)]
	pub enum RawTill {
		Nil,
		For(String),
		Till(String),
	}

	#[derive(Debug)]
	pub enum ValidInit {
		Now(chrono::DateTime<chrono::FixedOffset>),
		Retcon(chrono::DateTime<chrono::FixedOffset>),
		Time(chrono::DateTime<chrono::FixedOffset>),
	}

	#[derive(Debug)]
	pub enum ValidTill {
		Nil,
		For(chrono::Duration),
		Till(chrono::DateTime<chrono::FixedOffset>),
	}

	#[derive(Debug, Clone)]
	pub struct ValidData {
		pub data: String,
		pub kind: String,
		pub note: String,
	}

	#[derive(Debug)]
	pub struct RawStatement {// StrStatement
		pub init: RawInit,
		pub till: RawTill,
		pub data: Option<ValidData>,
	}

	#[derive(Debug)]
	pub struct ValidStatement {
		pub init: ValidInit,
		pub till: ValidTill,
		pub data: Option<ValidData>,
	}

	impl RawStatement {
		pub fn compile(&self, log: &mut Log) -> Option<ValidStatement> {
			let time = match self.init.clone() {
				RawInit::Now => now(),
				RawInit::Retcon => match log.task_at(now()).kind.as_str() {
					"∅" => {
						let time = log.task_at(now()).time;
						log.remove(time);
						time
					},
					_ => now(),
				},
				RawInit::Time(t) => map_time_after_datetime(parse_time(&t), now() - Duration::hours(12)),
			};
			let init = match self.init {
				RawInit::Now => ValidInit::Now(time),
				RawInit::Retcon => ValidInit::Retcon(time),
				RawInit::Time(_) => ValidInit::Time(time),
			};
			//let init = ValidInit::Time(time);
			let till = match self.till.clone() {
				RawTill::Nil => ValidTill::Nil,
				RawTill::For(t) => ValidTill::For(parse_duration(&t)),
				RawTill::Till(t) => ValidTill::Till(
					map_time_after_datetime(parse_time(&t), time - Duration::hours(12))),
			};
			let data = match &self.data {
				Some(d) => Some(d.clone()),
				None => None,
			};
			//map_time_after_datetime(parse_time(&t), now() - Duration::hours(12))
			//map_time_after_datetime(parse_time(&t), start_time - Duration::hours(12))
			Some(ValidStatement { init, data, till })
		}
	}

}


// //// Functions //// //


fn join_logs(logs: Vec<(String, usize)>) -> String {
	// Split the logs into vectors of graphemes paired with the allowed
	// width of that log's output.
	// Reverse the order of each log so we can pop each grapheme, first to last.
	let mut logs: Vec<(Vec<_>, usize)> = logs.into_iter()
		.map( |(log, len)| (
			log.graphemes(true)
			   .map( |n| n.to_string() )
			   .rev()
			   .collect(),
			len
		))
		.collect();

	// Compute how many rows we'll need to fit all the lots,
	// given their wrap-around widths.
	let cap: usize = logs.iter()
		.map( |(k, l)| (k.len() as f64 / *l as f64).ceil() as usize )
		.collect::<Vec<_>>()
		.into_iter()
		.max()
		.unwrap();

	// Generate string chart.
	let mut concat_log = Vec::new(); //vec![String::new(); logs.len()];
	for _i in 0..cap {
		let mut concat_log_line = String::new();
		for (i, (log, len)) in &mut logs.iter_mut().enumerate() {
			let len = *len;
			let mut substr = String::with_capacity(len);
			for _ in 0..len {
				match log.pop() {
					Some(grapheme) => substr.push_str(&grapheme),
					None => substr.push(' '),
				}
			}
			if i > 0 {
				concat_log_line.push('┃');
			}
			concat_log_line.push_str(&substr);
		}
		concat_log_line.push('\n');
		concat_log.push(concat_log_line);
	}
	concat_log.join("")
}


pub fn execute_command(cmd: lg_types::ValidStatement, log: &mut log::Log) -> Vec<log::LogEntry> {
	use lg_types::ValidInit::{Retcon, Now, Time};
	use lg_types::ValidTill::{Nil, For, Till};
	let time = match cmd.init {
		Retcon(t) => t,
		Now(t) => t,
		Time(t) => t,
	};
	let end = match cmd.till {
		Nil => None,
		For(duration) => Some(time + duration),
		Till(t) => Some(map_time_after_datetime(t.time(), time)),
	};
	let mut vec = Vec::new();
	let data = cmd.data.unwrap();
	vec.push( log::LogEntry::new(time, &data.kind, &data.data, &data.note) );
	if let Some(t) = end {
		vec.push( log::LogEntry::new(t, "∅", "", "") );
	}
	vec
}

pub fn process_command(cmd: lg_types::RawStatement, log: &mut log::Log) -> Vec<log::LogEntry> {
	use lg_types::ValidStatement;
	use lg_types::ValidInit::{Retcon, Now, Time};
	use lg_types::ValidTill::{Nil, For, Till};
	let cmd = match cmd.compile(log) {
		Some(cmd) => cmd,
		None => panic!("Command didn't compile!"),
	};
	match &cmd {
		ValidStatement { init: _, till: _, data: Some(_) } =>
			execute_command(cmd, log),
		ValidStatement { init: Retcon(time), till: Nil, data: None } =>
			// Set the currently active entry to nil or retrieve and print it?
			panic!(),
		ValidStatement { init: Time(time), till: Nil, data: None } =>
			// Insert a Nil task at @TIME.
			// TIME is mapped to +/- 12 hours from _now,_ whether that
			// is today, yesterday, or tomorrow.
			// 17:00 -> 5:00 - 4:59
			// 22:00 -> 10:00 - 9:59
			vec!( log::LogEntry::new(
				map_time_after_datetime(
					time.time(),
					now() - Duration::hours(12),
				),
				"",
				"",
				"",
			) ),
		ValidStatement { init: Now(time), till: Nil, data: None } => {
			// No actual data was provided! Retrieve and print the day.
			//
			let width = 80;
			let log = log.as_string(1.0, "|".to_string()).0;
			//let a_log = log.as_string(width as f32/60.0, "|".to_string()).0;
			//let b_log = beta_log.as_string(1.0/60.0, "".to_string()).0;

			let vec_logs = vec!( (log, width) );
			let joined_log = join_logs(vec_logs);
			//
			println!("{}", joined_log);
			panic!()
		},
		ValidStatement { init: Retcon(time), till: For(_), data: None }
		| ValidStatement { init: Retcon(time), till: Till(_), data: None }
		| ValidStatement { init: Now(time), till: For(_), data: None }
		| ValidStatement { init: Now(time), till: Till(_), data: None } =>
			// Retcon the current task to end at time T. If actively Nil, retcon the _preceding_ task.
			panic!(),
		ValidStatement { init: Time(time), till: For(_), data: None }
		| ValidStatement { init: Time(time), till: Till(_), data: None } =>
			// Set this timeframe as nil or retrieve and print this timeframe.
			panic!(),
	}
}


// Save/load
