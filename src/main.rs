// vim: set sw=4 ts=4 et :
extern crate chrono;
extern crate rust_decimal;
extern crate serde_json;

use std::cmp::max;
use std::collections::{HashMap, BTreeMap};
use std::convert::TryInto;
use std::fmt;
use std::io::{self, BufRead};

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use chrono::prelude::*;

const WEEKDAYS: usize = 7;

#[derive(Serialize, Deserialize, Debug)]
struct Inclusion {
    id: i32,
    start: String,
    #[serde(default)]
    end: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    annotation: String,
}

#[derive(Debug)]
pub struct Interval {
    project: String,
    total_seconds: i64,
    weekday: u32,
    inclusion: Option<Inclusion>,
}

impl Interval {
    pub fn new() -> Interval {
        Interval {
            project: "".to_string(),
            total_seconds: 0,
            weekday: 0,
            inclusion: None,
        }
    }

    pub fn total_seconds(&self) -> i64 {
        self.total_seconds
    }

    pub fn add_seconds(&mut self, seconds: i64) {
        self.total_seconds += seconds
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn weekday(&self) -> u32 {
        self.weekday
    }
}

pub struct IntervalFactory {
    valid_projects: Option<Vec<String>>,
}

impl IntervalFactory {
    pub fn new() -> IntervalFactory {
        IntervalFactory {
            valid_projects: None,
        }
    }

    pub fn parse_projects(&mut self, json: &str) -> Option<String> {
        let result: Result<Value, serde_json::Error> = serde_json::from_str(json);
        match result {
            Ok(json) => {
                let mut projects: Vec<String> = Vec::new();
                let json_array = match json.as_array() {
                    Some(array) => array,
                    None => {
                        return Some("not a valid JSON array".to_string());
                    },
                };
                for i in json_array.to_vec() {
                    projects.push(i.to_string().trim_matches('"').to_string());
                }
                self.valid_projects = Some(projects);
            }
            Err(error) => return Some(format!("Failed to parse projects: {}", error)),
        }
        None
    }

    pub fn string_to_datetime(datetime: &str) -> Result<DateTime<Local>, String> {
        if datetime == "" {
            return Ok(Local::now());
        }

        let dt_format = "%Y%m%dT%H%M%SZ";
        let result = NaiveDateTime::parse_from_str(datetime, dt_format);
        let start = match result {
            Ok(start) => DateTime::<Utc>::from_utc(start, Utc),
            Err(error) => return Err(error.to_string()),
        };
        Ok(start.with_timezone(&Local))
    }

    pub fn new_interval(&self, raw_json: &str) -> Result<Interval, String> {
        let valid_projects = match &self.valid_projects {
            Some(projects) => {
                if projects.len() == 0 {
                    return Err("No projects were defined in config.".to_string());
                } else {
                    projects
                }
            }
            None => return Err("No projects were defined in config.".to_string()),
        };

        let inclusion: Inclusion = match serde_json::from_str(raw_json) {
            Ok(inclusion) => inclusion,
            Err(error) => return Err(error.to_string()),
        };

        let mut project: &str = "";
        for tag in &inclusion.tags {
            if valid_projects.contains(&tag) {
                if project == "" {
                    project = tag;
                } else {
                    return Err("More than one valid project found.".to_string());
                }
            }
        }

        if project == "" {
            return Err("Failed to find a valid project.".to_string());
        }

        let start = IntervalFactory::string_to_datetime(&inclusion.start)?;
        let end = IntervalFactory::string_to_datetime(&inclusion.end)?;
        let total_seconds = end.signed_duration_since(start).num_seconds();

        Ok(Interval {
            project: project.to_string(),
            total_seconds: total_seconds,
            weekday: start.weekday().num_days_from_monday(),
            inclusion: Some(inclusion),
        })
    }
}

type RowT = Vec<Decimal>;

#[derive(Debug)]
pub struct Report {
    data: BTreeMap<String, RowT>,
    totals: RowT,
    column_width: usize,
    tag_width: usize,
}

impl Report {
    pub fn from_intervals(_options: &HashMap<String, String>, intervals: &Vec<Interval>) -> Report {
        // Sum up the intervals into total seconds per project / per day
        let mut raw_data: BTreeMap<String, Vec<i64>> = BTreeMap::new();
        for interval in intervals {
            let project_data = raw_data
                .entry(String::from(&interval.project))
                .or_insert(vec![0; WEEKDAYS]);

            let weekday: usize = interval.weekday().try_into().unwrap();
            project_data[weekday] += interval.total_seconds();
        }

        let seconds_per_hour = Decimal::new(3600, 0);

        let mut data: BTreeMap<String, RowT> = BTreeMap::new();
        let mut totals: RowT = vec![Decimal::new(0, 0); WEEKDAYS + 1];
        // Convert the raw seconds into hours and 10ths of hours, and sum up the
        // totals
        let mut tag_width: usize = 0;
        for (key, value) in &raw_data {
            tag_width = max(tag_width, key.len());
            let project_data = data
                .entry(String::from(key))
                .or_insert(vec![Decimal::new(0, 0); WEEKDAYS + 1]);

            let mut project_total = Decimal::new(0, 0);
            for weekday in 0..value.len() {
                project_data[weekday] =
                    (Decimal::new(value[weekday], 0) / seconds_per_hour).round_dp_with_strategy(1, RoundingStrategy::RoundHalfUp);
                project_total += project_data[weekday];
                totals[weekday] += project_data[weekday];
            }
            project_data[WEEKDAYS] = project_total;
        }

        let mut total = Decimal::new(0, 0);
        for weekday in 0..WEEKDAYS {
            total += totals[weekday];
        }
        totals[WEEKDAYS] = total;
        tag_width = max(tag_width, "totals".len());

        Report {
            data: data,
            totals: totals,
            column_width: 6,
            tag_width: tag_width,
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn write_project(report: &Report, f: &mut fmt::Formatter, project: &str, data: &RowT) -> fmt::Result {
            write!(f, "{:<0width$} |", project, width = report.tag_width)?;
            let zero = Decimal::new(0, 0);
            for val in data {
                if val == &zero {
                    write!(f, " {:>width$} |", " ", width = report.column_width)?;
                } else {
                    write!(f, " {:>width$} |", val, width =report.column_width)?;
                }
            }
            write!(f, "\n")
        }

        let separator = format!(
            "{}=|{}",
            "=".repeat(self.tag_width),
            format!("={}=|", "=".repeat(self.column_width)).repeat(WEEKDAYS + 1)
        );

        write!(f, "{} | ", " ".repeat(self.tag_width))?;
        for day in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Tot"].iter() {
            write!(f, "{:>0width$} | ", day, width = self.column_width)?;
        }
        write!(f, "\n{}\n", separator)?;

        for (key, value) in &self.data {
            write_project(self, f, &key, &value)?;
        }

        write!(f, "{}\n", separator)?;

        write_project(self, f, "totals", &self.totals)
    }
}

fn main() -> Result<(), io::Error> {
    let mut options_finished = false;
    let mut intervals: Vec<Interval> = Vec::new();
    let mut factory: IntervalFactory = IntervalFactory::new();
    let mut options: HashMap<String, String> = HashMap::new();

    let mut parse_error = false;
    for _line in io::stdin().lock().lines() {
        let line = _line.unwrap();
        if line == "[" {
            options_finished = true;
            match options.get("timesheet.projects") {
                Some(projects) => match factory.parse_projects(projects) {
                    Some(error) => {
                        eprintln!("Failed to parse timesheet.projects {} {}", projects, error);
                        parse_error = true;
                        break;
                    },
                    None => (),
                },
                None => { 
                    eprintln!("Failed to find timesheet.projects configuration item.");
                    parse_error = true;
                    break;
                },
            }
        } else if line != "" && line != "]" {
            if options_finished {
                let raw_json = line.trim_matches(',');
                let result = factory.new_interval(&raw_json);
                match result {
                    Ok(interval) => intervals.push(interval),
                    Err(error) => {
                        parse_error = true;
                        eprintln!("Failed to read {} {}", raw_json, error);
                    },
                }
            } else {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                let key = parts[0].trim().trim_matches(':');
                let value = parts[1].trim();
                options.insert(String::from(key), String::from(value));
            }
        }
    }

    if parse_error == true {
        std::process::exit(1);
    }

    let report = Report::from_intervals(&options, &intervals);
    println!("{}", report);

    Ok(())
}
