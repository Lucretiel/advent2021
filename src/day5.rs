use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::SparseGrid;
use nom::{
    character::complete::{char, digit1, multispace0, multispace1, space0},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{self, final_parser},
    multi::collect_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Line {
    root: Location,
    vec: Vector,
}

fn parse_location(input: &str) -> IResult<&str, Location, ErrorTree<&str>> {
    digit1
        .parse_from_str()
        .separated_array(char(','))
        .map(|[row, column]| Row(row) + Column(column))
        .parse(input)
}

fn parse_line(input: &str) -> IResult<&str, Line, ErrorTree<&str>> {
    parse_location
        .separated_array(tag("->").delimited_by(space0))
        .map(|[root, finish]| Line {
            root,
            vec: finish - root,
        })
        .parse(input)
}

fn parse_all_lines(input: &str) -> IResult<&str, Vec<Line>, ErrorTree<&str>> {
    collect_separated_terminated(parse_line, multispace1, multispace0.all_consuming()).parse(input)
}

fn final_parse_all_lines(input: &str) -> Result<Vec<Line>, ErrorTree<final_parser::Location>> {
    final_parser(parse_all_lines)(input)
}

fn solve(input: &str, filter: impl Fn(&Line) -> bool) -> anyhow::Result<usize> {
    let lines = final_parse_all_lines(input).context("failed to parse lines")?;

    let outer_root = lines
        .iter()
        .flat_map(|line| [line.root, line.root + line.vec])
        .fold(Location::zero(), |best, loc| Location {
            row: best.row.max(loc.row),
            column: best.column.max(loc.column),
        });

    let mut grid: SparseGrid<i32> =
        SparseGrid::new(outer_root - Location::zero() + Rows(1) + Columns(1));

    for line in lines.iter().filter(|&line| filter(line)) {
        let unit = Vector {
            rows: line.vec.rows.clamp(Rows(-1), Rows(1)),
            columns: line.vec.columns.clamp(Columns(-1), Columns(1)),
        };

        let magnitude = line.vec.rows.0.abs().max(line.vec.columns.0.abs()) + 1;

        for i in 0..magnitude {
            *grid
                .get_mut(line.root + (unit * i))
                .ok()
                .context("out of bounds??")? += 1;
        }
    }

    Ok(grid
        .occupied_entries()
        .filter(|&(_, &value)| value > 1)
        .count())
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    solve(input, |line| line.vec.direction().is_some())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    solve(input, |_| true)
}
