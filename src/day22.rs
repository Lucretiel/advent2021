use std::cmp;

use anyhow::Context;
use enum_map::{enum_map, Enum, EnumMap};
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0, multispace1, space1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{self, final_parser},
    multi::collect_separated_terminated,
    tag::complete::tag,
    ParserExt,
};
use rayon::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Enum)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Location {
    coordinates: EnumMap<Axis, i64>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Range {
    min: i64,
    max: i64,
}

impl Range {
    fn new(min: i64, max: i64) -> Self {
        Range {
            min: cmp::min(min, max),
            max: cmp::max(min, max),
        }
    }

    fn contains(&self, coord: i64) -> bool {
        self.min <= coord && coord <= self.max
    }
}

fn parse_range(input: &str) -> IResult<&str, Range, ErrorTree<&str>> {
    char('-')
        .opt()
        .terminated(digit1)
        .recognize()
        .parse_from_str()
        .separated_array(tag(".."))
        .map(|[min, max]| Range::new(min, max))
        .parse(input)
}

fn parse_named_range<'a>(axis: char) -> impl Parser<&'a str, Range, ErrorTree<&'a str>> {
    char(axis)
        .terminated(char('='))
        .precedes(parse_range.context("range"))
}

#[derive(Debug, Clone, Copy)]
struct Cube {
    ranges: EnumMap<Axis, Range>,
}

impl Cube {
    fn contains(&self, location: Location) -> bool {
        enum_map! {axis => self.ranges[axis].contains(location.coordinates[axis])}
            .values()
            .all(|&b| b)
    }
}

fn parse_cube(input: &str) -> IResult<&str, Cube, ErrorTree<&str>> {
    parse_named_range('x')
        .context("X")
        .terminated(char(','))
        .and(parse_named_range('y').context("Y"))
        .terminated(char(','))
        .and(parse_named_range('z').context("Z"))
        .map(|((x, y), z)| Cube {
            ranges: enum_map! {
                Axis::X => x,
                Axis::Y => y,
                Axis::Z => z,
            },
        })
        .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Off,
    On,
}

impl Default for State {
    fn default() -> Self {
        State::Off
    }
}

fn parse_state(input: &str) -> IResult<&str, State, ErrorTree<&str>> {
    alt((tag("on").value(State::On), tag("off").value(State::Off))).parse(input)
}

#[derive(Debug, Clone, Copy)]
struct Instruction {
    state: State,
    cube: Cube,
}

fn parse_instruction(input: &str) -> IResult<&str, Instruction, ErrorTree<&str>> {
    parse_state
        .context("instruction")
        .terminated(space1)
        .and(parse_cube.context("cube"))
        .map(|(state, cube)| Instruction { state, cube })
        .parse(input)
}

fn parse_instructions(input: &str) -> IResult<&str, Vec<Instruction>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_instruction.context("step"),
        multispace1,
        multispace0.all_consuming(),
    )
    .parse(input)
}

fn final_parse_instructions(
    input: &str,
) -> Result<Vec<Instruction>, ErrorTree<final_parser::Location>> {
    final_parser(parse_instructions)(input)
}

fn compute_location(instructions: &[Instruction], loc: Location) -> State {
    instructions
        .iter()
        .rev()
        .find(|instruction| instruction.cube.contains(loc))
        .map(|instruction| instruction.state)
        .unwrap_or(State::Off)
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let instructions = final_parse_instructions(input).context("failed to parse instructions")?;

    let count = (-50..51)
        .into_par_iter()
        .flat_map_iter(|x| (-50..51).map(move |y| (x, y)))
        .flat_map_iter(|(x, y)| (-50..51).map(move |z| (x, y, z)))
        .map(|(x, y, z)| Location {
            coordinates: enum_map! {
                Axis::X => x,
                Axis::Y => y,
                Axis::Z => z,
            },
        })
        .filter(|&location| compute_location(&instructions, location) == State::On)
        .count();

    Ok(count)
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    todo!()
}
