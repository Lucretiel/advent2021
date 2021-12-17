use std::iter;

use anyhow::Context;
use gridly::prelude::{GridBounds, GridMut, Location, TOUCHING_ADJACENCIES};
use gridly_grids::ArrayGrid;
use itertools::Itertools;

struct OctopusGrid {
    grid: ArrayGrid<i64, 10, 10>,
}

impl OctopusGrid {
    fn take_step(&mut self) -> i64 {
        let mut flashes: Vec<Location> = Vec::new();
        // increment
        for row in self.grid.row_range() {
            for column in self.grid.column_range() {
                let cell = self.grid.get_mut((row, column)).unwrap();
                *cell += 1;
                if *cell == 10 {
                    flashes.push(row + column);
                }
            }
        }

        let mut count = 0;
        while let Some(flash_loc) = flashes.pop() {
            count += 1;

            for direction in TOUCHING_ADJACENCIES {
                let neighbor_loc = flash_loc + direction;
                if let Ok(neighbor) = self.grid.get_mut(neighbor_loc) {
                    *neighbor += 1;
                    if *neighbor == 10 {
                        flashes.push(neighbor_loc);
                    }
                }
            }
        }

        // reset values > 9 to 0
        for row in self.grid.row_range() {
            for column in self.grid.column_range() {
                let cell = self.grid.get_mut(row + column).unwrap();
                if *cell > 9 {
                    *cell = 0;
                }
            }
        }

        count
    }
}

fn parse_grid(input: &str) -> anyhow::Result<OctopusGrid> {
    let rows = brownstone::try_build_iter(
        input
            .lines()
            .map(|line| {
                brownstone::try_build_iter(
                    line.chars()
                        .map(|c| c.to_digit(10).map(|d| d as i64))
                        .while_some(),
                )
            })
            .while_some(),
    )
    .context("failed to build grid")?;

    Ok(OctopusGrid {
        grid: ArrayGrid::from_rows(rows),
    })
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let mut grid = parse_grid(input)?;

    Ok((0..100).map(move |_| grid.take_step()).sum())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let mut grid = parse_grid(input)?;

    iter::repeat_with(|| grid.take_step())
        .position(|flash_count| flash_count == 100)
        .map(|step| step + 1)
        .context("infinite iterator wasn't infinite :(")
}
