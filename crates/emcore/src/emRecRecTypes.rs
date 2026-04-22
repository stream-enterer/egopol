// SPLIT: Split from emRec.h — record type definitions extracted.
//
// After Phase 4b.1, the canonical `emColorRec` / `emAlignmentRec` types live in
// `emColorRec.rs` / `emAlignmentRec.rs`. The legacy types that previously lived
// in this file have been removed. Remaining here are:
//   - Free serialization helpers used by consumers that haven't yet migrated
//     to the new `emRec`-based API (stopgap; see Phase 4b.1 ledger).
//   - `emRecFileReader` / `emRecFileWriter` convenience wrappers.
use std::path::Path;

use crate::emColor::emColor;
use crate::emRecParser::{
    parse_rec, parse_rec_with_format, write_rec, write_rec_with_format, RecError, RecStruct,
    RecValue,
};
use crate::emTiling::Alignment;

// ---- Free serialization helpers (stopgap) ----
//
// These functions preserve byte-for-byte the serialization semantics of the
// now-deleted `emColorRec::ToRecStruct` / `FromRecStruct` /
// `emAlignmentRec::ToRecValue` / `FromRecValue` static methods. They operate
// on value types (`emColor`, `Alignment`), have no state, and register no
// listeners. Consumers that previously called the static methods were migrated
// to these free functions in Phase 4b.1; the new `emColorRec` / `emAlignmentRec`
// types in their own modules will gain proper serialization hooks in a later
// phase, at which point these helpers can be retired.

/// Write a color to a `RecStruct`. Stopgap extracted from the legacy
/// `emColorRec::ToRecStruct`.
pub fn em_color_to_rec_struct(color: emColor, have_alpha: bool) -> RecStruct {
    let mut s = RecStruct::new();
    s.set_int("r", color.GetRed() as i32);
    s.set_int("g", color.GetGreen() as i32);
    s.set_int("b", color.GetBlue() as i32);
    if have_alpha {
        s.set_int("a", color.GetAlpha() as i32);
    }
    s
}

/// Read a color from an emRec struct field. Stopgap extracted from the legacy
/// `emColorRec::FromRecStruct`.
///
/// Expects a struct with fields `r`, `g`, `b`, and optionally `a`, each an
/// integer 0..255.
pub fn em_color_from_rec_struct(rec: &RecStruct, have_alpha: bool) -> Result<emColor, RecError> {
    let r = rec
        .get_int("r")
        .ok_or_else(|| RecError::MissingField("r".into()))? as u8;
    let g = rec
        .get_int("g")
        .ok_or_else(|| RecError::MissingField("g".into()))? as u8;
    let b = rec
        .get_int("b")
        .ok_or_else(|| RecError::MissingField("b".into()))? as u8;
    let a = if have_alpha {
        rec.get_int("a").unwrap_or(255) as u8
    } else {
        255
    };
    Ok(emColor::rgba(r, g, b, a))
}

/// Convert an `Alignment` to a `RecValue` identifier. Stopgap extracted from
/// the legacy `emAlignmentRec::ToRecValue`.
///
/// NOTE: `Alignment` is the Rust-only single-axis enum (Start|Center|End|
/// Stretch), not C++ `emAlignment` u8. This is a pre-existing Rust-only drift
/// from C++; the alignment-drift audit belongs to Phase 4e+.
pub fn em_alignment_to_rec_value(alignment: Alignment) -> RecValue {
    let s = match alignment {
        Alignment::Start => "start",
        Alignment::Center => "center",
        Alignment::End => "end",
        Alignment::Stretch => "stretch",
    };
    RecValue::Ident(s.into())
}

/// Read an `Alignment` from a `RecValue`. Stopgap extracted from the legacy
/// `emAlignmentRec::FromRecValue`.
///
/// C++ `emAlignmentRec` stores a bitmask combining TOP/BOTTOM/LEFT/RIGHT/CENTER.
/// The Rust `Alignment` enum is single-axis, so we accept hyphen-joined forms
/// like "bottom-left" and collapse to a single value by preferring the axis
/// that corresponds to Start/End. For symmetric combinations we return Center.
pub fn em_alignment_from_rec_value(val: &RecValue) -> Result<Alignment, RecError> {
    match val {
        RecValue::Ident(s) => {
            let mut has_top = false;
            let mut has_bottom = false;
            let mut has_left = false;
            let mut has_right = false;
            let mut has_center = false;
            let mut has_stretch = false;
            for part in s.split('-') {
                match part {
                    "top" => has_top = true,
                    "bottom" => has_bottom = true,
                    "left" => has_left = true,
                    "right" => has_right = true,
                    "center" => has_center = true,
                    "stretch" | "fill" => has_stretch = true,
                    "start" => has_left = true,
                    "end" => has_right = true,
                    "" => {}
                    other => {
                        return Err(RecError::InvalidValue {
                            field: "alignment".into(),
                            message: format!("unknown alignment part: {other}"),
                        });
                    }
                }
            }
            if has_stretch {
                return Ok(Alignment::Stretch);
            }
            if has_left {
                return Ok(Alignment::Start);
            }
            if has_right {
                return Ok(Alignment::End);
            }
            if has_top {
                return Ok(Alignment::Start);
            }
            if has_bottom {
                return Ok(Alignment::End);
            }
            if has_center {
                return Ok(Alignment::Center);
            }
            Err(RecError::InvalidValue {
                field: "alignment".into(),
                message: format!("unknown alignment: {s}"),
            })
        }
        _ => Err(RecError::InvalidValue {
            field: "alignment".into(),
            message: "expected identifier".into(),
        }),
    }
}

// ---- emRecFileReader / emRecFileWriter ----

/// Convenience wrapper for reading an emRec tree from a file.
///
/// Port of C++ `emRecFileReader`. Provides a simpler API than the C++ version
/// since Rust does not need the incremental read/continue/quit protocol.
pub struct emRecFileReader;

impl emRecFileReader {
    /// Read an emRec file and parse it into a `RecStruct`.
    pub fn read(path: &Path) -> Result<RecStruct, RecError> {
        let content = std::fs::read_to_string(path).map_err(RecError::Io)?;
        parse_rec(&content)
    }

    /// Read an emRec file, verifying the format header matches `format_name`.
    pub fn read_with_format(path: &Path, format_name: &str) -> Result<RecStruct, RecError> {
        let content = std::fs::read_to_string(path).map_err(RecError::Io)?;
        parse_rec_with_format(&content, format_name)
    }
}

/// Convenience wrapper for writing an emRec tree to a file.
///
/// Port of C++ `emRecFileWriter`. Provides a simpler API than the C++ version
/// since Rust does not need the incremental write/continue/quit protocol.
pub struct emRecFileWriter;

impl emRecFileWriter {
    /// Write a `RecStruct` to a file (no format header).
    pub fn write(path: &Path, rec: &RecStruct) -> Result<(), RecError> {
        let content = write_rec(rec);
        std::fs::write(path, content).map_err(RecError::Io)
    }

    /// Write a `RecStruct` to a file with a `#%rec:FormatName%#` header.
    pub fn write_with_format(
        path: &Path,
        rec: &RecStruct,
        format_name: &str,
    ) -> Result<(), RecError> {
        let content = write_rec_with_format(rec, format_name);
        std::fs::write(path, content).map_err(RecError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_value_round_trip() {
        for align in [
            Alignment::Start,
            Alignment::Center,
            Alignment::End,
            Alignment::Stretch,
        ] {
            let val = em_alignment_to_rec_value(align);
            let parsed = em_alignment_from_rec_value(&val).unwrap();
            assert_eq!(parsed, align);
        }
    }

    #[test]
    fn color_struct_round_trip() {
        let color = emColor::rgba(10, 20, 30, 255);
        let s = em_color_to_rec_struct(color, false);
        let parsed = em_color_from_rec_struct(&s, false).unwrap();
        assert_eq!(parsed, color);
    }

    #[test]
    fn color_struct_with_alpha_round_trip() {
        let color = emColor::rgba(10, 20, 30, 128);
        let s = em_color_to_rec_struct(color, true);
        let parsed = em_color_from_rec_struct(&s, true).unwrap();
        assert_eq!(parsed, color);
    }
}
