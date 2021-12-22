use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::VecGrid;
use itertools::Itertools;
use pathfinding::directed::astar::astar;

fn parse_map(input: &str) -> anyhow::Result<VecGrid<isize>> {
    let cells: Vec<Vec<isize>> = input
        .lines()
        .enumerate()
        .map(|(row, line)| {
            line.chars()
                .enumerate()
                .map(|(column, c)| {
                    c.to_digit(10)
                        .map(|d| d as isize)
                        .with_context(|| format!("failed to parse digit at column {}", column))
                })
                .try_collect()
                .with_context(|| format!("parse error in row {}", row))
        })
        .try_collect()
        .context("error parsing digit in grid")?;

    VecGrid::new_from_rows(cells).context("inconsistent row length")
}

pub fn part1(input: &str) -> anyhow::Result<isize> {
    let map = parse_map(input).context("error parsing map")?;

    let start = map.root();
    let end = map.outer_bound() - (1, 1);

    astar(
        // Start location
        &start,
        // For a given location, an iterator over the possible next steps to
        // take, along with their costs
        |&location| {
            EACH_DIRECTION
                .iter()
                .map(move |&direction| location + direction)
                .filter_map(|dest| map.get(dest).ok().map(|&cost| (dest, cost)))
        },
        // The approximate cost to get to the destination
        |&location| (end - location).manhattan_length(),
        |&location| location == end,
    )
    .context("no solution found")
    .map(|(_route, cost)| cost)
}

pub fn part2(input: &str) -> anyhow::Result<isize> {
    let tile = parse_map(input).context("error parsing map")?;
    let tile_dimensions = tile.dimensions();

    let map = VecGrid::new_with(tile.dimensions() * 5, |location| {
        let tile_location = Location::new(
            location.row.0 / tile_dimensions.rows.0,
            location.column.0 / tile_dimensions.columns.0,
        );

        let cell_location_in_tile = Location::new(
            location.row.0 % tile_dimensions.rows.0,
            location.column.0 % tile_dimensions.columns.0,
        );

        let base_value = *tile.get(cell_location_in_tile).unwrap();

        let tile_distance = (tile_location - Location::zero()).manhattan_length();

        ((base_value - 1 + tile_distance) % 9) + 1
    })
    .context("grid too large")?;

    let start = map.root();
    let end = map.outer_bound() - (1, 1);

    astar(
        // Start location
        &start,
        // For a given location, an iterator over the possible next steps to
        // take, along with their costs
        |&location| {
            EACH_DIRECTION
                .iter()
                .map(move |&direction| location + direction)
                .filter_map(|dest| map.get(dest).ok().map(|&cost| (dest, cost)))
        },
        // The approximate cost to get to the destination
        |&location| (end - location).manhattan_length(),
        |&location| location == end,
    )
    .context("no solution found")
    .map(|(_route, cost)| cost)
}
