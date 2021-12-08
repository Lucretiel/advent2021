use std::collections::{btree_map::Entry, BTreeMap};

use anyhow::Context;

use crate::library::parse_input_iter;

#[derive(Debug, Clone)]
struct CrabList {
    crab_counts: BTreeMap<i32, i32>,
}

impl FromIterator<i32> for CrabList {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut crab_counts = BTreeMap::new();
        iter.into_iter()
            .for_each(|item| *crab_counts.entry(item).or_default() += 1);
        Self { crab_counts }
    }
}

#[derive(Debug, Clone, Copy)]
struct FormationFlank {
    position: i32,
    count: i32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Outcome {
    Done,
    NotDone,
}

impl FormationFlank {
    // Send the crabs to a new position, update the count, and return the
    // expended fuel
    fn advance_to(&mut self, target: FormationFlank) -> i32 {
        let distance = (self.position - target.position).abs();
        let fuel = distance * self.count;

        self.position = target.position;
        self.count += target.count;

        fuel
    }
}

pub fn part1(input: &str) -> anyhow::Result<i32> {
    let crabs: CrabList = parse_input_iter(input.split(",")).context("failed to parse input")?;

    let mut fuel = 0;

    let mut crab_iter = crabs
        .crab_counts
        .range(..)
        .map(|(&position, &count)| FormationFlank { position, count });

    let mut left_flank = crab_iter.next().context("There are no crabs 🙁")?;

    let mut right_flank = match crab_iter.next_back() {
        Some(flank) => flank,
        None => return Ok(fuel),
    };

    loop {
        // Figure out which side is moving. The side with fewer 🦀🦀🦀🦀 needs
        // to move towards the center. Suppose it is the left flank, the
        // logic works the same either way. The left flank needs to move towars
        // a target flank; this is either the next leftmost flank or the right
        // flank; in the latter case, this will be the final move.
        let (mobile_flank, target_flank, outcome) = match match left_flank.count
            <= right_flank.count
        {
            // The left side will be moving
            true => (&mut left_flank, right_flank, crab_iter.next()),

            // The right side will be moving
            false => (&mut right_flank, left_flank, crab_iter.next_back()),
        } {
            // The mobile side will reach some inner destination
            (mobile_flank, _, Some(next_flank)) => (mobile_flank, next_flank, Outcome::NotDone),

            // The mobile side will reach the opposite end; this is the final move
            // The crabs move and are all together now 🦀🦀🦀🦀
            (mobile_flank, immobile_flank, None) => (mobile_flank, immobile_flank, Outcome::Done),
        };

        let spent = mobile_flank.advance_to(target_flank);
        fuel += spent;
        if outcome == Outcome::Done {
            break Ok(fuel);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CrabCohort {
    count: i32,
    fuel_cost: i32,
}

impl CrabCohort {
    fn step(&mut self) -> i32 {
        let step_cost = self.fuel_cost * self.count;
        self.fuel_cost += 1;
        step_cost
    }
    fn step_predict(&self) -> i32 {
        self.fuel_cost * self.count
    }
}

#[derive(Debug, Clone)]
struct CrabCahoot {
    cohorts: Vec<CrabCohort>,
}

impl CrabCahoot {
    fn step(&mut self) -> i32 {
        self.cohorts.iter_mut().map(|cohort| cohort.step()).sum()
    }
    fn step_predict(&self) -> i32 {
        self.cohorts
            .iter()
            .map(|cohort| cohort.step_predict())
            .sum()
    }
    fn merge(&mut self, cahoot: CrabCahoot) {
        self.cohorts.extend(cahoot.cohorts)
    }
}

#[derive(Debug, Clone)]
struct CrabPopulation {
    population: BTreeMap<i32, CrabCahoot>,
}

impl FromIterator<i32> for CrabPopulation {
    fn from_iter<T: IntoIterator<Item = i32>>(iter: T) -> Self {
        let mut population: BTreeMap<i32, CrabCahoot> = BTreeMap::new();
        iter.into_iter().for_each(|position| {
            population
                .entry(position)
                .and_modify(|cahoot| cahoot.cohorts[0].count += 1)
                .or_insert(CrabCahoot {
                    cohorts: vec![CrabCohort {
                        count: 1,
                        fuel_cost: 1,
                    }],
                });
        });
        Self { population }
    }
}

pub fn part2(input: &str) -> anyhow::Result<i32> {
    let mut crabs: CrabPopulation =
        parse_input_iter(input.split(",")).context("failed to parse input")?;

    let mut fuel = 0;

    loop {
        let mut range = crabs.population.range(..);

        let (&left_flank, left_cahoot) = range.next().context("there were no crabs :(")?;

        let (&right_flank, right_cahoot) = match range.next_back() {
            Some(entry) => entry,
            None => break Ok(fuel),
        };

        let (origin, destination) = if left_cahoot.step_predict() <= right_cahoot.step_predict() {
            (left_flank, left_flank + 1)
        } else {
            (right_flank, right_flank - 1)
        };

        let mut cahoot = crabs
            .population
            .remove(&origin)
            .expect("inconsitent iterator result");

        fuel += cahoot.step();

        match crabs.population.entry(destination) {
            Entry::Vacant(slot) => {
                slot.insert(cahoot);
            }
            Entry::Occupied(mut existing_cahoot) => existing_cahoot.get_mut().merge(cahoot),
        }
    }
}
