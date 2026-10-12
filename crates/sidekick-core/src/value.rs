//! Named values: bytes of the game's memory turned into facts, as a definition's `[values]` say.

use crate::Machine;
use sidekick_def::{Charset, Count, Field, Value, ValueType};
use std::{collections::BTreeMap, fmt};

/// The most records a table is read with, whatever its count says: a count read from a game
/// that isn't where the definition thinks would otherwise read the whole memory, many times.
const MOST_RECORDS: usize = 1024;

/// A value as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    /// A number: `u8`, `i8`, `u16`, `u16be`, `bcd`, `digits`.
    Number(i64),
    /// A table field's `bits`: whether they're all set.
    Flag(bool),
    /// A `bitset`: each item, from bit 0 of the first byte.
    Bits(Vec<bool>),
    /// `bytes`.
    Bytes(Vec<u8>),
    /// `text`, in its charset; a code outside it reads as `?`.
    Text(String),
    /// A table's records, each field by name.
    Table(Vec<BTreeMap<String, Self>>),
}

impl Read {
    /// The value as a number, for tests: a number, or a flag as 0 or 1.
    #[must_use]
    pub const fn number(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Flag(f) => Some(*f as i64),
            Self::Bits(_) | Self::Bytes(_) | Self::Text(_) | Self::Table(_) => None,
        }
    }
}

/// As a TOML value: numbers and flags as they are, a bitset as the items set, bytes in hex,
/// text quoted, a table as a list of inline tables.
impl fmt::Display for Read {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Flag(b) => write!(f, "{b}"),
            Self::Bits(bits) => {
                let set: Vec<String> = bits
                    .iter()
                    .enumerate()
                    .filter(|&(_, &b)| b)
                    .map(|(i, _)| i.to_string())
                    .collect();
                write!(f, "[{}]", set.join(", "))
            }
            Self::Bytes(bytes) => {
                let hex: Vec<String> = bytes.iter().map(|b| format!("0x{b:02X}")).collect();
                write!(f, "[{}]", hex.join(", "))
            }
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Table(records) => {
                f.write_str("[")?;
                for (i, record) in records.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    let fields: Vec<String> =
                        record.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                    write!(f, "{{ {} }}", fields.join(", "))?;
                }
                f.write_str("]")
            }
        }
    }
}

/// The game's memory as a value reads it: the processor's view, or one RAM bank of a 128K.
pub(crate) struct Memory<'a> {
    pub(crate) machine: &'a Machine,
    pub(crate) bank: Option<u8>,
}

impl Memory<'_> {
    /// The byte at `addr`; past the top of memory, or of a bank, reads as 0.
    fn byte(&self, addr: u32) -> u8 {
        self.bank.map_or_else(
            || u16::try_from(addr).map_or(0, |a| self.machine.peek(a)),
            |bank| {
                addr.checked_sub(0xC000)
                    .and_then(|offset| {
                        let offset = usize::try_from(offset).ok()?;
                        self.machine.ram_bank(bank)?.get(offset).copied()
                    })
                    .unwrap_or(0)
            },
        )
    }

    fn bytes(&self, addr: u32, len: u32) -> Vec<u8> {
        (addr..addr.saturating_add(len))
            .map(|a| self.byte(a))
            .collect()
    }
}

/// The parts of a value or a field that say how to decode its bytes.
struct Shape<'a> {
    kind: ValueType,
    len: u32,
    zero: u8,
    charset: Option<&'a Charset>,
}

impl Shape<'_> {
    fn read(&self, memory: &Memory, addr: u32) -> Read {
        let bytes = memory.bytes(addr, self.size());
        match self.kind {
            ValueType::U8 => Read::Number(i64::from(bytes[0])),
            ValueType::I8 => Read::Number(i64::from(i8::from_le_bytes([bytes[0]]))),
            ValueType::U16 => Read::Number(i64::from(u16::from_le_bytes([bytes[0], bytes[1]]))),
            ValueType::U16be => Read::Number(i64::from(u16::from_be_bytes([bytes[0], bytes[1]]))),
            ValueType::Bcd => Read::Number(bytes.iter().fold(0, |n, &b| {
                n * 100 + i64::from(b >> 4) * 10 + i64::from(b & 0x0F)
            })),
            ValueType::Digits => Read::Number(
                bytes
                    .iter()
                    .fold(0, |n, &b| n * 10 + i64::from(b.wrapping_sub(self.zero))),
            ),
            ValueType::Bitset => Read::Bits(
                bytes
                    .iter()
                    .flat_map(|&b| (0..8).map(move |bit| b & (1 << bit) != 0))
                    .collect(),
            ),
            ValueType::Text => Read::Text(bytes.iter().map(|&b| letter(self.charset, b)).collect()),
            ValueType::Bytes | ValueType::Table | ValueType::Font | ValueType::Sprites => {
                Read::Bytes(bytes)
            }
        }
    }

    /// How many bytes it takes.
    const fn size(&self) -> u32 {
        match self.kind {
            ValueType::U8 | ValueType::I8 => 1,
            ValueType::U16 | ValueType::U16be => 2,
            _ => self.len,
        }
    }
}

/// The letter a code is in a charset: ASCII by default, `?` for a code outside it.
fn letter(charset: Option<&Charset>, code: u8) -> char {
    match charset {
        Some(Charset::Table { first, chars }) => code
            .checked_sub(*first)
            .and_then(|i| chars.chars().nth(usize::from(i)))
            .unwrap_or('?'),
        Some(Charset::Named(_)) | None => {
            if (0x20..0x7F).contains(&code) {
                char::from(code)
            } else {
                '?'
            }
        }
    }
}

/// Reads a value of a definition. `count` gives the number a table's `count` names; `None`
/// when the value can't be read (no address; fonts and sprites, which the panel reads).
pub(crate) fn read(
    machine: &Machine,
    value: &Value,
    count: impl Fn(&str) -> Option<i64>,
) -> Option<Read> {
    let addr = u32::from(value.addr?.0);
    let memory = Memory {
        machine,
        bank: value.bank,
    };
    match value.kind {
        ValueType::Font | ValueType::Sprites => None,
        ValueType::Table => {
            let records = match value.count.as_ref()? {
                Count::Fixed(n) => usize::try_from(*n).ok()?,
                Count::Value(name) => usize::try_from(count(name)?).unwrap_or(0),
            }
            .min(MOST_RECORDS);
            let stride = value.stride.unwrap_or(1);
            let mut table = Vec::with_capacity(records);
            for i in 0..u32::try_from(records).ok()? {
                let at = addr + i * stride;
                table.push(
                    value
                        .fields
                        .iter()
                        .map(|(name, field)| (name.clone(), read_field(&memory, field, at)))
                        .collect(),
                );
            }
            Some(Read::Table(table))
        }
        kind => Some(
            Shape {
                kind,
                len: value.len.unwrap_or(1),
                zero: value.zero.unwrap_or(b'0'),
                charset: value.charset.as_ref(),
            }
            .read(&memory, addr),
        ),
    }
}

fn read_field(memory: &Memory, field: &Field, record: u32) -> Read {
    let at = record + field.offset;
    if let Some(bits) = field.bits {
        return Read::Flag(memory.byte(at) & bits == bits);
    }
    Shape {
        kind: field.kind,
        len: field.len.unwrap_or(1),
        zero: b'0',
        charset: field.charset.as_ref(),
    }
    .read(memory, at)
}

/// Why a write by name was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// No value has that name.
    Unknown,
    /// The definition doesn't mark it `writable`.
    NotWritable,
    /// It isn't a number: only `u8`, `i8`, `u16`, `u16be`, `bcd` and `digits` are written.
    NotANumber,
    /// The number doesn't fit the value.
    OutOfRange,
    /// It's in the ROM, or in a 128K bank (banked writes come with 128K play).
    NotInRam,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unknown => "no value has that name",
            Self::NotWritable => "the value isn't writable",
            Self::NotANumber => "only numbers are written",
            Self::OutOfRange => "the number doesn't fit the value",
            Self::NotInRam => "the value isn't in RAM the processor sees",
        })
    }
}

impl std::error::Error for WriteError {}

/// The bytes `number` is as `value`, and where they go.
pub(crate) fn encode(value: &Value, number: i64) -> Result<(u16, Vec<u8>), WriteError> {
    if !value.writable {
        return Err(WriteError::NotWritable);
    }
    let addr = value.addr.ok_or(WriteError::NotInRam)?.0;
    if value.bank.is_some() || addr < 0x4000 {
        return Err(WriteError::NotInRam);
    }
    let len = value.len.unwrap_or(1);
    let range = |r: std::ops::RangeInclusive<i64>| {
        if r.contains(&number) {
            Ok(())
        } else {
            Err(WriteError::OutOfRange)
        }
    };
    let digits = |base: u32| -> Result<Vec<i64>, WriteError> {
        range(0..=i64::from(base).pow(len).saturating_sub(1))?;
        Ok((0..len)
            .rev()
            .map(|i| number / i64::from(base).pow(i) % i64::from(base))
            .collect())
    };
    let bytes = match value.kind {
        ValueType::U8 => {
            range(0..=0xFF)?;
            vec![byte(number)]
        }
        ValueType::I8 => {
            range(-0x80..=0x7F)?;
            vec![byte(number.rem_euclid(0x100))]
        }
        ValueType::U16 | ValueType::U16be => {
            range(0..=0xFFFF)?;
            let word = [byte(number & 0xFF), byte(number >> 8)];
            if value.kind == ValueType::U16 {
                word.to_vec()
            } else {
                vec![word[1], word[0]]
            }
        }
        ValueType::Bcd => digits(100)?
            .into_iter()
            .map(|pair| byte(pair / 10 * 16 + pair % 10))
            .collect(),
        ValueType::Digits => {
            let zero = i64::from(value.zero.unwrap_or(b'0'));
            digits(10)?
                .into_iter()
                .map(|d| byte((zero + d) & 0xFF))
                .collect()
        }
        _ => return Err(WriteError::NotANumber),
    };
    if u32::from(addr) + u32::try_from(bytes.len()).unwrap_or(u32::MAX) > 0x1_0000 {
        return Err(WriteError::NotInRam);
    }
    Ok((addr, bytes))
}

/// The low byte of a number already known to fit.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers range-check first"
)]
const fn byte(n: i64) -> u8 {
    n as u8
}
