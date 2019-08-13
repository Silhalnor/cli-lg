
use time::*;
//use unicode_segmentation::UnicodeSegmentation;
extern crate unicode_segmentation;
extern crate chrono;
extern crate serde_yaml;
extern crate serde;

fn unit<T>(_: T) -> () {}

pub mod prelude {
	pub use super::{process_command, execute_command, lg_types};
	pub use super::log::*;
}

// //// Log //// //
pub mod log {
	use super::lg_types::*;
	use unicode_segmentation::UnicodeSegmentation;
	use chrono::prelude::*;

	// Primary log superblock.
	// Minlog representation of refs.
	// Slice representation of refs. But we're cutting on time blocks, slicing a single activity in twain.
	// Explicit time bound stamps.
	// Plus a "fake" nil event at the end_slice_point.
	// Plus a "fake" copy at the start_slice_point event.

	#[derive(Debug, Eq, Clone, serde::Serialize, serde::Deserialize)]
	pub struct LogEntry {
		pub time: DateTime<FixedOffset>,
		pub data: String,
		pub kind: String,
		pub note: String,
	}
	// span, data

	impl LogEntry {
		pub fn new(time: DateTime<FixedOffset>, kind: &str, data: &str, note: &str) -> LogEntry {
			LogEntry { time, kind: kind.to_string(), data: data.to_string(), note: note.to_string() }
		}
		pub fn make(time: DateTime<FixedOffset>, data: ValidData) -> LogEntry {
			LogEntry { time, kind: data.kind, data: data.data, note: data.note }
		}
		pub fn update(&mut self, time: DateTime<FixedOffset>, kind: &str, data: &str, note: &str) {
			self.time = time;
			self.kind = kind.to_string();
			self.data = data.to_string();
			self.note = note.to_string();
		}
		pub fn update_time(&mut self, time: DateTime<FixedOffset>) {
			self.time = time;
		}
		pub fn nil(time: DateTime<FixedOffset>) -> LogEntry {
			LogEntry::new(time, "∅", "", "")
		}
		pub fn empty(time: DateTime<FixedOffset>) -> LogEntry {
			LogEntry::new(time, "", "", "")
		}
		pub fn is_nil(&self) -> bool {
			self.kind == "∅"
		}
		pub fn is_empty(&self) -> bool {
			(self.kind == "") & (self.data == "") & (self.note == "")
		}
	}

	impl Default for LogEntry {
		fn default() -> Self {
			Self {
				time: DateTime::parse_from_str("2000 1 0:00 -0700", "%Y %j %H:%M %z").expect("T473"),
				data: "".to_string(),
				kind: "".to_string(),
				note: "".to_string(),
			}
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
				first: LogEntry::new(DateTime::parse_from_rfc3339("1900-01-01T00:00:00-07:00").unwrap(), "", "", ""),
				vec: Vec::<LogEntry>::new(),
			}
		}

		pub fn iter(&self) -> LogIter {
			self.into_iter()
		}

		pub fn iter_range(&self, start: DateTime<FixedOffset>, end: DateTime<FixedOffset>) -> LogRangeIter {
			let mut range: Vec<&LogEntry> = Vec::new();
			let mut iter = self.iter();

			// Initialize the loop so we can track
			// every element and its predecessor.
			let mut prev: &LogEntry = match iter.next() {
				Some(e) => e,
				None => return LogRangeIter {
					index: 0,
					range: Vec::<&LogEntry>::new(),
				},
			};

			// Seek the start time.
			loop {
				let task = match iter.next() {
					Some(e) => e,
					None => break,
				};

				// Add the task and its predecessor if it
				// proved to fall within range after all.
				if task.time == start {
					range.push(task);
					break;
				} else if task.time > start {
					range.push(prev);
					range.push(task);
					break;
				}
				prev = task;
			}

			// Then iterate normally till the end.
			for task in iter {
				if task.time > end {
					break;
				}
				range.push(task);
			}

			LogRangeIter {
				index: 0,
				range: range,
			}
		}

		pub fn add(&mut self, time: DateTime<FixedOffset>, data: String, kind: String, note: String) {
			self.push( LogEntry {
				time, data, kind, note
			});
		}

		pub fn update(&mut self, entry: LogEntry) {
			// If the timestamp already existed, then edit its content.
			// If kind, data, and note are _blank_ then remove the entry.
			// Otherwise, add as a new entry.
			let (task, index) = self.task_index_at(entry.time);
			match (entry.is_empty(), entry.time == task.time) {
				(true, _) => super::unit(self.vec.remove(index)),
				(false, true) => self.vec[index].update(entry.time, &entry.kind, &entry.data, &entry.note),
				(false, false) => self.push(entry),
			}
		}

		pub fn push(&mut self, entry: LogEntry) {
			// If the task to precede entry has
			// the same content, skip this entry.
			let (current, index) = self.task_index_at(entry.time);
			if (current.kind == entry.kind)
			& (current.data == entry.data)
			& (current.note == entry.note) {
				return
			}
			// If the subsequent task has
			// the same content, delete it.
			if let Some(next) = self.successor(current) {
				if (next.kind == entry.kind)
				& (next.data == entry.data)
				& (next.note == entry.note) {
					self.vec.remove(index+1);
				}
			}
			// _Now_ append the new entry and ensure orderedness.
			self.vec.push(entry);
			self.vec.sort();
		}

		pub fn remove(&mut self, time: DateTime<FixedOffset>) -> LogEntry {
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

		pub fn predecessor(&self, entry: &LogEntry) -> Option<&LogEntry> {
			let mut iter = self.iter();
			let mut pred = iter.next();
			for elem in iter {
				if &elem.time == &entry.time {
					return pred;
				}
				pred = Some(elem);
			}
			None
		}

		pub fn successor(&self, entry: &LogEntry) -> Option<&LogEntry> {
			let mut iter = self.iter();
			loop {
				match iter.next() {
					Some(elem) if std::ptr::eq(&elem, &entry) =>
						return iter.next(),
					Some(_) => (),
					None => return None,
				}
			}
		}

		fn task_index_at(&self, time: DateTime<FixedOffset>) -> (&LogEntry, usize) {
			// Identify the singular task that overlaps with the given datetime.
			let mut task = self.first();
			let mut i = -1; // Refactor?
			for entry in self {
				match entry.time <= time {
					true => task = entry,
					false => break,
				}
				i += 1;
			}
			if i < 0 { i = 0 }
			(task, i as usize)
		}

		pub fn task_at(&self, time: DateTime<FixedOffset>) -> &LogEntry {
			self.task_index_at(time).0
		}

		pub fn mut_task_at(&mut self, time: DateTime<FixedOffset>) -> &mut LogEntry {
			let mut pred: &mut LogEntry = &mut self.first;
			for elem in self.vec.iter_mut() {
				if elem.time > time {
					return pred;
				}
				pred = elem;
			}
			pred
		}

		pub fn slice(&self, start:DateTime<FixedOffset>, end: DateTime<FixedOffset>) -> LogSlice {
			let mut iter = self.iter_range(start, end);
			// Extract the first entry and set its
			// starting time to that of the slice.
			let first = match iter.next() {
				Some(first) => LogEntry {
					time: start,
					..(*first).clone()
				},
				None => LogEntry {
					time: start,
					kind: "∅".to_string(),
					data: "".to_string(),
					note: "".to_string(),
				},
			};
			LogSlice {
				start_bound: start,
				end_bound: end,
				first: first,
				slice: iter
					.collect::<Box<[&LogEntry]>>(),
			}
		}
		pub fn draw_day(&self, time: DateTime<FixedOffset>, width: usize) -> String {
			let time = time.with_minute(0).unwrap() + chrono::Duration::hours(1);
			self.slice(time - chrono::Duration::hours(28), time).draw(width)
		}
	}

	impl<'a> IntoIterator for &'a Log {
		type Item = &'a LogEntry;
		type IntoIter = LogIter<'a>;

		fn into_iter(self) -> Self::IntoIter {
			LogIter {
				//index0: true,
				//first: self.first.clone(),
				iter: self.vec.iter(),
			}
		}
	}

	pub struct LogIter<'a> {
		//index0: bool,
		//first: LogEntry,
		iter: ::std::slice::Iter<'a, LogEntry>,
	}

	impl<'a> Iterator for LogIter<'a> {
		type Item = &'a LogEntry;
		fn next(&mut self) -> Option<Self::Item> {
			self.iter.next()/*
			// Trying to figure out how to make a more complex iterator.
			self.index0 = false;
			match self.index0 {
				true => Some(&self.first),
				false => self.iter.next(),
			}// */
		}
	}

	pub struct LogRangeIter<'a> {
		index: usize,
		range: Vec<&'a LogEntry>,
	}

	impl <'a> Iterator for LogRangeIter<'a> {
		type Item = &'a LogEntry;
		fn next(&mut self) -> Option<Self::Item> {
			let result = match self.range.len() {
				x if x > self.index => Some(self.range[self.index]),
				_ => None,
			};
			self.index += 1;
			result
		}
	}

	pub struct LogSliceIter<'a> {
		first: &'a LogEntry,
		slice: Box<[&'a LogEntry]>,
		index: usize,
	}
	impl<'a> Iterator for LogSliceIter<'a> {
		type Item = &'a LogEntry;
		fn next(&mut self) -> Option<Self::Item> {
			let result = match self.index {
				0 => self.first,
				i if i < self.slice.len()+1 => self.slice[i-1],
				_ => return None,
			};
			self.index += 1;
			Some(result)
		}
	}

	#[derive(Debug)]
	pub struct LogSlice<'a> {
		pub start_bound: DateTime<FixedOffset>,
		pub end_bound: DateTime<FixedOffset>,
		first: LogEntry,
		slice: Box<[&'a LogEntry]>,
	}

	impl<'a> LogSlice<'a> {
		fn start(&self) -> DateTime<FixedOffset> {
			self.slice[0].time
			//self.start_bound
		}
		fn end(&self) -> DateTime<FixedOffset> {
			self.end_bound
		}
		fn first(&self) -> &LogEntry {
			self.slice[0]
		}
		//fn last(&self) -> &LogEntry {
		//	self.slice[self.slice.len()]
		//}

		pub fn iter(&self) -> LogSliceIter {
			LogSliceIter {
				first: &self.first,
				slice: self.slice.clone(),
				index: 0,
			}
		}

		pub fn task_at(&self, time: DateTime<FixedOffset>) -> Option<&LogEntry> {
			// Identify the singular task that overlaps with the given datetime.
			if (time < self.start()) | (time >= self.end()) {
				return None;
			}
			let mut task = self.first();
			for entry in self.iter() {
				if entry.time <= time {
					task = entry;
				} else {
					break;
				}
			}
			Some(task)
		}

		pub fn as_string(&self, period: chrono::Duration, scale: f32, delimit: String) -> (Vec<(DateTime<FixedOffset>, String)>, Vec<&str>) {
			fn trim_pad(data: &str, len: usize) -> String {
				// Trim or pad a string with ' ' to make it fit the desired len.
				match data.chars().count() {
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
			fn grapheme_slice(start: usize, end: usize, string: &str) -> String {
				let repr = string.graphemes(true)
					.collect::<Vec<&str>>()
					.to_vec();
				match end {
					0 => repr[start..].join(""),
					n if n > repr.len() => repr[start..].join(""),
					_ => repr[start..end].join(""),
				}
			}
			fn str_width(string: &str) -> usize {
				string.graphemes(true)
					.collect::<Vec<_>>()
					.len()
			}

			// Should refactor to guarante timestamp alignment with the horizontal axis when scaling.
			 /*

			struct EntryDuration {
				span: chrono::Duration,
				kind: String,
				data: String,
				note: String,
			}
			struct Period { vec: EntryDuration }

			let period_len = ;
			let key_set = Vec::<&str>::new();
			let periods = Vec::new();
			// duration, value, kind, note
			periods.push(Period { vec: Vec::new() });

			let mut iter = self.iter();
			let mut prev = ;
			for entry in iter {}
			loop {
				if event_end_time > next_period {

				}
			}
			// */

			let string_cap = (period.num_minutes() as f32 * scale) as usize;
			let mut key_set = Vec::<&str>::new();
			let mut rows = Vec::<_>::new();
			rows.push((self.start_bound, String::new()));
			let mut last_row = &mut rows[0].1;

			let mut iter = self.iter();
			let mut prev = match iter.next() {
				Some(e) => {
					key_set.push(&e.kind);
					e
				},
				None => return (vec![(self.start_bound, "".to_string())], key_set),
			};

			for entry in iter {
				// Compute available width.
				let len = entry.time
					.signed_duration_since(prev.time)
					.num_minutes() as usize;
				let len = (scale * len as f32).round() as usize;

				let mut value: String = match len >= str_width(&delimit) {
					true => [delimit.as_str(), trim_pad(&prev.data, len - str_width(&delimit)).as_str()].concat(),
					false => grapheme_slice(0, len, &delimit),
				};

				// If longer than the remaining duration/width, split on that boundary and append a new string.
				let row_remainder: usize = string_cap - str_width(&last_row);
				match str_width(&value) >= row_remainder {
					true => {
						// 
						last_row.push_str(&grapheme_slice(0, row_remainder, &value));
						value = grapheme_slice(row_remainder, 0, &value).to_string();
						// For long values, iteratively split it into new rows.
						while str_width(&value) >= string_cap {
							rows.push( (rows[rows.len()-1].0+period, grapheme_slice(0, string_cap, &value)) );
							value = grapheme_slice(string_cap, 0, &value);
						}
						// 
						if str_width(&rows[rows.len()-1].1) >= string_cap {
							rows.push( (rows[rows.len()-1].0+period, value.to_string()) );
						}
						let index = rows.len()-1;
						last_row = &mut rows[index].1;
					},
					false => last_row.push_str(&value),
				}
				prev = entry;
			}

			// Append the final entry, covering the remaining width.
			// Computing the remaining space as before.
			let mut value = [delimit.as_str(), &prev.data].concat();
			let row_remainder: usize = string_cap - str_width(&last_row);
			// Filling in that space and/or iterating many lines, as before.
			match str_width(&value) < row_remainder {
				true => {
					value = trim_pad(value.as_str(), row_remainder);
					last_row.push_str(&value);
				},
				false => {
					last_row.push_str(&grapheme_slice(0, row_remainder, &value));
					value = grapheme_slice(row_remainder, 0, &value).to_string();
					// For long values, iteratively split it into new rows.
					while str_width(&value) >= string_cap {
						rows.push( (rows[rows.len()-1].0+period, grapheme_slice(0, string_cap, &value)) );
						value = grapheme_slice(string_cap, 0, &value);
					}
					value = trim_pad(value.as_str(), string_cap);
					rows.push( (rows[rows.len()-1].0+period, value.to_string()) );
				},
			}
			// Fill in blank lines till we pass the slice's end_time.
			while rows[rows.len()-1].0 + period < self.end_bound {
				rows.push( (rows[rows.len()-1].0 + period, " ".repeat(string_cap)) );
			}
			(rows, key_set)
		}

		pub fn draw(&self, width: usize) -> String {
			let day_log = self
				.as_string(chrono::Duration::minutes(60), width as f32/60.0, "▌".to_string())
				.0;
			day_log.iter()
					.map(|row| format!("{:0>2}:▏{}┃", row.0.hour(), row.1))
					.collect::<Vec<_>>()
					.join("\n")
		}
	}
}

// //// Time //// //
mod time {
	use chrono::prelude::*;
	use chrono::Duration;

	pub fn now() -> DateTime<FixedOffset> {
		//DateTime::<FixedOffset>::from_utc(Utc::now().naive_utc(), FixedOffset)
		Utc::now().with_timezone(&FixedOffset::west(7 * 3600))
						.with_second(0).unwrap()
						.with_nanosecond(0).unwrap()
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

	pub fn parse_time(time_str: &str) -> NaiveTime {
		//DateTime::parse_from_str("2019 14 1:46 +0000", "%Y %j %H:%M %z").expect("T475")
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

	pub fn parse_duration(time_str: &str) -> Duration {
		let time_units = parse_time_str(time_str);
		// Convert into minute totals.
		let minutes = match time_units.len() {
			2 => time_units[0]*60 + time_units[1],
			1 => time_units[0],
			_ => panic!("Invalid timestamp--must be of `0:30` or `30` format.")
		};
		Duration::minutes(minutes)
	}

	pub fn map_time_after_datetime(time: NaiveTime, date_frame: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
		// Input time and date and output the earliest datetime matching that time.
		// I.e. `3:00AM` and `Wed 5:00PM` output `Tue, 3:00AM`.
		let time = date_frame.date().and_time(time).expect("Invalid date + time!");
		match date_frame.time() < time.time() {
			true => time,
			false => time + Duration::days(1),
		}
	}
}

// //// Types //// //
pub mod lg_types {
	use chrono::prelude::*;
	use chrono::Duration;
	use super::time::*;
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
		Now(DateTime<FixedOffset>),
		Retcon(DateTime<FixedOffset>),
		Time(DateTime<FixedOffset>),
	}

	#[derive(Debug)]
	pub enum ValidTill {
		Nil,
		For(Duration),
		Till(DateTime<FixedOffset>),
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
				RawInit::Retcon => log.task_at(now()).time,
				/*
				RawInit::Retcon => match log.task_at(now()).kind.as_str() {
					"∅" => {
						//let time = log.task_at(now()).time;
						//log.remove(time);
						//time
						log.task_at(now()).time
					},
					_ => now(),
				},*/
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
/*
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
*/

pub fn execute_command(cmd: lg_types::ValidStatement, log: &mut log::Log) -> Vec<log::LogEntry> {
	use lg_types::ValidInit::{Retcon, Now, Time};
	use lg_types::ValidTill::{Nil, For, Till};
	// Given a retcon signal, should we rewrite the _current_
	// entry no matter what it is or only if it is a nil?
		// Assume the user knows what they're doing. (Rewrite no matter what.)
		// Least surprisal. (Always rewrite.)
		// Minimize cost if the user makes a mistake. (Rewrite only if nil.)
	// Former is easier to code. Do that.

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

	// Record the task.
	let mut vec = Vec::new();
	let data = cmd.data.unwrap();
	vec.push( log::LogEntry::new(time, &data.kind, &data.data, &data.note) );

	// Record the task's endpoint, if specified, deleting the intervening events.
	if let Some(t) = end {
		let task = log.task_at(t);
		if (task.time >= time) & (task.time < t) {
			// Rebuild the last overlapped task
			// at the end and delete the rest.
			let end_task = log::LogEntry::new(t, &task.kind, &task.data, &task.note);
			let slice = log.slice(time, t);
			for log::LogEntry { time, .. } in slice.iter() {
				vec.push(log::LogEntry::empty(*time));
			}
			vec.push(end_task);
		} else {
			// Insert nil at the end.
			vec.push( log::LogEntry::nil(t) );
		}
	}
	vec
}

use chrono::Timelike;
pub fn process_command(cmd: lg_types::RawStatement, log: &mut log::Log) -> Vec<log::LogEntry> {
	use lg_types::ValidStatement;
	use lg_types::ValidInit::{Retcon, Now, Time};
	use lg_types::ValidTill::{Nil, For, Till};
	let cmd = match cmd.compile(log) {
		Some(cmd) => cmd,
		None => panic!("Command didn't compile!"),
	};
	match &cmd {
		// lg [ _ | @ | • ] [ - | + | • ] [ task | '' | • ]
		ValidStatement { init: _, till: _, data: Some(_) } =>
			// lg * * task|''
			execute_command(cmd, log),
		ValidStatement { init: Retcon(time), till: Nil, data: None } => {
			// lg _ • •
			// Print the active and preceding entry.
			let task = log.task_at(*time);
			if let Some(pred) = log.predecessor(task) {
				println!("{:0>2}:{:0>2}-{:0>2}:{:0>2} \t{}: {} - {}",
					pred.time.hour(), pred.time.minute(),
					task.time.hour(), task.time.minute(),
					pred.kind, pred.data, pred.note
				);
			}
			match log.successor(task) {
				Some(succ) => println!("{:0>2}:{:0>2}-{:0>2}:{:0>2} \t{}: {} - {}",
					task.time.hour(), task.time.minute(),
					succ.time.hour(), succ.time.minute(),
					task.kind, task.data, task.note),
				None => println!("{:0>2}:{:0>2}  ...  \t{}: {} - {}",
					task.time.hour(), task.time.minute(),
					task.kind, task.data, task.note),
			}
			Vec::<log::LogEntry>::new()
		},
		ValidStatement { init: Now(time), till: Nil, data: None } => {
			// lg • • •
			// Retrieve and print the day.
			println!("▁▁▁▏:00       :10       :20       :30       :40       :50       ┃");
			println!("{}", log.draw_day(time.clone(), 60));
			println!("▔▔▔▏:00       :10       :20       :30       :40       :50       ┃");
			Vec::<log::LogEntry>::new()
		},
		ValidStatement { init: Time(time), till: Nil, data: None }
		| ValidStatement { init: Retcon(_), till: Till(time), data: None }
		| ValidStatement { init: Now(_), till: Till(time), data: None } =>
			// lg @  • •
			// lg _• - •
			// Set time to nil.
			vec![ log::LogEntry::nil(*time) ],

		ValidStatement { init: Retcon(time), till: For(duration), data: None }
		| ValidStatement { init: Now(time), till: For(duration), data: None } => {
			// lg _• + •
			// Retcon the current--or last not-nil--task to end at time T.
			let mut task = log.task_at(*time);
			while task.is_nil() {
				task = match log.predecessor(task) {
					Some(t) => t,
					None => panic!("No available (not nil) task to retcon!"),
				};
			}
			vec![ log::LogEntry::nil(task.time + *duration) ]
		},
		ValidStatement { init: Time(_), till: For(_), data: None }
		| ValidStatement { init: Time(_), till: Till(_), data: None } => {
			// lg @  +- •
			// Set this timeframe as nil or retrieve and print this timeframe.
			// lg @ +- : // Delete
			// lg @ +- • // View?
			// So this command branch _views,_ if anything.
			println!("Please specify a task name to log.");
			Vec::<log::LogEntry>::new()
		},
	}
}

