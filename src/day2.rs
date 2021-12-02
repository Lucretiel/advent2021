use anyhow::Context;
use nom::{
    branch::alt,
    character::complete::{char, digit1, space1},
    combinator::eof,
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    tag::complete::tag,
    ParserExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Forward,
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cmd {
    direction: Direction,
    distance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Position {
    horizontal: i32,
    depth: i32,
    aim: i32,
}

fn parse_direction(input: &str) -> IResult<&str, Direction, ErrorTree<&str>> {
    alt((
        tag("forward").value(Direction::Forward),
        tag("down").value(Direction::Down),
        tag("up").value(Direction::Up),
    ))
    .context("direction")
    .parse(input)
}

fn parse_cmd(input: &str) -> IResult<&str, Cmd, ErrorTree<&str>> {
    parse_direction
        .terminated(space1)
        .and(digit1.parse_from_str().context("distance"))
        .map(|(direction, distance)| Cmd {
            direction,
            distance,
        })
        .context("command")
        .parse(input)
}

fn parse_cmd_list<'a>(
    func: impl Fn(Position, Cmd) -> Position,
) -> impl Parser<&'a str, Position, ErrorTree<&'a str>> {
    parse_separated_terminated(parse_cmd, char('\n'), eof, Position::default, func)
}

fn solve(input: &str, func: impl Fn(Position, Cmd) -> Position) -> anyhow::Result<i32> {
    let mut parser = final_parser(parse_cmd_list(func));
    let final_pos: Result<Position, ErrorTree<Location>> = parser(input.trim_end());
    let final_pos = final_pos.context("parse error")?;
    Ok(final_pos.depth * final_pos.horizontal)
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    solve(input, |pos, cmd| match cmd.direction {
        Direction::Forward => Position {
            horizontal: pos.horizontal + cmd.distance,
            ..pos
        },
        Direction::Down => Position {
            depth: pos.depth + cmd.distance,
            ..pos
        },
        Direction::Up => Position {
            depth: pos.depth - cmd.distance,
            ..pos
        },
    })
}

pub fn part2(input: &str) -> anyhow::Result<i32> {
    solve(input, |pos, cmd| match cmd.direction {
        Direction::Forward => Position {
            horizontal: pos.horizontal + cmd.distance,
            depth: pos.depth + (pos.aim * cmd.distance),
            ..pos
        },
        Direction::Down => Position {
            aim: pos.aim + cmd.distance,
            ..pos
        },
        Direction::Up => Position {
            aim: pos.aim - cmd.distance,
            ..pos
        },
    })
}
