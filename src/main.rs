use rayon::prelude::*;
use std::time::Instant;

const BOARD_SIZE: i32 = 50515093;
const S_0: i32 = 290797;

const T_MAX: usize = 100_000; // will later be changed to 10^5
const S_ARR: [i32; (T_MAX * 4) as usize] = compute_s_arr();

const fn compute_s_arr() -> [i32; (T_MAX * 4) as usize] {
    let mut s_arr = [0; (T_MAX * 4) as usize];
    let mut s_curr: i64 = S_0 as i64;
    let mut i = 0;
    while i < (T_MAX * 4) as usize {
        s_arr[i] = s_curr as i32;
        s_curr = (s_curr * s_curr) % (BOARD_SIZE as i64);
        i += 1;
    }

    s_arr
}

#[derive(Debug)]
struct Row {
    // a row containing rows with y coordinate from min_y to max_y (half inclusive)
    pub min_y: i32, // inclusive
    pub max_y: i32, // exclusive
}

fn main() {
    let mut rows: Vec<Row> = Vec::with_capacity(T_MAX * 2 + 1);

    let mut division_points = Vec::with_capacity(T_MAX * 2);

    for t in 1..=T_MAX {
        let mut y_min = S_ARR[4 * t - 2];
        let mut y_max = S_ARR[4 * t - 1];

        if y_max < y_min {
            std::mem::swap(&mut y_min, &mut y_max);
        }

        division_points.push(y_min);
        division_points.push(y_max + 1);
    }
    division_points.sort();

    rows.push(Row {
        min_y: 0,
        max_y: *division_points.get(0).unwrap(),
    });
    for i in 1..division_points.len() {
        rows.push(Row {
            min_y: *division_points.get(i - 1).unwrap(),
            max_y: *division_points.get(i).unwrap(),
        });
    }
    rows.push(Row {
        min_y: *division_points.get(division_points.len() - 1).unwrap(),
        max_y: BOARD_SIZE,
    });

    // dbg!(&rows[0..50]);
    // dbg!(&rows.len());

    let mut rows_list = Vec::new();
    let num_chunks = 50;
    let items_per_chunk = rows.len() / num_chunks;
    for i in 0..num_chunks {
        // clone NUM_ROWS
        let mut curr_row_chunk = Vec::new();
        for _ in 0..items_per_chunk {
            curr_row_chunk.push(rows.pop().unwrap());
        }
        if i == num_chunks - 1 {
            while !rows.is_empty() {
                curr_row_chunk.push(rows.pop().unwrap());
            }
        }

        rows_list.push(curr_row_chunk);
    }

    println!("finished setting up chunks");

    let mut grand_total = 0i64;

    let mut prev_instant = Instant::now();

    for (index, rows) in rows_list.iter().enumerate() {
        grand_total += rows
            .into_par_iter()
            .fold(
                || 0i64,
                |acc, row| {
                    acc + (row.max_y - row.min_y) as i64
                        * calculate_row_clock_hands_sum((row.min_y + row.max_y) / 2)
                },
            )
            .sum::<i64>();
        let now = Instant::now();
        println!(
            "finished chunk {} of {}. Last chunk took {} seconds",
            index,
            num_chunks,
            (now - prev_instant).as_secs()
        );
        prev_instant = now;
    }
    // dbg!(division_points);

    println!("the grand total is {}", grand_total);
}

#[derive(Debug)]
struct Region {
    count: i32,
    start: i32, // inclusive
    end: i32,   // exclusive
    children: Option<(Box<Region>, Box<Region>)>,
}

impl Region {
    pub fn new() -> Self {
        Region {
            count: 0,
            start: 0,
            end: BOARD_SIZE,
            children: None,
        }
    }
    pub fn update(&mut self, min: i32, max: i32) {
        debug_assert!(min >= self.start && max <= self.end);

        if min == self.start && max == self.end {
            // don't propagate to any children, just increase the count here
            self.count += 1;
            return;
        }
        match &mut self.children {
            None => {
                if min != self.start {
                    // let min be the new middle
                    let left_child = Region {
                        count: 0,
                        start: self.start,
                        end: min,
                        children: None,
                    };
                    let mut right_child = Region {
                        count: 0,
                        start: min,
                        end: self.end,
                        children: None,
                    };
                    right_child.update(min, max);
                    self.children = Some((Box::new(left_child), Box::new(right_child)));
                } else {
                    // let max be the new middle
                    let mut left_child = Region {
                        count: 0,
                        start: self.start,
                        end: max,
                        children: None,
                    };
                    let right_child = Region {
                        count: 0,
                        start: max,
                        end: self.end,
                        children: None,
                    };
                    left_child.update(min, max);
                    self.children = Some((Box::new(left_child), Box::new(right_child)));
                }
            }
            Some((left, right)) => {
                debug_assert_eq!(left.end, right.start);
                let middle = left.end;
                if max <= middle {
                    // propagate update to left child
                    left.update(min, max);
                } else if min >= middle {
                    // propagate update to right child
                    right.update(min, max);
                } else {
                    // propagate to both children
                    left.update(min, middle);
                    right.update(middle, max);
                }
            }
        }
    }

    pub fn propagate_counts_to_terminal_nodes(&mut self) {
        match &mut self.children {
            None => {
                // do nothing
            }
            Some((left, right)) => {
                left.increase_count(self.count);
                right.increase_count(self.count);
                self.count = 0;
                left.propagate_counts_to_terminal_nodes();
                right.propagate_counts_to_terminal_nodes();
            }
        }
    }

    pub fn report_sum_of_clock_hands(&self) -> i64 {
        // this function assumes we have already propagated counts to terminal nodes

        match &self.children {
            None => {
                let mut clock_num = self.count % 12;
                if clock_num == 0 {
                    clock_num = 12;
                }
                return clock_num as i64 * (self.end - self.start) as i64;
            }
            Some((left, right)) => {
                return left.report_sum_of_clock_hands() + right.report_sum_of_clock_hands();
            }
        }
    }

    pub fn increase_count(&mut self, change: i32) {
        self.count += change;
    }
}

fn calculate_row_clock_hands_sum(y_pos: i32) -> i64 {
    let mut region = Region::new();

    for t in 1..=T_MAX {
        let mut x_min = S_ARR[4 * t - 4];
        let mut x_max = S_ARR[4 * t - 3];
        let mut y_min = S_ARR[4 * t - 2];
        let mut y_max = S_ARR[4 * t - 1];
        if y_max < y_min {
            std::mem::swap(&mut y_min, &mut y_max);
        }
        if x_max < x_min {
            std::mem::swap(&mut x_min, &mut x_max);
        }

        if !(y_min <= y_pos && y_pos <= y_max) {
            continue;
        }
        region.update(x_min, x_max + 1);
    }

    // dbg!(&region);
    region.propagate_counts_to_terminal_nodes();

    return region.report_sum_of_clock_hands();
}

// fn calculate_row_clock_hands_sum(y_pos: i32) -> i64 {
//     let mut regions = vec![Region {
//         count: 0,
//         start: 0,
//         end: BOARD_SIZE,
//     }];

//     for t in 1..=T_MAX {
//         let mut x_min = S_ARR[4 * t - 4];
//         let mut x_max = S_ARR[4 * t - 3];
//         let mut y_min = S_ARR[4 * t - 2];
//         let mut y_max = S_ARR[4 * t - 1];
//         if y_max < y_min {
//             std::mem::swap(&mut y_min, &mut y_max);
//         }
//         if x_max < x_min {
//             std::mem::swap(&mut x_min, &mut x_max);
//         }

//         if !(y_min <= y_pos && y_pos <= y_max) {
//             continue;
//         }

//         // update all regions between x_min and x_max
//         let mut i = 0;
//         while i < regions.len() {
//             let r = regions.get_mut(i).unwrap();

//             if r.start >= x_min && r.end <= x_max + 1 {
//                 // case where region lies entirely within update region
//                 r.count += 1;
//             } else if r.end > x_max + 1 && r.start < x_min {
//                 // split into 3 regions
//                 let r = regions.swap_remove(i);
//                 regions.push(Region {
//                     count: r.count,
//                     start: r.start,
//                     end: x_min,
//                 });
//                 regions.push(Region {
//                     count: r.count,
//                     start: x_max + 1,
//                     end: r.end,
//                 });
//                 regions.push(Region {
//                     count: r.count + 1,
//                     start: x_min,
//                     end: x_max + 1,
//                 });
//                 let last_index = regions.len() - 1;
//                 regions.swap(i, last_index);
//                 break; // we can break here because no other regions will be affected.
//             } else if r.start <= x_max && r.end > x_max + 1 {
//                 // split into 2 regions
//                 let r = regions.swap_remove(i);
//                 regions.push(Region {
//                     count: r.count,
//                     start: x_max + 1,
//                     end: r.end,
//                 });
//                 regions.push(Region {
//                     count: r.count + 1,
//                     start: r.start,
//                     end: x_max + 1,
//                 });
//                 let last_index = regions.len() - 1;
//                 regions.swap(i, last_index);
//             } else if r.end > x_min && r.start < x_min {
//                 // split into 2 regions
//                 let r = regions.swap_remove(i);
//                 regions.push(Region {
//                     count: r.count,
//                     start: r.start,
//                     end: x_min,
//                 });
//                 regions.push(Region {
//                     count: r.count + 1,
//                     start: x_min,
//                     end: r.end,
//                 });
//                 let last_index = regions.len() - 1;
//                 regions.swap(i, last_index);
//             } else {
//                 // no overlap, do nothing.
//             }
//             i += 1;
//         }
//         // regions.sort_by_key(|region| region.start);
//         // dbg!(&regions);
//     }
//     let mut total: i64 = 0;

//     for region in &regions {
//         let mut clock_num = region.count % 12;
//         if clock_num == 0 {
//             clock_num = 12;
//         }
//         total += ((region.end - region.start) * clock_num) as i64;
//     }

//     // dbg!(&regions);
//     // println!("the sum at {} is {}", y_pos, total);
//     total
// }
