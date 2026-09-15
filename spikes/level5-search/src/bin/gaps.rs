//! Scratch: how wide are the vertical gaps Blob crossed, against how wide
//! the map's open vertical edges are; the same for the height of sideways gaps.
#[path = "../main.rs"] #[allow(dead_code)] mod app;
use sidekick::map::free;
use sidekick::starquake::{read_room, at};
use std::collections::BTreeMap;

fn main() {
    let tape = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let exits = std::fs::read_to_string(std::env::args().nth(2).unwrap()).unwrap();
    let base = app::into_play(&tape);
    // Every room's cells as drawn.
    let mut cells: Vec<[[bool; 32]; 18]> = Vec::new();
    for r in 0..512u16 {
        let mut m = base.clone();
        read_room(&mut m, r);
        let mut g = [[false; 32]; 18];
        for row in 0..18 { for col in 0..32 { g[row][col] = free(m.zx.mem[0x5800 + (row + 6) * 32 + col]); } }
        cells.push(g);
    }
    let _ = at::ROOM;
    // Width of the free run in `line` containing `c` and `c+1`.
    let run = |line: &[bool; 32], c: usize| -> usize {
        if c + 1 >= 32 || !line[c] || !line[c + 1] { return 0; }
        let mut lo = c; while lo > 0 && line[lo - 1] { lo -= 1; }
        let mut hi = c + 1; while hi + 1 < 32 && line[hi + 1] { hi += 1; }
        hi - lo + 1
    };
    let col_run = |g: &[[bool; 32]; 18], col: usize, r: usize| -> usize {
        if r + 1 >= 18 || !g[r][col] || !g[r + 1][col] { return 0; }
        let mut lo = r; while lo > 0 && g[lo - 1][col] { lo -= 1; }
        let mut hi = r + 1; while hi + 1 < 18 && g[hi + 1][col] { hi += 1; }
        hi - lo + 1
    };
    // The map's gaps: every maximal free run of >= 2 along each top and bottom edge (as pairs matching both rooms),
    // and every free run of >= 2 down each left and right edge.
    let mut vert_all: BTreeMap<usize, usize> = BTreeMap::new();
    let mut side_all: BTreeMap<usize, usize> = BTreeMap::new();
    for r in 0..512usize {
        let g = &cells[r];
        for (line, other) in [(0usize, r.checked_sub(16).map(|t| (t, 17))), (17, (r + 16 < 512).then(|| (r + 16, 0)))] {
            let Some((t, oline)) = other else { continue };
            let mut c = 0;
            while c < 32 {
                if g[line][c] && cells[t][oline][c] { let s = c; while c < 32 && g[line][c] && cells[t][oline][c] { c += 1; } if c - s >= 2 { *vert_all.entry(c - s).or_default() += 1; } } else { c += 1; }
            }
        }
        for (col, other) in [(0usize, (r % 16 > 0).then(|| (r - 1, 31))), (31, (r % 16 < 15).then(|| (r + 1, 0)))] {
            let Some((t, ocol)) = other else { continue };
            let mut rr = 0;
            while rr < 18 {
                if g[rr][col] && cells[t][rr][ocol] { let s = rr; while rr < 18 && g[rr][col] && cells[t][rr][ocol] { rr += 1; } if rr - s >= 2 { *side_all.entry(rr - s).or_default() += 1; } } else { rr += 1; }
            }
        }
    }
    // The crossings: width of the gap at the arrival column / row (in the room arrived in, at its edge).
    let mut vert_x: BTreeMap<usize, usize> = BTreeMap::new();
    let mut side_x: BTreeMap<usize, usize> = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    for line in exits.lines() {
        let Some(rest) = line.trim().strip_prefix("exit ") else { continue };
        let mut it = rest.split_whitespace();
        let from: usize = it.next().unwrap().parse().unwrap(); it.next();
        let to: usize = it.next().unwrap().parse().unwrap(); it.next();
        let xy = it.next().unwrap().trim_matches(|c| c == '(' || c == ')');
        let (x, y) = xy.split_once(',').unwrap();
        let (x, y): (usize, usize) = (x.parse().unwrap(), y.parse().unwrap());
        let d = to as i32 - from as i32;
        let g = &cells[to];
        let col = x >> 3;
        let row = (0xBF - y) >> 3; // Blob's top-left cell row on screen
        let row = row.saturating_sub(6).min(17);
        match d {
            16 | -16 => { let line = if d == 16 { 0 } else { 17 }; let w = run(&g[line], col); if seen.insert((from, to, col)) { *vert_x.entry(w).or_default() += 1; if w <= 2 { println!("narrow: {from} -> {to} at ({x},{y}) col {col} width {w}"); } } }
            1 | -1 => { let c = if d == 1 { 0 } else { 31 }; let h = col_run(g, c, row); if seen.insert((from, to, row)) { *side_x.entry(h).or_default() += 1; } }
            _ => {}
        }
    }
    println!("vertical gaps on the map, by width in cells: {vert_all:?}");
    println!("vertical crossings by the search, by the gap's width at the arrival column: {vert_x:?}");
    println!("sideways gaps on the map, by height in cells: {side_all:?}");
    println!("sideways crossings by the search, by the gap's height at the arrival row: {side_x:?}");
}
