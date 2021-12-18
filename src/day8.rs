use anyhow::Context;
use enum_map::{enum_map, Enum, EnumMap};
use nom::{
    branch::alt,
    character::complete::{char, multispace0, multispace1},
    combinator::{eof, success},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::{collect_separated_terminated, parse_separated_terminated},
    ParserExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Enum)]
enum Segment {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

use Segment::*;

use crate::library::IterExt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SegmentSet {
    segments: EnumMap<Segment, bool>,
}

impl SegmentSet {
    fn count(&self) -> usize {
        self.segments.values().filter(|&b| *b).count()
    }

    // What digit is this, if any?
    fn identify(&self) -> Option<usize> {
        get_digit_shapes()
            .iter()
            .position(|candidate| self == candidate)
    }
}

fn parse_segment(input: &str) -> IResult<&str, Segment, ErrorTree<&str>> {
    alt((
        char('a').value(A),
        char('b').value(B),
        char('c').value(C),
        char('d').value(D),
        char('e').value(E),
        char('f').value(F),
        char('g').value(G),
    ))
    .parse(input)
}

fn parse_segment_set(input: &str) -> IResult<&str, SegmentSet, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_segment.context("segment"),
        success(()),
        multispace1.or(eof).peek(),
        SegmentSet::default,
        |mut set, segment| {
            set.segments[segment] = true;
            set
        },
    )
    .parse(input)
}

fn parse_signals(input: &str) -> IResult<&str, [SegmentSet; 10], ErrorTree<&str>> {
    parse_segment_set
        .context("signal")
        .separated_array(multispace1)
        .parse(input)
}

fn parse_output_digits(input: &str) -> IResult<&str, [SegmentSet; 4], ErrorTree<&str>> {
    parse_segment_set
        .context("output digit")
        .separated_array(multispace1)
        .parse(input)
}

fn parse_display(input: &str) -> IResult<&str, Display, ErrorTree<&str>> {
    parse_signals
        .context("signal data")
        .terminated(char('|').delimited_by(multispace0))
        .and(parse_output_digits.context("output digits"))
        .map(|(signals, output_digits)| Display {
            signals,
            output_digits,
        })
        .parse(input)
}

fn parse_all_displays(input: &str) -> Result<Vec<Display>, ErrorTree<Location>> {
    let parser = collect_separated_terminated(
        parse_display.context("display"),
        multispace1,
        eof.preceded_by(multispace0),
    );
    final_parser(parser)(input)
}

#[derive(Debug, Clone)]
struct Display {
    signals: [SegmentSet; 10],
    output_digits: [SegmentSet; 4],
}

#[derive(Debug, Clone, Copy)]
struct DisplayWiring {
    // Key: the correct output signal
    // value: the input  segment
    wires: EnumMap<Segment, Segment>,
}

impl DisplayWiring {
    fn compute(signals: &[SegmentSet; 10]) -> Option<Self> {
        let mut counts: EnumMap<Segment, u8> = EnumMap::default();

        signals.iter().for_each(|signal| {
            signal
                .segments
                .iter()
                .filter(|(_, &on)| on)
                .for_each(|(segment, _)| counts[segment] += 1);
        });

        counts
            .iter()
            .map(|(input_signal, &count)| {
                Some((
                    match count {
                        6 => B,
                        4 => E,
                        9 => F,
                        // Either A or C; distinguish by identifying the 1
                        8 => match signals.iter().find(|signal| signal.count() == 2)?.segments
                            [input_signal]
                        {
                            true => C,
                            false => A,
                        },
                        // either D or G, distinguish by identifying the 4
                        7 => match signals.iter().find(|signal| signal.count() == 4)?.segments
                            [input_signal]
                        {
                            true => D,
                            false => G,
                        },
                        _ => return None,
                    },
                    input_signal,
                ))
            })
            .map(|opt| opt.ok_or(()))
            .use_oks(|signals| {
                let mut wires = enum_map! { _ => A};
                wires.extend(signals);
                DisplayWiring { wires }
            })
            .ok()
    }

    fn get_digit(self, input: SegmentSet) -> SegmentSet {
        SegmentSet {
            segments: enum_map!(segment => input.segments[self.wires[segment]]),
        }
    }
}

fn get_digit_shapes() -> [SegmentSet; 10] {
    [
        // 0
        SegmentSet {
            segments: enum_map! {
                A | B | C | E | F | G => true,
                _ => false,
            },
        },
        // 1
        SegmentSet {
            segments: enum_map! {
                C | F => true,
                _ => false,
            },
        },
        // 2
        SegmentSet {
            segments: enum_map! {
             A | C | D | E | G => true,
                _ => false,
            },
        },
        // 3
        SegmentSet {
            segments: enum_map! {
             A | C | D | F | G  => true,
                _ => false,
            },
        },
        // 4
        SegmentSet {
            segments: enum_map! {
             B | C | D | F => true,
                _ => false,
            },
        },
        // 5
        SegmentSet {
            segments: enum_map! {
            A | B | D | F | G => true,
               _ => false,
               },
        },
        // 6
        SegmentSet {
            segments: enum_map! {
             A | B | D | E| F | G => true,
                _ => false,
            },
        },
        // 7
        SegmentSet {
            segments: enum_map! {
             A | C | F => true,
                _ => false,
            },
        },
        // 8
        SegmentSet {
            segments: enum_map! {
                _ => true
            },
        },
        // 9
        SegmentSet {
            segments: enum_map! {
             A | B | C | D | F | G => true,
                _ => false,
            },
        },
    ]
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let display_data = parse_all_displays(input).context("parse error")?;
    let digits = get_digit_shapes();

    let mut digit_counts = [0; 10];

    for display in display_data {
        let wiring =
            DisplayWiring::compute(&display.signals).context("failed to compute display wiring")?;

        for output_digit in display.output_digits {
            let digit = wiring.get_digit(output_digit);
            let digit = digits
                .iter()
                .position(|&candidate| digit == candidate)
                .context("no matching digit")?;

            digit_counts[digit] += 1;
        }
    }

    Ok([1, 4, 7, 8].iter().map(|&digit| digit_counts[digit]).sum())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let display_data = parse_all_displays(input).context("parse error")?;

    // I'd much rather have this be an iterator sum, but the control flow
    // (dealing with Results) got pretty hairy
    let mut total = 0;

    for display_data in display_data {
        // Perform the solve- figure out which input segments are associated
        // with which output segments
        let display_wiring = DisplayWiring::compute(&display_data.signals)
            .context("failed to compute display wiring")?;

        // iterate over the 4 output digits. Associate each one with an
        // exponent so that the total sum can be accumulated
        for (&digit, exp) in display_data.output_digits.iter().rev().zip(0..) {
            // Run the input signal through the wiring to get the corrected
            // segments
            let digit = display_wiring.get_digit(digit);

            // Figure out which digit is on the display
            let digit = digit.identify().context("no matching digit")?;
            total += digit * 10usize.pow(exp);
        }
    }

    Ok(total)
}
