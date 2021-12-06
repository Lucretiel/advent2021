use std::collections::HashMap;

use anyhow::Context;
use gridly::prelude::*;
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

    let mut counts: HashMap<Location, usize> = HashMap::new();

    lines
        .iter()
        .filter(|&line| filter(line))
        .flat_map(|line| {
            let unit = Vector {
                rows: line.vec.rows.clamp(Rows(-1), Rows(1)),
                columns: line.vec.columns.clamp(Columns(-1), Columns(1)),
            };

            let magnitude = line.vec.rows.0.abs().max(line.vec.columns.0.abs()) + 1;

            (0..magnitude).map(move |i| line.root + (unit * i))
        })
        .for_each(|loc| *counts.entry(loc).or_default() += 1);

    Ok(counts.values().filter(|&&count| count > 1).count())
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    solve(input, |line| line.vec.direction().is_some())
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    solve(input, |_| true)
}
