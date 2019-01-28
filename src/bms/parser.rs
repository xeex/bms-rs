use crate::bms::{
    format::{BmsBuilder, BMS},
    timeline::TimelineEvent,
    Alphanumeric, ObjType, Object,
};
use encoding::{all::UTF_8, DecoderTrap, Encoding};
use regex::Regex;
use std::{fs::File, io::Read, str::FromStr};

const METADATA_HEADERS: [&'static str; 8] = [
    "PLAYER",
    "GENRE",
    "TITLE",
    "ARTIST",
    // "BPM",
    "PLAYLEVEL",
    "RANK",
    "TOTAL",
    "STAGEFILE",
];

pub struct BmsParser;

/// Temporary BPM struct
#[derive(Debug)]
pub struct BPM {
    measure: f32,
    bpm: f32,
}

#[derive(Debug)]
pub struct STOP {
    measure: f32,
    stop_val: f32, // Most #STOP values are integers
}

impl BmsParser {
    /// Parses the file
    /// TODO: Make a better documentation
    pub fn parse(&self, file: &mut File) -> BMS {
        let mut bms_contents = Vec::new();
        file.read_to_end(&mut bms_contents)
            .expect("File reading error");

        let mut parsers: Vec<Box<BmsLineParser>> = Vec::new();
        parsers.push(Box::new(MetadataParser {}));
        parsers.push(Box::new(WavParser::new()));
        parsers.push(Box::new(BgaParser::new()));
        parsers.push(Box::new(BpmParser::new()));
        parsers.push(Box::new(ObjParser::new()));
        parsers.push(Box::new(StopParser::new()));

        let mut bms_builder = BmsBuilder::new();
        for line in UTF_8
            .decode(&bms_contents, DecoderTrap::Replace)
            .expect("Could not decode line in UTF-8")
            .lines()
        {
            for line_parser in parsers.iter() {
                if line_parser.parse_line_into_bms(&line.into(), &mut bms_builder) {
                    break;
                }
            }
        }
        let bms = bms_builder.build();

        bms
    }

    /// Parses the title of the chart from the given file.
    /// TODO: Make a better documentation
    fn parse_title(&self, file: &mut File) -> Option<String> {
        let mut bms_contents = Vec::new();
        file.read_to_end(&mut bms_contents)
            .expect("File reading error");

        let metadata_parser: MetadataParser = MetadataParser {};
        for line in UTF_8
            .decode(&bms_contents, DecoderTrap::Replace)
            .expect("Could not decode line in UTF-8")
            .lines()
        {
            let result = metadata_parser.parse_line(&line.into());
            if result.is_none() {
                continue;
            }

            let value = result.unwrap();
            if "TITLE".eq(&value.0) {
                return Option::from(value.1);
            }
        }
        Option::None
    }
}

trait BmsLineParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool;
    fn parse_line(&self, line: &String) -> Option<(String, String)>;
}

struct MetadataParser;

impl BmsLineParser for MetadataParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let result = self.parse_line(line);
        if result.is_none() {
            return false;
        }

        let value = result.unwrap();
        bms_builder.with_metadata(value.0, value.1);
        true
    }

    fn parse_line(&self, line: &String) -> Option<(String, String)> {
        if !line.starts_with("#") {
            return Option::None;
        }

        let entry = &line[1..];
        for header in METADATA_HEADERS.iter() {
            if entry.starts_with(header) {
                let keydata = (header.to_string(), entry[header.len()..].trim().to_string());
                return Option::from(keydata);
            }
        }
        Option::None
    }
}

struct WavParser {
    regex_parser: Regex,
}

impl WavParser {
    pub fn new() -> WavParser {
        WavParser {
            regex_parser: Regex::new(r"#WAV(?P<key>.+?) (?P<data>.*)")
                .expect("Could not initialize regex"),
        }
    }
}

impl BmsLineParser for WavParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let result = self.parse_line(line);
        if result.is_none() {
            return false;
        }

        let value = result.unwrap();
        let key = Alphanumeric::from_str(&value.0);
        bms_builder.with_keysound(key, value.1);
        true
    }

    fn parse_line(&self, line: &String) -> Option<(String, String)> {
        let capture_res = self.regex_parser.captures(line);
        if capture_res.is_none() {
            return Option::None;
        }

        let res = capture_res.unwrap();
        let (key, data): (&str, &str) = (&res["key"], &res["data"]);
        let keydata: (String, String) = (String::from(key.trim()), String::from(data.trim()));
        Option::from(keydata)
    }
}

struct BgaParser {
    regex_parser: Regex,
}

impl BgaParser {
    pub fn new() -> BgaParser {
        BgaParser {
            regex_parser: Regex::new(r"#BMP(?P<key>.+?) (?P<data>.*)")
                .expect("Could not initialize regex"),
        }
    }
}

impl BmsLineParser for BgaParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let result = self.parse_line(line);
        if result.is_none() {
            return false;
        }

        let value = result.unwrap();
        let key = Alphanumeric::from_str(&value.0);
        bms_builder.with_bga_layer(key, value.1);
        true
    }

    fn parse_line(&self, line: &String) -> Option<(String, String)> {
        let capture_res = self.regex_parser.captures(line);
        if capture_res.is_none() {
            return Option::None;
        }

        let res = capture_res.unwrap();
        let (key, data): (&str, &str) = (&res["key"], &res["data"]);
        let keydata: (String, String) = (String::from(key.trim()), String::from(data.trim()));
        Option::from(keydata)
    }
}

struct BpmParser {
    regex_parser: Regex,
}

impl BpmParser {
    pub fn new() -> BpmParser {
        BpmParser {
            // Should've been  "(?P<key>.{0}|.{2})", but it doesn't seem to work. Let's make do with 0,2 length.
            regex_parser: Regex::new(r"#BPM(?P<key>.{0,2}) (?P<data>.*)")
                .expect("Could not initialize regex"),
        }
    }
}

impl BmsLineParser for BpmParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let result = self.parse_line(line);
        if result.is_none() {
            return false;
        }

        let value = result.unwrap();
        let key = Alphanumeric::from_str(&value.0);
        let bpm = f32::from_str(&value.1).expect("Invalid data found on BPM header.");
        if key.key == 0 {
            bms_builder.timeline_builder.with_base_bpm(bpm);
        } else {
            bms_builder.with_bpm(key, bpm);
        }
        true
    }

    fn parse_line(&self, line: &String) -> Option<(String, String)> {
        let capture_res = self.regex_parser.captures(line);
        if capture_res.is_none() {
            return Option::None;
        }

        let res = capture_res.unwrap();
        let (key, data): (&str, &str) = (&res["key"], &res["data"]);

        let keydata: (String, String) = (String::from(key.trim()), String::from(data.trim()));
        Option::from(keydata)
    }
}

struct ObjParser {
    regex_parser: Regex,
}

impl ObjParser {
    pub fn new() -> ObjParser {
        ObjParser {
            regex_parser: Regex::new(r"#(?P<measure>[0-9]{3})(?P<channel>[0-9]{2}):(?P<data>.*)")
                .expect("Could not initialize regex"),
        }
    }
}

impl BmsLineParser for ObjParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let capture_res = self.regex_parser.captures(line);
        if capture_res.is_none() {
            return false;
        }

        let res = capture_res.unwrap();
        let (measure, channel, data): (u32, u32, &str) = (
            res["measure"].parse::<u32>().unwrap(),
            res["channel"].parse::<u32>().unwrap(),
            &res["data"].trim(),
        );

        match channel {
            1 | 11 | 12 | 13 | 14 | 15 | 16 | 18 | 19 | 4 => {
                // Autoplay + played notes + BGA
                let mut iter = 0;
                while iter < data.len() {
                    let n_measure: f32 = measure as f32 + (iter as f32) / data.len() as f32;
                    let n_keysound = Alphanumeric::from_str(&data[iter..iter + 2]);
                    // Prepare. Take note iter has been changed here!
                    iter += 2;

                    // Ignore "placeholder" notes
                    if n_keysound.key == 0 {
                        continue;
                    }

                    let objtype = match channel {
                        4 => ObjType::BGA(n_keysound),
                        1 => ObjType::Auto(n_keysound),
                        _ => ObjType::Note(n_keysound),
                    };

                    let obj = Object {
                        time: 0,
                        measure: n_measure,
                        channel,
                        objtype,
                        hit_offset: None,
                        longnote_hit_offset: None,
                    };

                    bms_builder.add_object(obj);
                }
                return true;
            }
            3 | 8 => {
                // BPM changes
                let mut iter = 0;
                while iter < data.len() {
                    let bpm_measure: f32 = measure as f32 + (iter as f32) / data.len() as f32;
                    let bpm_value: f32 = if channel == 3 {
                        // Channel 3; parse hexadecimal value directly
                        f32::from(u16::from_str_radix(&data[iter..iter + 2], 16).unwrap())
                    } else {
                        // Channel 8; get BPM value from mapping
                        bms_builder
                            .timeline_builder
                            .find_bpm(Alphanumeric::from_str(&data[iter..iter + 2]))
                    };
                    if (bpm_value - 0_f32).abs() > 0.000_001_f32
                    /* sane float comparison */
                    {
                        bms_builder.timeline_builder.with_event(TimelineEvent::BPM {
                            measure: bpm_measure,
                            bpm: bpm_value,
                        });
                    }
                    iter += 2;
                }
                return true;
            }
            9 => {
                // STOP command
                let mut iter = 0;
                while iter < data.len() {
                    let stop_measure: f32 = measure as f32 + (iter as f32) / data.len() as f32;
                    let stop_val: f32 = bms_builder
                        .timeline_builder
                        .find_stop(Alphanumeric::from_str(&data[iter..iter + 2]));
                    if stop_val != 0_f32 {
                        bms_builder
                            .timeline_builder
                            .with_event(TimelineEvent::STOP {
                                measure: stop_measure,
                                duration: stop_val,
                            });
                    }
                    iter += 2;
                }
                return true;
            }
            2 => {
                // Measure length
                bms_builder
                    .timeline_builder
                    .with_measure_len(measure, f32::from_str(data).unwrap());
                return true;
            }
            _ => return false,
        }
        // let result = self.parse_line(line);
        // if result.is_none() {
        //     return false
        // }

        // let value = result.unwrap();
        // let key = u16::from_str_radix(&value.0, 36).unwrap_or(0);//("Invalid key found on BPM header.");
        // let bpm = f32::from_str(&value.1).expect("Invalid data found on BPM header.");
        // if key == 0 {
        //     // Not sure if necessary, but for completeness sake
        //     bms_builder.with_metadata("BPM".to_string(), value.1);
        // }
        // bms_builder.with_bpm(key, bpm);
        // true
    }

    fn parse_line(&self, _line: &String) -> Option<(String, String)> {
        // INIMPLEMENTED!!! Bad method.
        Option::None
    }
}

struct StopParser {
    regex_parser: Regex,
}

impl StopParser {
    pub fn new() -> StopParser {
        StopParser {
            regex_parser: Regex::new(r"#STOP(?P<key>.{0,2}) (?P<data>.*)")
                .expect("Could not initialize regex"),
        }
    }
}

impl BmsLineParser for StopParser {
    fn parse_line_into_bms(&self, line: &String, bms_builder: &mut BmsBuilder) -> bool {
        let result = self.parse_line(line);
        if result.is_none() {
            return false;
        }

        let value = result.unwrap();
        let key = Alphanumeric::from_str(&value.0); //("Invalid key found on STOP header.");
        let stop_value = f32::from_str(&value.1).expect("Invalid data found on STOP header.");
        bms_builder.with_stop(key, stop_value);
        true
    }

    fn parse_line(&self, line: &String) -> Option<(String, String)> {
        let capture_res = self.regex_parser.captures(line);
        if capture_res.is_none() {
            return Option::None;
        }

        let res = capture_res.unwrap();
        let (key, data): (&str, &str) = (&res["key"], &res["data"]);
        let keydata: (String, String) = (String::from(key.trim()), String::from(data.trim()));
        Option::from(keydata)
    }
}
