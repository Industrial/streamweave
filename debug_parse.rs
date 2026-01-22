extern crate chrono;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

fn main() {
  let time_str = "2024-01-15";
  let format_str = "%Y-%m-%d";

  println!(
    "Testing parse of '{}' with format '{}'",
    time_str, format_str
  );

  // Try DateTime::parse_from_str
  match DateTime::parse_from_str(time_str, format_str) {
    Ok(dt) => {
      let timestamp = dt.timestamp_millis();
      println!("DateTime::parse_from_str succeeded: {}", timestamp);
    }
    Err(e) => println!("DateTime::parse_from_str failed: {}", e),
  }

  // Try NaiveDateTime::parse_from_str
  match NaiveDateTime::parse_from_str(time_str, format_str) {
    Ok(ndt) => {
      let datetime = Utc.from_utc_datetime(&ndt);
      let timestamp = datetime.timestamp_millis();
      println!("NaiveDateTime::parse_from_str succeeded: {}", timestamp);
    }
    Err(e) => println!("NaiveDateTime::parse_from_str failed: {}", e),
  }

  // Try common formats
  let common_formats = vec![
    "%Y-%m-%dT%H:%M:%S%.3fZ",
    "%Y-%m-%dT%H:%M:%SZ",
    "%Y-%m-%d %H:%M:%S",
    "%Y/%m/%d %H:%M:%S",
    "%Y-%m-%d",
    "%H:%M:%S",
  ];

  for fmt in common_formats {
    if let Ok(ndt) = NaiveDateTime::parse_from_str(time_str, fmt) {
      let datetime = Utc.from_utc_datetime(&ndt);
      let timestamp = datetime.timestamp_millis();
      println!("Common format '{}' succeeded: {}", fmt, timestamp);
    } else if let Ok(dt) = DateTime::parse_from_str(time_str, fmt) {
      let timestamp = dt.timestamp_millis();
      println!(
        "Common format '{}' (DateTime) succeeded: {}",
        fmt, timestamp
      );
    }
  }
}
