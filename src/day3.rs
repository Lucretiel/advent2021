use anyhow::Context;
use itertools::{self, Itertools};

use crate::library::{IterExt, StrExt};

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

    let gamma_rate: u32 = gamma_rate
        .parse_radix(2)
        .context("failed to parse binary number")?;

    let epsilon_rate: u32 = epsilon_rate
        .parse_radix(2)
        .context("failed to parse binary number")?;

    Ok(gamma_rate * epsilon_rate)
}

/// bit_criteria is a function taking (column_bit, bit)
fn identify_diagnostic_code(
    mut signals: Vec<&str>,
    bit_criteria: impl Fn(bool, bool) -> bool,
) -> Option<&str> {
    for i in 0.. {
        if let Ok(signal) = signals.iter().at_most_one() {
            return signal.copied();
        }

        // Count the true bits in column `i`, but also return `None` if `i`
        // is out of bounds for the column
        let ones_count = signals
            .iter()
            .map(|signal| signal.as_bytes().get(i))
            .map(|bit| bit.ok_or(()))
            .use_oks(|column_bits| column_bits.filter(|&&b| b == b'1').count())
            .ok()?;

        let zeroes_count = signals.len() - ones_count;

        signals.retain(|signal| {
            bit_criteria(ones_count >= zeroes_count, signal.as_bytes()[i] == b'1')
        });
    }

    None
}

fn parse_diagnostic_code(
    signals: Vec<&str>,
    bit_criteria: impl Fn(bool, bool) -> bool,
) -> anyhow::Result<u32> {
    identify_diagnostic_code(signals, bit_criteria)
        .context("no rating found")?
        .parse_radix(2)
        .context("failed to parse rating")
}

pub fn part2(input: &str) -> anyhow::Result<u32> {
    let input = input.lines().collect_vec();

    let o2_rating: u32 = parse_diagnostic_code(input.clone(), |column_bit, signal_bit| {
        column_bit == signal_bit
    })
    .context("error getting o2 rating")?;

    let co2_rating: u32 =
        parse_diagnostic_code(input, |column_bit, signal_bit| column_bit != signal_bit)
            .context("error getting co2 rating")?;

    eprintln!("o2: {}, co2: {}", o2_rating, co2_rating);

    Ok(o2_rating * co2_rating)
}
