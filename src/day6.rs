use std::collections::HashMap;

use anyhow::Context;
use itertools::Itertools;

struct FishCounter {
    population: HashMap<i32, i64>,
}

impl FromIterator<i32> for FishCounter {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut population = HashMap::new();
        iter.into_iter()
            .for_each(|item| *population.entry(item).or_default() += 1);
        Self { population }
    }
}

pub fn solve(input: &str, days: i32) -> anyhow::Result<i64> {
    let mut counter: FishCounter = input
        .split(",")
        .map(|day| day.parse().context("failed to parse day"))
        .try_collect()?;

    for day in 0..days {
        if let Some(day_count) = counter.population.remove(&day) {
            // The fish create new fish
            *counter.population.entry(day + 9).or_default() += day_count;

            // Then they get sleepy
            *counter.population.entry(day + 7).or_default() += day_count;
        }
    }

    Ok(counter.population.values().copied().sum())
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    solve(input, 80)
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    solve(input, 256)
}
