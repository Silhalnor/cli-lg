use ::lg::prelude::*;
use lg_types::{ValidData, RawInit, RawTill, RawStatement};
use std::env;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
extern crate unicode_segmentation;
extern crate chrono;
extern crate serde_yaml;
extern crate serde;

fn infer_kind(data: &str) -> Option<String> {
	match data {
		"" => Some("∅".to_string()),
		"chatting" => Some("Leisure".to_string()),
		"sleep" => Some("Sleep".to_string()),
		"slp" => Some("Sleep".to_string()),
		"break" => Some("Rest".to_string()),
		"walk" => Some("Exercise".to_string()),
		"trot" => Some("Exercise".to_string()),
		"breakfast" => Some("Meal".to_string()),
		"lunch" => Some("Meal".to_string()),
		"dinner" => Some("Meal".to_string()),
		"game" => Some("Rest".to_string()),
		"distracted" => Some("Distraction".to_string()),
		data => panic!("Entry is of unknown kind! {}", data),
	}
}




fn print_help() {
	let msg = r#"
	lg [ _ | @ | • ] [ - | + | • ] [ task | '' | • ]

	Input a task and category type/kind to log it immediately.
	lg "Task Name" :MyCategory

	To mark when your task is ending, input an empty task.
	(Or specify a timestamp.)
	lg ''
	lg @12:00
	lg -12:00

	To review the events of the past 24 hours, pass no arguments.
	lg

	Use the `@` flag to specify when it starts.
	Use the `-` or `+` flags to specify when it ends.
	`@13:00` Starts _at_ 13:00.
	`-14:45` Lasts _till_ 14:45.
	`+2:00` Lasts _for_ two hours.
	`+:15` Lasts _for_ fifteen minutes.
		lg Running :Exercise
		lg Running :Exercise @14:00 -14:45
		lg Running :Exercise @14:00 +:45

	The retcon flag, `_`, modifies the currently active event.
	E.g. If you have put in that you are exercising and it
	is now 14:10, this will revise it to end at 15:00.
	Retcon is particularly useful and convenient for making
	corrections since the current event will often--though
	not always--be the last inputted command.
		lg _ -15:00
		lg _ Jogging :Exercise

	Retcon with no additional specifiers simply tell you
	what the current and preceding events _are._

	Specifying a time but an empty category type, just `:`
	with no value, will delete any event starting at that time.
		lg @11:00 :
		lg _ :

	Do note that `@` and `-` specify time periods from 12
	hours in the past and 12 hours in the future. So, if
	it is now midnight, `@7:00` will reference _tomorrow_
	morning, not _this_ morning.
	So, if it is now 17:00 in the afternoon, you can only
	reference between 5:00 this morning to 4:49 tomorrow.
	Likewise, if it's 22:00, you can reference between
	10:00 in the morning to 9:49 tomorrow morning.

"#;
	println!("{}", msg);
}

#[derive(Debug)]
enum CLIFlag {
	Help,
}

#[derive(Debug)]
enum CLIArgType<'a> {
	Flag(CLIFlag),
	Retcon,
	AtTime(&'a str),
	TillTime(&'a str),
	ForTime(&'a str),
	Kind(&'a str),
	Data(&'a str),
}


fn match_arg_type(arg: &str) -> CLIArgType {
	if (arg == "-h") | (arg == "--help") {
		return CLIArgType::Flag(CLIFlag::Help);
	}

	let mut arg_iter = UnicodeSegmentation::graphemes(arg, true);
	let prefix: &str = match arg_iter.next() {
		Some(chr) => chr,
		None => "",
	};
	let term: &str = arg_iter.as_str();
	match prefix {
		"_" => CLIArgType::Retcon,
		"@" => CLIArgType::AtTime(term),
		"-" => CLIArgType::TillTime(term),
		"+" => CLIArgType::ForTime(term),
		":" => CLIArgType::Kind(term),
		_ => CLIArgType::Data(arg),
	}
}

fn parse_commit_args<'a>(args: Vec<&str>) -> Option<RawStatement> {
	//use lg_types::{RawStatement, RawInit, RawTill};
	let mut init = RawInit::Now;
	let mut till = RawTill::Nil;
	let mut data: Option<String> = None;
	let mut kind: Option<String> = None;
	let mut note = Vec::<&str>::new();

	// Parse arguments into their appropriate types.
	for arg in args {
		match match_arg_type(arg) {
			CLIArgType::Flag(CLIFlag::Help) => {print_help(); return None;},
			CLIArgType::Retcon if init != RawInit::Now => panic!("Retcon \"_\" flag already used!"),
			CLIArgType::AtTime(t) if init != RawInit::Now => panic!("\"@00:00\" or retcon \"_\" flag already used! @{}", t),
			CLIArgType::TillTime(t) if till != RawTill::Nil => panic!("\"+/-\" flag already used! -{}", t),
			CLIArgType::ForTime(t) if till != RawTill::Nil => panic!("\"+/-\" flag already used! +{}", t),
			CLIArgType::Kind(k) if kind.is_some() => panic!("Entry kind already specified! :{}", k),
			CLIArgType::Data(d) if data.is_some() => note.push(d),
			CLIArgType::Retcon => init = RawInit::Retcon,//InstrInit::RetNone,
			CLIArgType::AtTime(t) => init = RawInit::Time(t.to_string()),//InstrInit::Time(parse_time(t)),
			CLIArgType::TillTime(t) => till = RawTill::Till(t.to_string()),//InstrTill::Halt(parse_time(t)),
			CLIArgType::ForTime(t) => till = RawTill::For(t.to_string()),//InstrTill::Span(parse_duration(t)),
			CLIArgType::Kind(k) => kind = Some(k.to_string()),
			CLIArgType::Data(d) => data = Some(d.to_string()),
		}
	}

	// Data-kind-note validity check.
	// If data is available at all, both kind and data must be present.
	if let (None, Some(d)) = (&kind, &data) {
		kind = infer_kind(d);
	}

	let note = note.join(" ");

	let data: Option<ValidData> = match (kind, data, note.len() > 0) {
		(None, None, false) => None,
		(Some(kind), Some(data), _) => Some(ValidData { kind, data, note }),
		(Some(ref kind), None, _) if kind == "" => Some(ValidData { kind: "".to_string(), data: "".to_string(), note }),
		(Some(_), None, _) => panic!("No data provided."),
		(_, Some(_), _) => panic!("No kind provided."),
		(None, None, true) => panic!("Notes provided but no kind nor data."),
	};
	Some(RawStatement { init, till, data })
}

fn read_log(file_path: &str) -> HashMap<String, Log> {
	// Serde load; ensure correctly sorted
	let file_path = std::path::Path::new(file_path);
	let file = std::fs::File::open(file_path).expect("Log file not found.");
	let min_log: HashMap<String, Vec<LogEntry>> = serde_yaml::from_reader(file).expect("Log file is invalid.");
	//let min_log = HashMap::new();//HashMap<String, Vec<LogEntry>>
	//load minlog
	let mut full_log = HashMap::new();//HashMap<String, Log>
	for (key, log) in min_log {
		full_log.insert(key.to_string(), ::lg::log::Log::new());
		for entry in log {
			let time = entry.time;
			let data = entry.data;
			let kind = entry.kind;
			let note = entry.note;
			full_log.get_mut(&key)
				.unwrap()
				.add(time, data, kind, note);
		}
	}
	full_log
}

fn record_log(file_path: &str, log_set: HashMap<String, &::lg::log::Log>) {
	let mut min_log = HashMap::new();
	for (key, log) in log_set {
		min_log.insert(key, &log.vec);
	}

	let file_path = std::path::Path::new(file_path);
	let file = std::fs::File::create(file_path).expect("Log file not found.");
	if let Err(_err) = serde_yaml::to_writer(file, &min_log) {
		panic!("Log serialization failed.");
	}
}

/*
fn new_test_log() -> Log {let ltr1 = "1".to_string();
	let sz2 = chrono::Duration::minutes(2);
	let sz3 = chrono::Duration::minutes(3);
	let mut log = Log::new();

	log.add(DateTime::parse_from_str("2019 14 09:00 +0000", "%Y %j %H:%M %z").expect("T472"), "A".to_string(), "B".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 09:20 +0000", "%Y %j %H:%M %z").expect("T472"), "UVXYZ".to_string(), "Y".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 10:00 +0000", "%Y %j %H:%M %z").expect("T472"), "QWERT".to_string(), "Q".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 10:40 +0000", "%Y %j %H:%M %z").expect("T472"), "12345".to_string(), "1".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 11:00 +0000", "%Y %j %H:%M %z").expect("T472"), "67890".to_string(), "6".to_string(), ltr1.clone());
	log
}*/

/*
	lg			# Alias for lg hr 24.
	lg day		# Alias for lg day 28.
	lg hr 24	# A day
	lg day 28	# 4 weeks
	lg week 24	# 6 months
	lg day Feb4-Feb10	# 7 days. Assumes this year.
	lg day 19Feb4-19Feb10
	lg day 4-10
	lg hr 14:37-15:00
	lg hr :37-:00
	lg hr 14-16
*/

fn main() {
	let log_path = "/home/lemma/lglog.yml";
	let mut log = read_log(log_path)
		.remove("Lemma")
		.unwrap();
	match parse_commit_args(env::args().skip(1)
									   .collect::<Vec<String>>()
									   .iter()
									   .map(AsRef::as_ref)
									   .collect::<Vec<&str>>()) {
		Some(cmd) => {
			let cmd = process_command(cmd, &mut log);
			for entry in cmd {
				//println!("{:#?}", entry);
				log.update(entry);
			}
			let mut log_map = HashMap::new();
			log_map.insert("Lemma".to_string(), &log);
			record_log(log_path, log_map);
		},
		None => (),
	}
}
