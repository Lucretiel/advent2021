use anyhow::Context;
use gridly::prelude::*;
use gridly_grids::ArrayGrid;

use nom::{
    character::complete::{char, digit1, line_ending, multispace1, space0},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    ParserExt,
};

#[derive(Debug, Copy, Clone)]
struct Cell {
    value: i32,
    mark: bool,
}

#[derive(Copy, Clone)]
struct Board {
    grid: ArrayGrid<Cell, 5, 5>,
    win: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Win {
    score: i32,
}

impl Board {
    pub fn mark_number(&mut self, num: i32) -> Option<Win> {
        // let (cell, location) = Row(0)
        //     .span(Rows(5))
        //     .flat_map(|r| Column(0).span(Columns(5)).map(move |c| r + c))
        //     .find_map(move |loc| self.grid.get_mut(loc).ok().map(|cell| (cell, loc)))?;

        for row in Row(0).span(Rows(5)) {
            for column in Column(0).span(Columns(5)) {
                let cell = self.grid.get_mut(row + column).unwrap();
                if cell.value == num {
                    cell.mark = true;

                    return (self.grid.row(row).unwrap().iter().all(|cell| cell.mark)
                        || self
                            .grid
                            .column(column)
                            .unwrap()
                            .iter()
                            .all(|cell| cell.mark))
                    .then(|| {
                        self.win = true;
                        Win {
                            score: self
                                .grid
                                .rows()
                                .iter()
                                .flat_map(|row| row.iter())
                                .filter(|cell| !cell.mark)
                                .map(|cell| cell.value)
                                .sum::<i32>()
                                * num,
                        }
                    });
                }
            }
        }

        None
    }
}

#[derive(Clone)]
struct Game {
    boards: Vec<Board>,
    balls: Vec<i32>,
}

fn parse_board(input: &str) -> IResult<&str, Board, ErrorTree<&str>> {
    digit1
        .preceded_by(space0)
        .parse_from_str()
        .map(|value| Cell { value, mark: false })
        .context("cell")
        .array()
        .map(|row: [Cell; 5]| row)
        .context("row")
        .separated_array(line_ending)
        .map(ArrayGrid::from_rows)
        .map(|grid| Board { grid, win: false })
        .parse(input)
}

fn parse_input(input: &str) -> IResult<&str, Game, ErrorTree<&str>> {
    collect_separated_terminated(
        digit1.parse_from_str::<i32>().context("ball"),
        char(','),
        line_ending.terminated(line_ending),
    )
    .context("balls")
    .and(
        collect_separated_terminated(
            parse_board.context("board"),
            line_ending.terminated(line_ending),
            multispace1.opt().all_consuming(),
        )
        .context("boards"),
    )
    .map(|(balls, boards)| Game { balls, boards })
    .context("game")
    .parse(input)
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let game: Result<Game, ErrorTree<Location>> = final_parser(parse_input)(input);
    let mut game = game.context("error parsing input into game")?;

    game.balls
        .iter()
        .find_map(|&ball| {
            game.boards
                .iter_mut()
                .find_map(|board| board.mark_number(ball).map(|win| win.score))
        })
        .context("no winning board")
}

pub fn part2(input: &str) -> anyhow::Result<i32> {
    let game: Result<Game, ErrorTree<Location>> = final_parser(parse_input)(input);
    let Game { mut boards, balls } = game.context("error parsing input into game")?;

    balls
        .iter()
        .filter_map(|&ball| {
            boards
                .iter_mut()
                .filter(|board| !board.win)
                .filter_map(|board| board.mark_number(ball).map(|win| win.score))
                .last()
        })
        .last()
        .context("no winning board")
}
