/// Collections of structs, functions, and consts common to everything in the BMS module.
pub mod format;
pub mod parser;
pub mod timeline;

const BASE36: &'static str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A hexadecimal representation of an "object". Takes the range 00-ZZ.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct Alphanumeric {
    pub key: usize,
}

impl Alphanumeric {
    /// Create an Alphanumeric from a str. Defaults to an Alphanumeric of key 0 if the str is invalid.
    pub fn from_str(key: &str) -> Alphanumeric {
        Alphanumeric {
            key: usize::from_str_radix(key, 36).unwrap_or(0),
        }
    }

    /// Create an Alphanumeric from an int.
    pub fn from_int(key: usize) -> Alphanumeric {
        Alphanumeric { key }
    }

    /// Returns a base-36 string representation of the Alphanumeric key.
    pub fn as_base36(&self) -> String {
        // There is definitely a more elegant way to do this :)
        let mut s = String::new();
        s.push(BASE36.chars().nth(self.key / 36).expect("Bad alphanumeric"));
        s.push(BASE36.chars().nth(self.key % 36).expect("Bad alphanumeric"));
        s
    }
}

impl std::fmt::Display for Alphanumeric {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Alphanumeric: {} (Base 36: {})",
            self.key,
            self.as_base36()
        )
    }
}

/// BMS object types
#[derive(Debug)]
pub enum ObjType {
    Auto(Alphanumeric), // Keysound
    BGA(Alphanumeric),
    Note(Alphanumeric),
    LongNote(f32), // Denotes ending measure of a long note object
}

impl Default for ObjType {
    fn default() -> Self {
        ObjType::Auto(Alphanumeric::from_int(0))
    }
}

/// An "object" in a BMS file, represented as a
#[derive(Debug, Default)]
pub struct Object {
    pub time: i64,
    pub measure: f32,
    pub channel: u32,
    pub objtype: ObjType,

    // Timing offset from when the note was hit, in measures
    // Initializes to None, which can be used to determine whether this note has already been hit.
    pub hit_offset: Option<f32>,
    pub longnote_hit_offset: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let a = Alphanumeric::from_str("X5");
        assert_eq!(a.key, 1193);
    }

    #[test]
    fn test_from_int() {
        let a = Alphanumeric::from_int(1193);
        assert_eq!(a.key, 1193);
    }

    #[test]
    fn test_from_str_invalid_str() {
        let a = Alphanumeric::from_str("!@#");
        assert_eq!(a.key, 0);
    }

    #[test]
    fn test_as_base36() {
        let a = Alphanumeric::from_int(1193);
        assert_eq!(a.as_base36(), "X5");
    }
}
