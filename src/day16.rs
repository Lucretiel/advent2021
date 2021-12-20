use std::{
    fmt::Display,
    ops::{AddAssign, Shl, Shr},
};

use anyhow::Context;
use nom::{
    bits::complete::{tag as tag_bits, take},
    branch::alt,
    combinator::{eof, success},
    error::{ErrorKind as NomErrorKind, ParseError},
    sequence::pair,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, ExtractContext, Location, RecreateContext},
    multi::collect_separated_terminated,
    ParserExt,
};
use thiserror::Error;

use crate::library::IterExt;

#[derive(Debug, Clone)]
enum PacketData {
    Literal(u64),
    Operator(Operator),
}

impl PacketData {
    fn version_sum(&self) -> u64 {
        match self {
            PacketData::Literal(..) => 0,
            PacketData::Operator(op) => op.version_sum(),
        }
    }

    fn value(&self) -> u64 {
        match *self {
            PacketData::Literal(value) => value,
            PacketData::Operator(ref op) => op.value(),
        }
    }
}

#[derive(Debug, Clone)]
struct Packet {
    version: u64,
    data: PacketData,
}

impl Packet {
    fn version_sum(&self) -> u64 {
        self.version + self.data.version_sum()
    }

    fn value(&self) -> u64 {
        self.data.value()
    }
}

type BitsInput<'a> = (&'a [u8], usize);

/// The length, in bits, of a `BitsInput`
fn len(input: BitsInput) -> usize {
    let (buffer, offset) = input;
    (buffer.len() * 8) - offset
}

/// Const generic bits parser. Parse N bits into a value of type T.
fn take_bits<T, const N: usize>(input: BitsInput) -> IResult<BitsInput, T, ErrorTree<BitsInput>>
where
    T: From<u8> + AddAssign + Shl<usize, Output = T> + Shr<usize, Output = T>,
{
    take(N).parse(input)
}

/// Parse a single bit as a bool
fn take_bit(input: BitsInput) -> IResult<BitsInput, bool, ErrorTree<BitsInput>> {
    take_bits::<u8, 1>.map(|b| b != 0).parse(input)
}

/// Parse a chunk of a literal value: a single continuation bit, followed by 4
/// payload bits
fn parse_chunk(input: BitsInput) -> IResult<BitsInput, (bool, u8), ErrorTree<BitsInput>> {
    pair(take_bit, take_bits::<u8, 4>).parse(input)
}

fn parse_literal_packet(input: BitsInput) -> IResult<BitsInput, u64, ErrorTree<BitsInput>> {
    let (mut input, _type_id) = tag_bits(4u8, 3usize).context("type id").parse(input)?;

    let mut result = 0;

    loop {
        // Use cut here because the earlier successful parse of the type ID
        // means that this is definitely a literal packet
        let (tail, (more, payload)) = parse_chunk.cut().context("chunk").parse(input)?;
        result = (result << 4) + (payload as u64);
        input = tail;

        if !more {
            break;
        }
    }

    Ok((input, result))
}

#[derive(Debug, Copy, Clone)]
enum Opcode {
    Sum,
    Product,
    Min,
    Max,
    Greater,
    Less,
    Eq,
}

fn parse_opcode(input: BitsInput) -> IResult<BitsInput, Opcode, ErrorTree<BitsInput>> {
    let (tail, type_id) = take_bits::<u8, 3>.context("opcode").parse(input)?;

    match type_id {
        0 => Some(Opcode::Sum),
        1 => Some(Opcode::Product),
        2 => Some(Opcode::Min),
        3 => Some(Opcode::Max),
        5 => Some(Opcode::Greater),
        6 => Some(Opcode::Less),
        7 => Some(Opcode::Eq),
        _ => None,
    }
    .map(|code| (tail, code))
    .ok_or_else(|| nom::Err::Error(ParseError::from_error_kind(input, NomErrorKind::TagBits)))
}

#[derive(Debug, Clone)]
struct Operator {
    type_id: Opcode,
    operands: Vec<Packet>,
}

impl Operator {
    fn version_sum(&self) -> u64 {
        self.operands
            .iter()
            .map(|packet| packet.version_sum())
            .sum()
    }

    fn value(&self) -> u64 {
        let operands = self.operands.iter().map(|op| op.value());

        match self.type_id {
            Opcode::Sum => operands.sum(),
            Opcode::Product => operands.product(),
            Opcode::Min => operands.min().unwrap_or(0),
            Opcode::Max => operands.max().unwrap_or(0),
            Opcode::Greater => {
                if operands.streaming_windows().all(|[a, b]| a > b) {
                    1
                } else {
                    0
                }
            }
            Opcode::Less => {
                if operands.streaming_windows().all(|[a, b]| a < b) {
                    1
                } else {
                    0
                }
            }
            Opcode::Eq => {
                if operands.streaming_windows().all(|[a, b]| a == b) {
                    1
                } else {
                    0
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OperatorLength {
    Bits(usize),
    Packets(usize),
}

impl OperatorLength {
    fn empty(&self) -> bool {
        matches!(*self, OperatorLength::Bits(0) | OperatorLength::Packets(0))
    }
}

fn parse_operator_length(
    input: BitsInput,
) -> IResult<BitsInput, OperatorLength, ErrorTree<BitsInput>> {
    let (input, length_type) = take_bit(input)?;

    match length_type {
        false => take_bits::<usize, 15>
            .map(OperatorLength::Bits)
            .parse(input),
        true => take_bits::<usize, 11>
            .map(OperatorLength::Packets)
            .parse(input),
    }
}

fn parse_operator_packet(input: BitsInput) -> IResult<BitsInput, Operator, ErrorTree<BitsInput>> {
    let (input, opcode) = parse_opcode(input)?;
    let (mut input, mut length) = parse_operator_length.cut().parse(input)?;

    let mut packets = match length {
        OperatorLength::Bits(_) => Vec::new(),
        OperatorLength::Packets(n) => Vec::with_capacity(n),
    };

    while !length.empty() {
        let (tail, packet) = parse_packet.cut().parse(input)?;
        packets.push(packet);

        match length {
            OperatorLength::Bits(ref mut bit_count) => {
                let packet_len = len(input) - len(tail);
                *bit_count -= packet_len
            }
            OperatorLength::Packets(ref mut count) => *count -= 1,
        }

        input = tail;
    }

    Ok((
        input,
        Operator {
            operands: packets,
            type_id: opcode,
        },
    ))
}

fn parse_packet(input: BitsInput) -> IResult<BitsInput, Packet, ErrorTree<BitsInput>> {
    take_bits::<u64, 3>
        .and(alt((
            parse_literal_packet.map(PacketData::Literal),
            parse_operator_packet.map(PacketData::Operator),
        )))
        .map(|(version, data)| Packet { version, data })
        .parse(input)
}

fn final_parse_top_packet(input: &[u8]) -> Result<Packet, ErrorTree<BitErrorLocation>> {
    let parse_trailing_zeroes =
        collect_separated_terminated(tag_bits(0u8, 1usize).value(()), success(()), eof)
            .map(|()| ());
    let mut parse_top_packet = parse_packet
        .terminated(parse_trailing_zeroes)
        .complete()
        .all_consuming();

    match parse_top_packet.parse((input, 0)) {
        Ok((_, packet)) => Ok(packet),
        Err(nom::Err::Error(err) | nom::Err::Failure(err)) => Err(err.extract_context((input, 0))),
        Err(nom::Err::Incomplete(..)) => unreachable!(),
    }
}

fn parse_hex_byte(input: &str) -> IResult<&str, u8, ErrorTree<&str>> {
    match input.len() {
        0 => Err(nom::Err::Error(ErrorTree::from_error_kind(
            input,
            NomErrorKind::HexDigit,
        ))),
        1 => u8::from_str_radix(input, 16)
            .map(|b| ("", b << 4))
            .map_err(|_| {
                nom::Err::Error(ErrorTree::from_error_kind(input, NomErrorKind::HexDigit))
            }),
        _ => {
            let (byte, tail) = input.split_at(2);
            u8::from_str_radix(byte, 16)
                .map(|b| (tail, b))
                .map_err(|_| {
                    nom::Err::Error(ErrorTree::from_error_kind(input, NomErrorKind::HexDigit))
                })
        }
    }
}

fn parse_hex(input: &str) -> IResult<&str, Vec<u8>, ErrorTree<&str>> {
    collect_separated_terminated(parse_hex_byte, success(()), eof).parse(input.trim())
}

fn final_parse_hex(input: &str) -> Result<Vec<u8>, ErrorTree<Location>> {
    final_parser(parse_hex)(input)
}

#[derive(Debug, Clone, Copy)]
struct BitErrorLocation {
    byte_offset: usize,
    bit_offset: usize,
}

impl BitErrorLocation {
    fn from_input(input: BitsInput) -> Self {
        let (buf, bits) = input;

        Self {
            byte_offset: buf.len(),
            bit_offset: bits,
        }
        .normalize()
    }
    fn normalize(self) -> Self {
        Self {
            byte_offset: self.byte_offset + self.bit_offset / 8,
            bit_offset: self.bit_offset % 8,
        }
    }
}

impl<'a> RecreateContext<BitsInput<'a>> for BitErrorLocation {
    fn recreate_context(original_input: BitsInput, tail: BitsInput) -> Self {
        let original = BitErrorLocation::from_input(original_input);
        let mut tail = BitErrorLocation::from_input(tail);

        if original.bit_offset > tail.bit_offset {
            tail.bit_offset += 8;
            tail.byte_offset += 1;
        }

        Self {
            byte_offset: original.byte_offset - tail.byte_offset,
            bit_offset: tail.bit_offset - original.bit_offset,
        }
    }
}

impl Display for BitErrorLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "byte {}, bit {}", self.byte_offset, self.bit_offset)
    }
}

#[derive(Debug, Error)]
enum HexPacketParseError {
    #[error("error parsing hex encoding to binary")]
    HexError(#[from] ErrorTree<Location>),

    #[error("error parsing binary packet into structure")]
    BitError(#[from] ErrorTree<BitErrorLocation>),
}

fn final_parse_hex_packet(input: &str) -> Result<Packet, HexPacketParseError> {
    let hex = final_parse_hex(input)?;
    let packet = final_parse_top_packet(&hex)?;

    Ok(packet)
}

pub fn part1(input: &str) -> anyhow::Result<u64> {
    let packet = final_parse_hex_packet(input).context("Parse error")?;

    Ok(packet.version_sum())
}

pub fn part2(input: &str) -> anyhow::Result<u64> {
    let packet = final_parse_hex_packet(input).context("parse error")?;

    Ok(packet.value())
}
