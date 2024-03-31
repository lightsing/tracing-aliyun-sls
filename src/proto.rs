use std::borrow::Cow;
use std::io;
use std::io::Write;

pub trait Message {
    fn encode(&self, writer: &mut impl Write) -> io::Result<()>;
    fn encoded_len(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyValue<'a> {
    pub key: &'static str,
    pub value: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log<'a> {
    /// UNIX Time Format
    pub time: u32,
    pub contents: Vec<KeyValue<'a>>,
    /// for time nano part
    pub time_ns: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogGroup<'a> {
    pub logs: Vec<Log<'a>>,
    /// reserved fields
    pub reserved: Option<String>,
    pub topic: Option<String>,
    pub source: Option<String>,
    pub log_tags: Vec<KeyValue<'a>>,
}

impl KeyValue<'_> {
    pub fn new<'a>(key: &'static str, value: impl Into<Cow<'a, str>>) -> KeyValue<'a> {
        KeyValue {
            key,
            value: value.into(),
        }
    }
}

// Manual implementation for faster encoding and Cow optimization

impl Message for KeyValue<'_> {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        encode_str(1u32, self.key, writer)?;
        encode_str(2u32, &self.value, writer)
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_str_len(1u32, self.key) + encoded_str_len(2u32, &self.value)
    }
}

impl Message for Log<'_> {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        encode_varint_field(1u32, self.time as u64, writer)?;
        for msg in &self.contents {
            encode_message(2u32, msg, writer)?;
        }
        if let Some(value) = self.time_ns {
            encode_fixed32(4u32, value, writer)?;
        }
        Ok(())
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_varint_field_len(1u32, self.time as u64)
            + encoded_len_repeated(2u32, &self.contents)
            + self
                .time_ns
                .as_ref()
                .map_or(0, |_| encoded_fixed32_len(4u32))
    }
}

impl Message for LogGroup<'_> {
    #[inline]
    fn encode(&self, writer: &mut impl Write) -> io::Result<()> {
        for msg in &self.logs {
            encode_message(1u32, msg, writer)?;
        }
        if let Some(ref value) = self.reserved {
            encode_str(2u32, value, writer)?;
        }
        if let Some(ref value) = self.topic {
            encode_str(3u32, value, writer)?;
        }
        if let Some(ref value) = self.source {
            encode_str(4u32, value, writer)?;
        }
        for msg in &self.log_tags {
            encode_message(6u32, msg, writer)?;
        }
        Ok(())
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        encoded_len_repeated(1u32, &self.logs)
            + self
                .reserved
                .as_ref()
                .map_or(0, |value| encoded_str_len(2u32, value))
            + self
                .topic
                .as_ref()
                .map_or(0, |value| encoded_str_len(3u32, value))
            + self
                .source
                .as_ref()
                .map_or(0, |value| encoded_str_len(4u32, value))
            + encoded_len_repeated(6u32, &self.log_tags)
    }
}

impl LogGroup<'_> {
    pub fn estimate_size(logs: &[Log], tags: &[KeyValue]) -> usize {
        encoded_len_repeated(1u32, logs) + encoded_len_repeated(6u32, tags)
    }
}

// Copy from prost

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum WireType {
    Varint = 0,
    SixtyFourBit = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    ThirtyTwoBit = 5,
}

#[inline]
fn encode_varint(mut value: u64, writer: &mut impl Write) -> io::Result<()> {
    loop {
        if value < 0x80 {
            writer.write_all(&[value as u8])?;
            break;
        } else {
            writer.write_all(&[((value & 0x7F) | 0x80) as u8])?;
            value >>= 7;
        }
    }
    Ok(())
}

#[inline]
fn encode_key(tag: u32, wire_type: WireType, writer: &mut impl Write) -> io::Result<()> {
    let key = (tag << 3) | wire_type as u32;
    encode_varint(u64::from(key), writer)
}

#[inline]
fn encode_varint_field(tag: u32, value: u64, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::Varint, writer)?;
    encode_varint(value, writer)
}

#[inline]
fn encode_fixed32(tag: u32, value: u32, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::ThirtyTwoBit, writer)?;
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

#[inline]
fn encode_message(tag: u32, msg: &impl Message, writer: &mut impl Write) -> io::Result<()> {
    encode_key(tag, WireType::LengthDelimited, writer)?;
    encode_varint(msg.encoded_len() as u64, writer)?;
    msg.encode(writer)
}

#[inline]
fn encode_str(tag: u32, value: impl AsRef<str>, writer: &mut impl Write) -> io::Result<()> {
    let value = value.as_ref();
    encode_key(tag, WireType::LengthDelimited, writer)?;
    encode_varint(value.len() as u64, writer)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

#[inline]
pub fn encoded_len_varint(value: u64) -> usize {
    // Based on [VarintSize64][1].
    // [1]: https://github.com/google/protobuf/blob/3.3.x/src/google/protobuf/io/coded_stream.h#L1301-L1309
    ((((value | 1).leading_zeros() ^ 63) * 9 + 73) / 64) as usize
}

#[inline]
pub fn key_len(tag: u32) -> usize {
    encoded_len_varint(u64::from(tag << 3))
}

#[inline]
fn encoded_str_len(tag: u32, value: impl AsRef<str>) -> usize {
    let value = value.as_ref();
    key_len(tag) + encoded_len_varint(value.len() as u64) + value.len()
}

#[inline]
pub fn encoded_len_repeated(tag: u32, messages: &[impl Message]) -> usize {
    key_len(tag) * messages.len()
        + messages
            .iter()
            .map(Message::encoded_len)
            .map(|len| len + encoded_len_varint(len as u64))
            .sum::<usize>()
}

#[inline]
fn encoded_varint_field_len(tag: u32, value: u64) -> usize {
    key_len(tag) + encoded_len_varint(value)
}

#[inline]
fn encoded_fixed32_len(tag: u32) -> usize {
    key_len(tag) + 4
}
