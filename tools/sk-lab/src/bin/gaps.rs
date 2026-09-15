//! How wide the gaps are that the search crossed, against how wide the
//! map's openings are: vertical gaps by width in cells, sideways gaps by
//! height, from `exits.txt`. Crossings through a gap two cells wide are
//! listed.
//!
//! `gaps <assets-dir>`

use std::collections::{BTreeMap, HashSet};

use sidekick::map::free;
use sk_lab::exits;
use sk_lab::rooms::{Cells, Planet};
use sk_lab::{Args, into_play, top_row};

/// The free run along `line` holding cells `c` and `c + 1`, in cells.
fn run(line: &[u8; 32], c: usize) -> usize {
    if c + 1 >= 32 || !free(line[c]) || !free(line[c + 1]) {
        return 0;
    }
    let mut lo = c;
    while lo > 0 && free(line[lo - 1]) {
        lo -= 1;
    }
    let mut hi = c + 1;
    while hi + 1 < 32 && free(line[hi + 1]) {
        hi += 1;
    }
    hi - lo + 1
}

/// The free run down column `col` holding rows `r` and `r + 1`.
fn column_run(cells: &Cells, col: usize, r: usize) -> usize {
    if r + 1 >= 18 || !free(cells[r][col]) || !free(cells[r + 1][col]) {
        return 0;
    }
    let mut lo = r;
    while lo > 0 && free(cells[lo - 1][col]) {
        lo -= 1;
    }
    let mut hi = r + 1;
    while hi + 1 < 18 && free(cells[hi + 1][col]) {
        hi += 1;
    }
    hi - lo + 1
}

fn main() {
    let args = Args::parse("gaps <assets-dir>");
    let text = std::fs::read_to_string(args.path(exits::FILE)).unwrap_or_else(|e| {
        eprintln!(
            "cannot read {}: {e} (run whole first)",
            args.path(exits::FILE).display()
        );
        std::process::exit(2);
    });
    let dump = exits::read(&text);
    let base = into_play(&args.tape());
    let planet = Planet::read(&base);
    let cells = &planet.cells;
    // The map's gaps: every maximal free run of two or more along a shared
    // edge, free in both rooms.
    let mut vertical: BTreeMap<usize, usize> = BTreeMap::new();
    let mut sideways: BTreeMap<usize, usize> = BTreeMap::new();
    for r in 0..512usize {
        let g = &cells[r];
        if r + 16 < 512 {
            let t = &cells[r + 16];
            let mut c = 0;
            while c < 32 {
                let s = c;
                while c < 32 && free(g[17][c]) && free(t[0][c]) {
                    c += 1;
                }
                if c - s >= 2 {
                    *vertical.entry(c - s).or_default() += 1;
                }
                c += 1;
            }
        }
        if r % 16 < 15 {
            let t = &cells[r + 1];
            let mut rr = 0;
            while rr < 18 {
                let s = rr;
                while rr < 18 && free(g[rr][31]) && free(t[rr][0]) {
                    rr += 1;
                }
                if rr - s >= 2 {
                    *sideways.entry(rr - s).or_default() += 1;
                }
                rr += 1;
            }
        }
    }
    // The crossings: the gap's width at the arrival column, or its height at
    // the arrival row, in the room arrived in.
    let mut vertical_crossed: BTreeMap<usize, usize> = BTreeMap::new();
    let mut sideways_crossed: BTreeMap<usize, usize> = BTreeMap::new();
    let mut seen = HashSet::new();
    let mut narrow = Vec::new();
    for (&from, record) in &dump {
        for &(to, x, y) in &record.exits {
            let d = i32::from(to) - i32::from(from);
            let g = &cells[usize::from(to)];
            let col = usize::from(x >> 3);
            let row = usize::from(top_row(y)).saturating_sub(6).min(17);
            match d {
                16 | -16 => {
                    let line = if d == 16 { 0 } else { 17 };
                    let w = run(&g[line], col);
                    if seen.insert((from, to, col)) {
                        *vertical_crossed.entry(w).or_default() += 1;
                        if w <= 2 {
                            narrow.push(format!(
                                "{from} -> {to} at ({x},{y}), column {col}, width {w}"
                            ));
                        }
                    }
                }
                1 | -1 => {
                    let c = if d == 1 { 0 } else { 31 };
                    let h = column_run(g, c, row);
                    if seen.insert((from, to, row)) {
                        *sideways_crossed.entry(h).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
    }
    println!("vertical gaps on the map, by width in cells: {vertical:?}");
    println!(
        "vertical crossings by the search, by the gap's width where Blob arrived: {vertical_crossed:?}"
    );
    println!("sideways gaps on the map, by height in cells: {sideways:?}");
    println!(
        "sideways crossings by the search, by the gap's height where Blob arrived: {sideways_crossed:?}"
    );
    println!("crossings through a vertical gap two cells wide or less:");
    for line in &narrow {
        println!("  {line}");
    }
}
