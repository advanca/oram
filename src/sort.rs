// Copyright 2020 ADVANCA PTE. LTD.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(feature = "sgx")]
use sgx_tstd::{self as std, prelude::v1::*};

use std::marker::PhantomData;
use std::ops::Range;

/// Use Batcher's odd-even mergesort algorithm to sort an external array (or array-like structure)
///
/// # Arguments
///
/// `range` specifies a 0-indexed range starting from 0 in the external array.
/// `cmp` is the compare function used to sort the element in ascending order.
/// `access` is the access function to read element from or write to the external array.
///
/// # Examples
///
/// ```
/// let mut v = vec![4, 3, 2, 1, 0];
/// let sorted = vec![0, 1, 2, 3, 4];
/// oram::sort::odd_even_mergesort(0..v.len(), |x: &i32, y: &i32| x<y, |i: usize, w: Option<&i32>| match w {
///     Some(x) => { v[i] = *x; None } // It's a write
///     None => Some(v[i])  // It's a read
/// });
/// assert_eq!(v, sorted);
/// ```
pub fn odd_even_mergesort<T, C, A>(range: Range<usize>, cmp: C, access: A)
where
    C: Fn(&T, &T) -> bool,
    A: FnMut(usize, Option<&T>) -> Option<T>,
{
    BatcherSort::new(range, cmp, access).sort();
}

struct BatcherSort<T, C, A>
where
    C: Fn(&T, &T) -> bool,
    A: FnMut(usize, Option<&T>) -> Option<T>,
{
    range: Range<usize>,
    cmp: C,
    access: A,
    phantom: PhantomData<T>,
}

impl<T, C, A> BatcherSort<T, C, A>
where
    C: Fn(&T, &T) -> bool,
    A: FnMut(usize, Option<&T>) -> Option<T>,
{
    fn new(range: Range<usize>, cmp: C, access: A) -> Self {
        BatcherSort {
            range,
            cmp,
            access,
            phantom: PhantomData,
        }
    }

    fn sort(&mut self) {
        assert_eq!(self.range.start, 0, "range must start from 0");
        let high = self.range.end.next_power_of_two();

        self.odd_even_merge_sort(self.range.start, high);
    }

    fn odd_even_merge_sort(&mut self, low: usize, high: usize) {
        if high - low > 1 {
            let m = (high - low) >> 1;
            self.odd_even_merge_sort(low, low + m);
            self.odd_even_merge_sort(low + m, high);
            self.odd_even_merge(low, high, 1);
        }
    }

    fn odd_even_merge(&mut self, low: usize, high: usize, d: usize) {
        if (high - low) > 2 * d {
            self.odd_even_merge(low, high, 2 * d);
            self.odd_even_merge(low + d, high, 2 * d);
            for i in (low + d..high - d).step_by(2 * d) {
                if i + d < self.range.end {
                    self.compare_and_swap(i, i + d);
                }
            }
        } else if low + d < self.range.end {
            self.compare_and_swap(low, low + d);
        }
    }

    fn compare_and_swap(&mut self, a: usize, b: usize) {
        let a_obj = (self.access)(a, None).expect("read operation");
        let b_obj = (self.access)(b, None).expect("read operation");

        if !(self.cmp)(&a_obj, &b_obj) {
            (self.access)(a, Some(&b_obj));
            (self.access)(b, Some(&a_obj));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sort_until_n(n: usize, source: &Vec<i32>) {
        let mut v = source.clone();
        let mut v2 = source.clone();

        let mut bat = BatcherSort::new(
            0..n,
            |x: &i32, y: &i32| x < y,
            |i: usize, w: Option<&i32>| match w {
                Some(x) => {
                    v[i] = *x;
                    None
                }
                None => Some(v[i]),
            },
        );

        bat.sort();
        v2[0..n].sort();
        assert_eq!(v[0..n], v2[0..n]);
    }

    #[test]
    fn sort_should_work() {
        let v = vec![8, 1, 3, 4, 6, 7, 1, 2, 3];

        for i in 1..v.len() {
            sort_until_n(i, &v);
        }
    }

    #[test]
    fn sort_struct() {
        let mut v = vec![(0, 1), (1, 3), (4, 1), (4, 2), (3, 9)];

        let mut bat = BatcherSort::new(
            0..v.len(),
            |x: &(i32, i32), y| (x.0 < y.0) || (x.0 == y.0 && x.1 < y.1),
            |i, w| match w {
                Some(x) => {
                    v[i] = *x;
                    None
                }
                None => Some(v[i]),
            },
        );

        bat.sort();
        assert_eq!(v, vec![(0, 1), (1, 3), (3, 9), (4, 1), (4, 2)])
    }
}
