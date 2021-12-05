use anyhow::Context;
use itertools::Itertools;

use crate::library::IterExt;

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let numbers: Vec<i32> = input
        .split_whitespace()
        .map(|token| token.parse())
        .try_collect()
        .context("failed to parse integer")?;

    Ok(numbers.windows(2).filter(|pair| pair[0] < pair[1]).count())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    input
        .split_whitespace()
        .map(|token| token.parse::<u32>())
        .use_oks(|numbers| {
            numbers
                .streaming_windows()
                .map(|[a, b, c]| a + b + c)
                .streaming_windows()
                .filter(|[a, b]| a < b)
                .count()
        })
        .context("failed to parse integer")
}
