use ::lg::prelude::*;
use lg_types::{ValidData, RawInit, RawTill, RawStatement};
use std::env;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use chrono::prelude::*;
use serde::{Serialize, Deserialize};
//use chrono::Duration;
extern crate unicode_segmentation;
extern crate chrono;
extern crate serde_yaml;
extern crate serde;

/*
fn test_log() {
	let ltr1 = "1".to_string();
	let sz2 = chrono::Duration::minutes(2);
	let sz3 = chrono::Duration::minutes(3);
	let mut log = Log::new();
	// Minute lengths: 2 3 2 4 2
	log.add(DateTime::parse_from_str("2019 14 09:00 +0000", "%Y %j %H:%M %z").expect("T472"), "A".to_string(), "B".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 09:20 +0000", "%Y %j %H:%M %z").expect("T472"), "UVXYZ".to_string(), "Y".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 10:00 +0000", "%Y %j %H:%M %z").expect("T472"), "QWERT".to_string(), "Q".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 10:40 +0000", "%Y %j %H:%M %z").expect("T472"), "12345".to_string(), "1".to_string(), ltr1.clone());
	log.add(DateTime::parse_from_str("2019 14 11:00 +0000", "%Y %j %H:%M %z").expect("T472"), "67890".to_string(), "6".to_string(), ltr1.clone());

	let mut beta_log = Log::new();
	// Length 1
	beta_log.add(DateTime::parse_from_str("2019 14 9:00 +0000", "%Y %j %H:%M %z").expect("T472"), "^".to_string(), ltr1.clone(), ltr1.clone());
	beta_log.add(DateTime::parse_from_str("2019 14 10:00 +0000", "%Y %j %H:%M %z").expect("T472"), "*".to_string(), ltr1.clone(), ltr1.clone());
	beta_log.add(DateTime::parse_from_str("2019 14 11:00 +0000", "%Y %j %H:%M %z").expect("T472"), "^".to_string(), ltr1.clone(), ltr1.clone());

	//println!("Alpha: {}", log.as_string(0.15, "|".to_string()).0);
	//println!("Beta : {}", beta_log.as_string(1.0/60.0, "".to_string()).0);

	let width = 20;
	let a_log = log.as_string(width as f32/60.0, "|".to_string()).0;
	let b_log = beta_log.as_string(1.0/60.0, "".to_string()).0;

	let vec_logs = vec!( (a_log, width), (b_log, 1) );
	let joined_log = join_logs(vec_logs);
	println!("\"\n{}\"", joined_log);
	for entry in log.iter() {
		println!(":{} : {}", entry.time, entry.data);
	}
}*/



fn infer_kind(data: &str) -> Option<String> {
	match data {
		"" => Some("∅".to_string()),
		"chatting" => Some("leisure".to_string()),
		data => panic!("Entry is of unknown kind! {}", data),
	}
}



// //// Instruction Definition //// //

#[derive(Debug)]
struct RetrievePeriod {
	start: DateTime<FixedOffset>,
	end: DateTime<FixedOffset>,
	period: DateTime<FixedOffset>, //Range. The span each onscreen line is to take up. Or
	// other metric such as 24-hr blocks if we're printing a 30-day 7x5 calendar.
}

#[derive(Debug, PartialEq)]
enum InstrInit {
	Now(DateTime<FixedOffset>),
	Retcon(DateTime<FixedOffset>),
	RetNone,
	Time(DateTime<FixedOffset>),
}

#[derive(Debug, PartialEq)]
enum InstrTill {
	None,
	Span(chrono::Duration),
	Halt(DateTime<FixedOffset>),
}

#[derive(Debug)]
struct InstrEvent {
	init: InstrInit,
	till: InstrTill,
	data: Option<ValidData>,
}

fn fill_instruction(init: InstrInit, till: InstrTill, data: ValidData) -> Vec<LogEntry> {
	//if let InstrInit::RetNone = init {}
	//else
	//let init_time = init;
	let init_time: DateTime<FixedOffset> = match init {
		InstrInit::Now(now) => now,
		InstrInit::RetNone => DateTime::parse_from_str("2000 1 0:00 -0700", "%Y %j %H:%M %z").expect("T473"), // Get time of currently active event.
		InstrInit::Time(time) => time,
		InstrInit::Retcon(time) => time,//Get timestamp of currently active task.
	};
	let mut vec = Vec::<LogEntry>::new();
	vec.push(LogEntry {
		time: init_time,
		data: data.data,
		kind: data.kind,
		note: data.note,
	} );
	match till {
		InstrTill::None => (),
		InstrTill::Span(span) => vec.push(LogEntry {
			time: init_time + span, // Does span need to be an explicit range type? To be converted to a time?
			data: "".to_string(),
			kind: "∅".to_string(),
			note: "".to_string(),
		}),
		InstrTill::Halt(t) => vec.push(LogEntry {
			time: t,
			data: "".to_string(),
			kind: "∅".to_string(),
			note: "".to_string(),
		}),
	}
	// If vec.len == 2 & vec[1].time > vec[0].time { "Increment vec[1] by 1 day, keeping the same hour." }
	vec
}

fn manage_instruction_branches(instr: InstrEvent) -> (Option<RetrievePeriod>, Vec<LogEntry>) {
	match instr {
		InstrEvent { init, till, data: Some(data) } =>
			( None, fill_instruction(init, till, data) ),
		InstrEvent { init: InstrInit::Retcon(_), till: InstrTill::None, data: None } =>
			unimplemented!("Set the currently active entry to nil or retrieve and print it?"),
		InstrEvent { init: InstrInit::Time(_), till: InstrTill::None, data: None } =>
			unimplemented!("Set time to nil."),
		InstrEvent { init: InstrInit::Now(_), till: InstrTill::None, data: None } =>
			unimplemented!("Retrieve and print the day."),
		InstrEvent { init: InstrInit::Retcon(_), till: InstrTill::Span(_), data: None }
		| InstrEvent { init: InstrInit::Retcon(_), till: InstrTill::Halt(_), data: None }
		| InstrEvent { init: InstrInit::Now(_), till: InstrTill::Span(_), data: None }
		| InstrEvent { init: InstrInit::Now(_), till: InstrTill::Halt(_), data: None } =>
		//| InstrEvent { init: (InstrInit::Retcon(_), InstrInit::Now(_)), till: (InstrTill::Span(_), InstrTill::Halt(_)), data: None } =>
			unimplemented!("Retroactively end the current task at time T. (Warn if nil.)"),
		InstrEvent { init: InstrInit::Time(_), till: InstrTill::Span(_), data: None }
		| InstrEvent { init: InstrInit::Time(_), till: InstrTill::Halt(_), data: None } =>
			unimplemented!("Set this timeframe as nil or retrieve and print this timeframe."),
		InstrEvent { init: InstrInit::RetNone, .. } =>
			panic!("Unexpected RetNone received in `manage_instruction_branches`!"),
	}
}


#[derive(Debug)]
enum CLIArgType<'a> {
	Retcon,
	AtTime(&'a str),
	TillTime(&'a str),
	ForTime(&'a str),
	Kind(&'a str),
	Data(&'a str),
}


fn match_arg_type(arg: &str) -> CLIArgType {
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

fn parse_commit_args<'a>(args: Vec<&str>) -> RawStatement {
	use lg_types::{RawStatement, RawInit, RawTill};
	let mut init = RawInit::Now;
	let mut till = RawTill::Nil;
	let mut data: Option<String> = None;
	let mut kind: Option<String> = None;
	let mut note = Vec::<&str>::new();

	// Parse arguments into their appropriate types.
	for arg in args {
		match match_arg_type(arg) {
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
		(Some(_), _, _) => panic!("No data provided."),
		(_, Some(_), _) => panic!("No kind provided."),
		(None, None, true) => panic!("Notes provided but no kind nor data."),
	};
	RawStatement { init, till, data }
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
	if let Err(err) = serde_yaml::to_writer(file, &min_log) {
		panic!("Log serialization failed.");
	}
}

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
}

fn main() {
	let mut log = read_log("/home/lemma/Documents/lg/serde_test.yml")
		.remove("Lemma")
		.unwrap();
	// Manual arg parsing
	/*let (log_period, entries) = match argparse(last_lemma_stamp) {
		cli_call::Retrieval => (),
		cli_call::Commit(c) => manage_instruction_branches(c),
	};*/
	//let log = new_test_log();
	let cmd = parse_commit_args(env::args().skip(1)
										   .collect::<Vec<String>>()
										   .iter()
										   .map(AsRef::as_ref)
										   .collect::<Vec<&str>>());
	//let cmd = compile_command(cmd);
	let cmd = process_command(cmd, &mut log);
	println!("{:?}", cmd);
	for entry in cmd {
		log.push(entry);
	}
	let mut log_map = HashMap::new();
	log_map.insert("Lemma".to_string(), &log);
	record_log("/home/lemma/Documents/lg/serde_test.yml", log_map);
	//for e in entries {
	//	println!("e. {}", e);
	//}

	/* clap arg parsing
	let yaml = load_yaml!("cli.yml");
	let matches = App::from_yaml(yaml).get_matches();
	let data: String = match matches.value_of("test") {
		Some(x) => x.to_string(),
		None => "".to_string(),
	};
	println!("CLI: {}", data);
	*/

	//let test_val = test_histogram();
	//println!("{}", test_val);
	//test_log();
}
