use anyhow::Context;
use nom::{
    branch::alt,
    character::complete::{char, digit1, multispace0},
    IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::collect_separated_terminated,
    ParserExt,
};

#[derive(Debug, Clone)]
enum Element {
    Regular(i64),
    Pair(Box<Pair>),
}

impl Element {
    fn new_pair(left: i64, right: i64) -> Self {
        Element::Pair(Box::new(Pair::new(left, right)))
    }

    fn get_regular(&self) -> Option<i64> {
        match *self {
            Element::Regular(value) => Some(value),
            Element::Pair(..) => None,
        }
    }

    fn get_regular_pair(&self) -> Option<[i64; 2]> {
        match self {
            Element::Regular(..) => None,
            Element::Pair(pair) => pair.get_regular_pair(),
        }
    }

    fn begin_explode(&mut self, left_receiver: Option<&mut i64>, depth: i32) -> ExplodeOutcome {
        // Check if this element can explode
        if depth >= 4 {
            if let Some([left_payload, right_payload]) = self.get_regular_pair() {
                // Explode! Replace self with 0, and send the payloads outward
                *self = Element::Regular(0);

                if let Some(left_receiver) = left_receiver {
                    *left_receiver += left_payload;
                }

                return ExplodeOutcome::ExplodeBegun(right_payload);
            }
        }

        // No explosion happening here, so resolve recursion
        match self {
            Element::Regular(value) => ExplodeOutcome::NewLeftReceiver(value),
            Element::Pair(pair) => pair.begin_explode(left_receiver, depth),
        }
    }

    fn finish_explode(&mut self, payload: i64) {
        match self {
            Element::Regular(value) => *value += payload,
            Element::Pair(pair) => pair.finish_explode(payload),
        }
    }

    fn split(&mut self) -> SplitOutcome {
        match *self {
            Element::Regular(value) if value >= 10 => {
                let left = value / 2;
                let right = value / 2 + value % 2;

                *self = Self::new_pair(left, right);
                SplitOutcome::SplitFinished
            }
            Element::Regular(_) => SplitOutcome::Nothing,
            Element::Pair(ref mut pair) => pair.split(),
        }
    }

    fn magnitude(&self) -> i64 {
        match *self {
            Element::Regular(value) => value,
            Element::Pair(ref pair) => pair.magnitude(),
        }
    }
}

#[derive(Debug, Clone)]
struct Pair {
    elements: [Element; 2],
}

impl Pair {
    fn new(left: i64, right: i64) -> Self {
        Self {
            elements: [Element::Regular(left), Element::Regular(right)],
        }
    }

    fn get_regular_pair(&self) -> Option<[i64; 2]> {
        let [left, right] = &self.elements;

        Some([left.get_regular()?, right.get_regular()?])
    }

    fn add(self, other: Self) -> Self {
        let mut paired = Self {
            elements: [
                Element::Pair(Box::new(self)),
                Element::Pair(Box::new(other)),
            ],
        };

        paired.reduce();
        paired
    }

    fn begin_explode(&mut self, left_receiver: Option<&mut i64>, depth: i32) -> ExplodeOutcome {
        let [left, right] = &mut self.elements;

        match left.begin_explode(left_receiver, depth + 1) {
            ExplodeOutcome::NewLeftReceiver(left_receiver) => {
                right.begin_explode(Some(left_receiver), depth + 1)
            }
            ExplodeOutcome::ExplodeBegun(right_payload) => {
                right.finish_explode(right_payload);
                ExplodeOutcome::ExplodeFinished
            }
            ExplodeOutcome::ExplodeFinished => ExplodeOutcome::ExplodeFinished,
        }
    }

    fn finish_explode(&mut self, payload: i64) {
        self.elements[0].finish_explode(payload)
    }

    // Returns true if a split happened
    fn split(&mut self) -> SplitOutcome {
        let [left, right] = &mut self.elements;

        match left.split() {
            SplitOutcome::Nothing => right.split(),
            SplitOutcome::SplitFinished => SplitOutcome::SplitFinished,
        }
    }

    fn reduce(&mut self) {
        loop {
            match self.begin_explode(None, 0) {
                // If the best we could find was a left receiver, no explosion
                // happened. Attempt a split instead.
                ExplodeOutcome::NewLeftReceiver(..) => match self.split() {
                    SplitOutcome::Nothing => break,
                    SplitOutcome::SplitFinished => continue,
                },
                ExplodeOutcome::ExplodeBegun(..) | ExplodeOutcome::ExplodeFinished => continue,
            }
        }
    }

    fn magnitude(&self) -> i64 {
        let [left, right] = &self.elements;
        (left.magnitude() * 3) + (right.magnitude() * 2)
    }
}

#[derive(Debug)]
enum ExplodeOutcome<'a> {
    // A new receiver for the left side of the explode
    NewLeftReceiver(&'a mut i64),

    // The explode happened; this is the payload for the right side
    ExplodeBegun(i64),

    // An explode completed
    ExplodeFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitOutcome {
    Nothing,
    SplitFinished,
}

fn parse_pair(input: &str) -> IResult<&str, Pair, ErrorTree<&str>> {
    parse_element
        .context("left element")
        .terminated(char(',').delimited_by(multispace0))
        .and(parse_element.context("right element"))
        .terminated(char(']'))
        .cut()
        .preceded_by(char('['))
        .map(|(left, right)| Pair {
            elements: [left, right],
        })
        .parse(input)
}

fn parse_element(input: &str) -> IResult<&str, Element, ErrorTree<&str>> {
    alt((
        digit1
            .parse_from_str_cut()
            .map(Element::Regular)
            .context("regular value"),
        parse_pair.map(Box::new).map(Element::Pair).context("pair"),
    ))
    .parse(input)
}

fn parse_pair_list(input: &str) -> IResult<&str, Vec<Pair>, ErrorTree<&str>> {
    collect_separated_terminated(
        parse_pair.context("pair"),
        multispace0,
        multispace0.all_consuming(),
    )
    .parse(input)
}

fn final_parse_pair_list(input: &str) -> Result<Vec<Pair>, ErrorTree<Location>> {
    final_parser(parse_pair_list)(input)
}

pub fn part1(input: &str) -> anyhow::Result<i64> {
    let pairs = final_parse_pair_list(input).context("parse error")?;
    pairs
        .into_iter()
        .reduce(Pair::add)
        .context("no pairs in input")
        .map(|pair| pair.magnitude())
}

pub fn part2(input: &str) -> anyhow::Result<i64> {
    let pairs = final_parse_pair_list(input).context("parse error")?;

    pairs
        .iter()
        .enumerate()
        .flat_map(|(i1, first)| {
            pairs
                .iter()
                .enumerate()
                .filter(move |&(i2, _)| i1 != i2)
                .map(move |(_, second)| (first, second))
        })
        .map(|(p1, p2)| Pair::add(p1.clone(), p2.clone()))
        .map(|sum| sum.magnitude())
        .max()
        .context("no pairs in input")
}
