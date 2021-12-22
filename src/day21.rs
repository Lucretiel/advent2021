use std::iter;

use anyhow::Context;
use enum_map::{enum_map, Enum, EnumMap};
use nom::{
    character::complete::{digit1, multispace0, multispace1},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    tag::complete::tag,
    ParserExt,
};
use rayon::prelude::*;

use crate::library::{Counter, IterExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Hash)]
enum Player {
    One,
    Two,
}

impl Player {
    fn other(self) -> Self {
        match self {
            Player::One => Player::Two,
            Player::Two => Player::One,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Position {
    idx: u8,
}

impl Position {
    fn new(position: i64) -> Self {
        Self {
            idx: ((position - 1) % 10) as u8,
        }
    }
    fn value(&self) -> i64 {
        (self.idx + 1).into()
    }

    fn add(&mut self, amount: i64) {
        self.idx = ((self.idx as i64 + amount) % 10) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    position: Position,
    score: i64,
}

impl State {
    fn new(position: Position) -> Self {
        Self { position, score: 0 }
    }
    fn do_move(&mut self, amount: i64, winning_score: i64) -> Option<Win> {
        self.position.add(amount);
        self.score += self.position.value();

        (self.score >= winning_score).then(|| Win)
    }
}

#[derive(Debug, Clone, Copy)]
struct Win;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Game {
    players: EnumMap<Player, State>,
}

impl Game {
    fn new(player1: Position, player2: Position) -> Self {
        Self {
            players: enum_map! {
                Player::One => State::new(player1),
                Player::Two => State::new(player2),
            },
        }
    }
    fn do_move(&mut self, player: Player, amount: i64, winning_score: i64) -> Option<Win> {
        self.players[player].do_move(amount, winning_score)
    }

    fn play(&mut self, dice: impl Iterator<Item = i64>, winning_score: i64) -> Option<GameOutcome> {
        let mut dice = IterCounter::new(dice);
        let mut dice_sums = dice.by_ref().streaming_chunks().map(|[a, b, c]| a + b + c);

        loop {
            for player in [Player::One, Player::Two] {
                let total = dice_sums.next()?;

                if let Some(Win) = self.do_move(player, total, winning_score) {
                    return Some(GameOutcome {
                        winner: player,
                        scores: enum_map! {player => self.players[player].score},
                        dice_rolled: dice.count,
                    });
                }
            }
        }
    }
}

struct GameOutcome {
    winner: Player,
    scores: EnumMap<Player, i64>,
    dice_rolled: usize,
}

#[derive(Debug, Clone)]
struct IterCounter<I> {
    pub count: usize,
    iter: I,
}

impl<I: Iterator> IterCounter<I> {
    fn new(iter: I) -> Self {
        Self { iter, count: 0 }
    }
}

impl<I: Iterator> Iterator for IterCounter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|item| {
            self.count += 1;
            item
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

fn parse_position<'a>(
    player_id: &'static str,
) -> impl Parser<&'a str, Position, ErrorTree<&'a str>> {
    tag("Player ")
        .terminated(tag(player_id))
        .terminated(tag(" starting position: "))
        .precedes(digit1)
        .parse_from_str()
        .map(Position::new)
}

fn parse_game(input: &str) -> IResult<&str, Game, ErrorTree<&str>> {
    parse_position("1")
        .terminated(multispace1)
        .and(parse_position("2"))
        .terminated(multispace0)
        .map(|(player1, player2)| Game::new(player1, player2))
        .parse(input)
}

fn final_parse_game(input: &str) -> Result<Game, ErrorTree<Location>> {
    final_parser(parse_game)(input)
}

struct DeterministicDice {
    next: i64,
}

impl DeterministicDice {
    fn new() -> Self {
        Self { next: 0 }
    }
}

impl Iterator for DeterministicDice {
    type Item = i64;

    fn next(&mut self) -> Option<i64> {
        self.next += 1;
        let next = self.next;
        self.next %= 100;

        Some(next)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let mut game = final_parse_game(input).context("failed to parse game")?;

    game.play(DeterministicDice::new(), 1000)
        .context("dice ran out of dice")
        .map(|outcome| outcome.scores[outcome.winner.other()] * outcome.dice_rolled as i64)
}

#[derive(Debug, Clone)]
struct Multiverse {
    next_to_play: Player,
    states: Counter<Game>,
    wins: Counter<Player>,
}

impl Multiverse {
    fn step(self) -> Self {
        let dice = [
            3, 4, 5, 4, 5, 6, 5, 6, 7, 4, 5, 6, 5, 6, 7, 6, 7, 8, 5, 6, 7, 6, 7, 8, 7, 8, 9,
        ];

        // Iterator of ((Game, count), (winning player, count))
        let game_events = dice.iter().flat_map(|&total_roll| {
            self.states.iter_counts().map(move |(game, count)| {
                let mut game = *game;

                if let Some(Win) = game.do_move(self.next_to_play, total_roll, 21) {
                    // If there's a win, remove these games from existence, and
                    // log the wins
                    ((game, 0), (self.next_to_play, count))
                } else {
                    // Otherwise, add new games to the multiverse
                    ((game, count), (self.next_to_play, 0))
                }
            })
        });

        let (new_states, new_wins) = game_events.unzip();

        Self {
            next_to_play: self.next_to_play.other(),
            states: new_states,
            wins: self.wins.merge(new_wins),
        }
    }

    fn new(initial_game: Game) -> Self {
        Self {
            next_to_play: Player::One,
            wins: Counter::new(),
            states: iter::once(initial_game).collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.states
            .iter_counts()
            .map(|(_, count)| count)
            .sum::<usize>()
            == 0
    }
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let initial_game = final_parse_game(input).context("failed to parse game")?;
    let mut multiverse = Multiverse::new(initial_game);

    while !multiverse.is_empty() {
        multiverse = multiverse.step();
    }

    Ok(multiverse
        .wins
        .iter_counts()
        .map(|(_, wins)| wins)
        .max()
        .unwrap())
}
