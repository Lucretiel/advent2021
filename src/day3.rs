use std::num::ParseIntError;

use anyhow::Context;
use itertools::{self, Itertools};

#[derive(Default)]
struct Counts {
    ones: u32,
}

struct IterCounter<'a, I> {
    iter: I,
    count: &'a mut u32,
}

impl<'a, I: Iterator> Iterator for IterCounter<'a, I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next().map(|item| {
            *self.count += 1;
            item
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

pub fn part1(input: &str) -> anyhow::Result<u32> {
    let mut signal_count = 0;
    let signals = IterCounter {
        iter: input.lines(),
        count: &mut signal_count,
    };

    let signals = signals.fold(Vec::new(), |mut counts, signal| {
        if counts.is_empty() {
            counts.resize_with(signal.len(), Counts::default);
        }

        counts
            .iter_mut()
            .zip(signal.bytes())
            .filter(|&(_, b)| b == b'1')
            .for_each(|(slot, _)| slot.ones += 1);

        counts
    });

    let (gamma_rate, epsilon_rate): (String, String) = signals
        .iter()
        .map(|column| {
            let zero_count = signal_count - column.ones;
            column.ones > zero_count
        })
        .map(|b| if b { ('1', '0') } else { ('0', '1') })
        .unzip();

    let gamma_rate =
        u32::from_str_radix(&gamma_rate, 2).context("failed to parse binary number")?;
    let epsilon_rate =
        u32::from_str_radix(&epsilon_rate, 2).context("failed to parse binary number")?;

    Ok(gamma_rate * epsilon_rate)
}

/// bit_critera is a function taking (num_zeroes, num_ones, bit)
fn identify_diagnostic_code<'a>(
    mut signals: Vec<&'a str>,
    bit_critera: impl Fn(usize, usize, bool) -> bool,
) -> Option<&'a str> {
    for i in 0.. {
        if let Ok(signal) = signals.iter().exactly_one() {
            return Some(signal);
        }

        let signal_count = signals.len();
        let ones_count = signals
            .iter()
            .map(|signal| signal.as_bytes().get(i))
            .try_fold(0, |count, b| {
                b.map(|&b| if b == b'1' { count + 1 } else { count })
            })?;
        let zeroes_count = signal_count - ones_count;

        signals
            .retain(|signal| bit_critera(zeroes_count, ones_count, signal.as_bytes()[i] == b'1'));
    }

    None
}

trait StrExt {
    fn parse_radix(&self, radix: u32) -> Result<usize, ParseIntError>;
}

impl StrExt for str {
    fn parse_radix(&self, radix: u32) -> Result<usize, ParseIntError> {
        usize::from_str_radix(self, radix)
    }
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    // o2_copy & co2_copy
    let input = input.lines().collect_vec();
    let o2_rating = identify_diagnostic_code(input.clone(), |zerocount, onecount, bit| {
        (zerocount <= onecount) == bit
    })
    .context("no o2 rating found")?
    .parse_radix(2)
    .context("failed to parse o2 rating")?;

    let co2_rating = identify_diagnostic_code(input.clone(), |zerocount, onecount, bit| {
        (onecount < zerocount) == bit
    })
    .context("no co2 rating found")?
    .parse_radix(2)
    .context("failed to parse co2 rating")?;

    eprintln!("o2: {}, co2: {}", o2_rating, co2_rating);

    Ok(o2_rating * co2_rating)
}
