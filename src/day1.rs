use anyhow::Context;
use itertools::Itertools;

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let numbers: Vec<i32> = input
        .split_whitespace()
        .map(|token| token.parse())
        .try_collect()
        .context("failed to parse integer")?;

    Ok(numbers.windows(2).filter(|pair| pair[0] < pair[1]).count())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let numbers: Vec<i32> = input
        .split_whitespace()
        .map(|token| token.parse())
        .try_collect()
        .context("failed to parse integer")?;

    let chunk_sums: Vec<i32> = numbers
        .windows(3)
        .map(|chunk| chunk.iter().copied().sum())
        .collect();

    Ok(chunk_sums
        .windows(2)
        .filter(|pair| pair[0] < pair[1])
        .count())
}
