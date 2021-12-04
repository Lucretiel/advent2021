use std::mem;

use num::Num;

#[derive(Debug, Clone)]
enum State<T, const N: usize> {
    Begin,
    Buffered([T; N]),
    Done,
}

impl<T, const N: usize> State<T, N> {
    fn take(&mut self) -> Self {
        mem::replace(self, State::Done)
    }
}

#[derive(Debug)]
pub struct Windows<I: Iterator, const N: usize> {
    iter: I,
    state: State<I::Item, N>,
}

impl<I: Iterator, const N: usize> Windows<I, N> {
    fn compute_size_hint(inner_size: usize) -> usize {
        inner_size.saturating_sub(N - 1)
    }
}

impl<I: Iterator, const N: usize> Iterator for Windows<I, N>
where
    I::Item: Clone,
{
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = match self.state.take() {
            State::Begin => brownstone::try_build_iter(&mut self.iter)?,
            State::Buffered(buffer) => buffer,
            State::Done => return None,
        };

        if let Some(next) = self.iter.next() {
            self.state = State::Buffered(brownstone::build_iter(
                buffer.iter().skip(1).cloned().chain(Some(next)),
            ))
        }

        Some(buffer)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.iter.size_hint();
        (
            Self::compute_size_hint(min),
            max.map(Self::compute_size_hint),
        )
    }
}

impl<I: Iterator + Clone, const N: usize> Clone for Windows<I, N>
where
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            state: self.state.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.iter.clone_from(&source.iter);
        self.state.clone_from(&source.state);
    }
}

#[derive(Debug)]
pub struct UseOksAdapter<'a, I, E> {
    iter: I,
    error: &'a mut Result<(), E>,
}

impl<I: Iterator<Item = Result<T, E>>, T, E> Iterator for UseOksAdapter<'_, I, E> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.error.as_ref().ok()?;
        self.iter
            .next()?
            .map_err(|err| {
                *self.error = Err(err);
            })
            .ok()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match *self.error {
            Err(_) => (0, Some(0)),
            Ok(()) => {
                let (_, max) = self.iter.size_hint();
                (0, max)
            }
        }
    }
}

pub trait IterExt: Iterator + Sized {
    fn streaming_windows<const N: usize>(self) -> Windows<Self, N>
    where
        Self::Item: Clone,
    {
        Windows {
            iter: self,
            state: State::Begin,
        }
    }

    fn use_oks<T, U, E, F>(self, body: F) -> Result<U, E>
    where
        Self: Iterator<Item = Result<T, E>>,
        F: for<'a> FnOnce(UseOksAdapter<'a, Self, E>) -> U,
    {
        let mut err = Ok(());

        let value = body(UseOksAdapter {
            iter: self,
            error: &mut err,
        });

        err.map(|()| value)
    }
}

impl<I: Iterator> IterExt for I {}

#[cfg(test)]
mod iter_ext_tests {
    use super::*;

    #[test]
    fn test_streaming_windows() {
        assert!((0..6).streaming_windows().map(|[a, b, c]| [a, b, c]).eq([
            [0, 1, 2],
            [1, 2, 3],
            [2, 3, 4],
            [3, 4, 5],
        ]))
    }
}

pub trait StrExt {
    fn parse_radix<N: Num>(&self, radix: u32) -> Result<N, N::FromStrRadixErr>;
}

impl StrExt for str {
    fn parse_radix<N: Num>(&self, radix: u32) -> Result<N, N::FromStrRadixErr> {
        N::from_str_radix(self, radix)
    }
}
