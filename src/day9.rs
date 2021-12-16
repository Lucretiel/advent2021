use std::{
    collections::{BinaryHeap, HashMap},
    iter,
};

use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::VecGrid;
use itertools::Itertools;

use crate::library::Counter;

fn parse_grid(input: &str) -> anyhow::Result<VecGrid<i32>> {
    let rows: Vec<Vec<i32>> = input
        .lines()
        .map(|line| {
            line.chars()
                .map(|c| c.to_digit(10).context("parsing digit"))
                .map_ok(|digit| digit.try_into().unwrap())
                .try_collect()
        })
        .try_collect()?;

    VecGrid::new_from_rows(rows).context("inconsistent row length")
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let grid = parse_grid(input)?;

    Ok(grid
        .rows()
        .iter()
        .flat_map(|row| row.iter_with_locations())
        .filter(|&(loc, &cell)| {
            EACH_DIRECTION
                .iter()
                .map(|&dir| loc + dir)
                .filter_map(|neighbor_loc| grid.get(neighbor_loc).ok())
                .all(|&neighbor| neighbor > cell)
        })
        .map(|(_, &min)| min + 1)
        .sum())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BasinId {
    low_point: Location,
}

// returning the basin that input
fn identify_basin(
    height: i32,
    input: Location,
    grid: &VecGrid<i32>,
    basins: &mut HashMap<Location, BasinId>,
) -> BasinId {
    // Find the value and location of the neighbor with the lowest height,
    // or None if this is the lowest
    let min_neighbor = EACH_DIRECTION
        .iter()
        .map(|&dir| input + dir)
        .filter_map(|neighbor_loc| grid.get(neighbor_loc).map(|cell| (cell, neighbor_loc)).ok())
        .filter(|&(&ncell, _)| ncell < height)
        .min_by_key(|&(&ncell, _)| ncell);

    let basin_id = match min_neighbor {
        // Found a lower neighbor; identify the basin associated with it
        Some((&neighbor_height, neighbor_location)) => match basins.get(&neighbor_location) {
            Some(&basin_id) => basin_id,
            None => identify_basin(neighbor_height, neighbor_location, grid, basins),
        },
        // There are no lower neighbors; this location is the basin.
        None => BasinId { low_point: input },
    };

    basins.insert(input, basin_id);
    basin_id
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let grid = parse_grid(input)?;
    // key - location :: value - basin_id
    let mut basins: HashMap<Location, BasinId> = HashMap::new();

    grid.rows()
        .iter()
        .flat_map(|row| row.iter_with_locations())
        .filter(|&(_, &cell)| cell < 9)
        .for_each(|(loc, &cell)| {
            identify_basin(cell, loc, &grid, &mut basins);
        });

    let basin_counts: Counter<BasinId> = basins.values().copied().collect();

    let mut sorted_counts: BinaryHeap<usize> =
        basin_counts.iter_counts().map(|(_, count)| count).collect();

    Ok(iter::from_fn(|| sorted_counts.pop()).take(3).product())
}
