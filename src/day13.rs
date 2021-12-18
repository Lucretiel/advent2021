use std::{cmp::max, collections::HashSet};

use anyhow::Context;
use joinery::JoinableIterator;
use nom::{
    branch::alt,
    character::complete::{char, digit1, line_ending, multispace0},
    sequence::pair,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{self, final_parser},
    multi::collect_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct Location {
    x: i32,
    y: i32,
}

impl Location {
    fn edit_axis(self, axis: Axis, body: impl Fn(i32) -> i32) -> Location {
        match axis {
            Axis::X => Location {
                y: self.y,
                x: body(self.x),
            },
            Axis::Y => Location {
                x: self.x,
                y: body(self.y),
            },
        }
    }
}

fn parse_coordinate(input: &str) -> IResult<&str, i32, ErrorTree<&str>> {
    digit1.parse_from_str_cut().parse(input)
}

fn parse_location(input: &str) -> IResult<&str, Location, ErrorTree<&str>> {
    parse_coordinate
        .context("x")
        .terminated(char(','))
        .and(parse_coordinate.context("y"))
        .map(|(x, y)| Location { x, y })
        .parse(input)
}

fn parse_location_set<T: Extend<Location> + Default>(
    input: &str,
) -> IResult<&str, T, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_location.context("location"),
        line_ending,
        pair(line_ending, line_ending),
    )
    .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FoldInstruction {
    axis: Axis,
    edge: i32,
}

fn parse_axis(input: &str) -> IResult<&str, Axis, ErrorTree<&str>> {
    alt((char('x').value(Axis::X), char('y').value(Axis::Y))).parse(input)
}

fn parse_fold_instruction(input: &str) -> IResult<&str, FoldInstruction, ErrorTree<&str>> {
    parse_axis
        .context("axis")
        .terminated(char('='))
        .and(parse_coordinate.context("dimension"))
        .preceded_by(tag("fold along "))
        .map(|(axis, dimension)| FoldInstruction {
            axis,
            edge: dimension,
        })
        .parse(input)
}

fn parse_fold_list(input: &str) -> IResult<&str, Vec<FoldInstruction>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_fold_instruction.context("fold instruction"),
        line_ending,
        multispace0.all_consuming(),
    )
    .parse(input)
}

#[derive(Debug, Clone, Default)]
struct Page {
    dots: HashSet<Location>,
}

impl Page {
    fn apply_fold(&mut self, fold: FoldInstruction) {
        self.dots = self
            .dots
            .drain()
            .map(move |loc| loc.edit_axis(fold.axis, |value| fold.edge - (value - fold.edge).abs()))
            .collect();
    }
}

impl Extend<Location> for Page {
    fn extend<T: IntoIterator<Item = Location>>(&mut self, iter: T) {
        self.dots.extend(iter)
    }
}

fn parse_problem(input: &str) -> IResult<&str, (Page, Vec<FoldInstruction>), ErrorTree<&str>> {
    parse_location_set
        .context("dots")
        .and(parse_fold_list.context("fold list"))
        .parse(input)
}

fn final_parse_problem(
    input: &str,
) -> Result<(Page, Vec<FoldInstruction>), ErrorTree<final_parser::Location>> {
    final_parser(parse_problem)(input)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let (mut page, instructions) = final_parse_problem(input).context("parse error")?;
    let first = *instructions.first().context("no instructions in list")?;
    page.apply_fold(first);
    Ok(page.dots.len())
}

pub fn part2(input: &str) -> anyhow::Result<String> {
    let (mut page, instructions) = final_parse_problem(input).context("parse error")?;

    instructions
        .iter()
        .for_each(|&instruction| page.apply_fold(instruction));

    let max_coords = page
        .dots
        .iter()
        .fold(Location::default(), |corner, &dot| Location {
            x: max(corner.x, dot.x),
            y: max(corner.y, dot.y),
        });

    Ok((0..=max_coords.y)
        .map(|y| {
            (0..=max_coords.x)
                .map(move |x| Location { x, y })
                .map(|loc| match page.dots.contains(&loc) {
                    true => '█',
                    false => ' ',
                })
                .join_concat()
        })
        .join_with('\n')
        // TODO: Find a way to get rid of this to_string
        .to_string())
}
