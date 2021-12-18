use std::{collections::HashSet, iter};

use anyhow::Context;
use gridly::prelude::{GridBounds, GridMut, GridSetter, Location, TOUCHING_ADJACENCIES};
use gridly_grids::ArrayGrid;
use itertools::Itertools;

struct OctopusGrid {
    grid: ArrayGrid<i64, 10, 10>,

    // Store the buffers used in `take_step` so that they can be reused over
    // several steps
    increment_buffer: Vec<Location>,
    flash_buffer: HashSet<Location>,
}

impl OctopusGrid {
    fn from_rows(rows: [[i64; 10]; 10]) -> Self {
        Self {
            grid: ArrayGrid::from_rows(rows),
            increment_buffer: Vec::with_capacity(100),
            flash_buffer: HashSet::new(),
        }
    }

    fn take_step(&mut self) -> usize {
        // Clear buffers
        self.increment_buffer.clear();
        self.flash_buffer.clear();

        // Initial state: all octopuses will increment
        self.increment_buffer.extend(
            self.grid
                .row_range()
                .flat_map(|row| self.grid.column_range().map(move |column| row + column)),
        );

        // Increment octopuses and resolve flashes
        while let Some(increment_loc) = self.increment_buffer.pop() {
            if let Ok(cell) = self.grid.get_mut(increment_loc) {
                *cell += 1;
                if *cell == 10 {
                    // Record a flash
                    self.flash_buffer.insert(increment_loc);

                    // All adjacent octopuses will increment again
                    self.increment_buffer
                        .extend(TOUCHING_ADJACENCIES.iter().map(|&dir| increment_loc + dir))
                }
            }
        }

        // Reset flashes
        self.flash_buffer
            .iter()
            .for_each(|&flash_loc| self.grid.set(flash_loc, 0).expect("out of bounds flash"));

        self.flash_buffer.len()
    }
}

fn parse_grid(input: &str) -> anyhow::Result<OctopusGrid> {
    brownstone::try_build_iter(
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
    .context("failed to build grid")
    .map(OctopusGrid::from_rows)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
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
