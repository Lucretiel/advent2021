use std::collections::{HashMap, HashSet};

use anyhow::Context;
use nom::{
    bytes::complete::take_while,
    character::complete::{char, multispace0, multispace1},
    AsChar, IResult, Parser,
};
use nom_supreme::{
    error::ErrorTree,
    final_parser::{final_parser, Location},
    multi::parse_separated_terminated,
    ParserExt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CaveId<'a> {
    Start,
    Big(&'a str),
    Small(&'a str),
    End,
}

impl<'a> CaveId<'a> {
    fn from_str(s: &'a str) -> Self {
        match s {
            "start" => CaveId::Start,
            "end" => CaveId::End,
            s if s.chars().all(|c| c.is_lowercase()) => CaveId::Small(s),
            s => CaveId::Big(s),
        }
    }

    fn small_name(&self) -> Option<&str> {
        match *self {
            CaveId::Small(s) => Some(s),
            _ => None,
        }
    }
}

fn parse_cave_id(input: &str) -> IResult<&str, CaveId<'_>, ErrorTree<&str>> {
    take_while(|c: char| c.is_alpha())
        .map(CaveId::from_str)
        .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Link<'a> {
    head: CaveId<'a>,
    tail: CaveId<'a>,
}

fn parse_link(input: &str) -> IResult<&str, Link<'_>, ErrorTree<&str>> {
    parse_cave_id
        .terminated(char('-'))
        .and(parse_cave_id)
        .map(|(head, tail)| Link { head, tail })
        .parse(input)
}

#[derive(Debug, Clone, Default)]
struct CaveMap<'a> {
    links: HashMap<CaveId<'a>, HashSet<CaveId<'a>>>,
}

fn parse_cave_map(input: &str) -> IResult<&str, CaveMap<'_>, ErrorTree<&str>> {
    parse_separated_terminated(
        parse_link,
        multispace1,
        multispace0.all_consuming(),
        CaveMap::default,
        |mut map, Link { head, tail }| {
            map.links.entry(head).or_default().insert(tail);
            map.links.entry(tail).or_default().insert(head);
            map
        },
    )
    .parse(input)
}

fn final_parse_cave_map(input: &str) -> Result<CaveMap<'_>, ErrorTree<Location>> {
    final_parser(parse_cave_map)(input)
}

struct SmallCaveChain<'a> {
    id: &'a str,
    prev: Option<&'a SmallCaveChain<'a>>,
}

impl SmallCaveChain<'_> {
    fn contains(&self, name: &str) -> bool {
        self.id == name
            || match self.prev {
                Some(prev) => prev.contains(name),
                None => false,
            }
    }
}

fn count_routes_from(
    map: &CaveMap,
    start: CaveId,
    small_caves: Option<&SmallCaveChain<'_>>,
) -> usize {
    if start == CaveId::End {
        return 1;
    }

    let destinations = map
        .links
        .get(&start)
        .unwrap_or_else(|| panic!("Unexpected uni-directional link to cave {:?}", start));

    destinations
        .iter()
        .filter(|&dest| match (*dest, small_caves) {
            (CaveId::Small(name), Some(small_caves)) => !small_caves.contains(name),
            (CaveId::Start, _) => false,
            _ => true,
        })
        .map(|&dest| match dest.small_name() {
            None => count_routes_from(map, dest, small_caves),
            Some(name) => {
                let small_caves = SmallCaveChain {
                    id: name,
                    prev: small_caves,
                };
                count_routes_from(map, dest, Some(&small_caves))
            }
        })
        .sum()
}

pub fn part1(input: &str) -> anyhow::Result<usize> {
    let map = final_parse_cave_map(input).context("parse error")?;
    Ok(count_routes_from(&map, CaveId::Start, None))
}

fn count_routes_from_visit_twice(
    map: &CaveMap,
    start: CaveId,
    small_caves: Option<&SmallCaveChain<'_>>,
    any_doubled: bool,
) -> usize {
    if start == CaveId::End {
        return 1;
    }

    let destinations = map
        .links
        .get(&start)
        .unwrap_or_else(|| panic!("Unexpected uni-directional link to cave {:?}", start));

    destinations
        .iter()
        .filter(|&dest| match (*dest, small_caves, any_doubled) {
            // Only visit the start node once
            (CaveId::Start, ..) => false,

            // If we've visited any small cave twice, visited small caves are now off limits
            (CaveId::Small(name), Some(small_caves), true) => !small_caves.contains(name),

            // All other nodes can freely be revisited
            _ => true,
        })
        .map(|&dest| match dest.small_name() {
            None => count_routes_from_visit_twice(map, dest, small_caves, any_doubled),
            Some(name) => {
                let any_doubled = any_doubled
                    || match small_caves {
                        Some(caves) => caves.contains(name),
                        None => false,
                    };

                let small_caves = SmallCaveChain {
                    id: name,
                    prev: small_caves,
                };

                count_routes_from_visit_twice(map, dest, Some(&small_caves), any_doubled)
            }
        })
        .sum()
}

pub fn part2(input: &str) -> anyhow::Result<usize> {
    let map = final_parse_cave_map(input).context("parse error")?;
    Ok(count_routes_from_visit_twice(
        &map,
        CaveId::Start,
        None,
        false,
    ))
}
