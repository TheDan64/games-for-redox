use std::cmp::PartialEq;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Index;
use std::slice::Chunks;

#[derive(Debug, Copy, Clone, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub enum CellValue {
    One   = 1,
    Two   = 2,
    Three = 3,
    Four  = 4,
    Five  = 5,
    Six   = 6,
    Seven = 7,
    Eight = 8,
    Nine  = 9,
}

impl CellValue {
    pub fn from_usize(val: usize) -> CellValue {
        match val {
            1 => CellValue::One,
            2 => CellValue::Two,
            3 => CellValue::Three,
            4 => CellValue::Four,
            5 => CellValue::Five,
            6 => CellValue::Six,
            7 => CellValue::Seven,
            8 => CellValue::Eight,
            9 => CellValue::Nine,
            _ => panic!("Value {} is not a valid CellValue", val)
        }
    }
}

pub struct Redoku {
    cells: [Option<CellValue>; 81],
    row_values: BTreeMap<usize, BTreeSet<CellValue>>,
    column_values: BTreeMap<usize, BTreeSet<CellValue>>,
    block_values: BTreeMap<(usize, usize), BTreeSet<CellValue>>,
}

impl Redoku {
    pub fn new() -> Redoku {
        let mut row_values = BTreeMap::new();
        let mut column_values = BTreeMap::new();
        let mut block_values = BTreeMap::new();

        for i in 0..9 {
            row_values.insert(i, BTreeSet::new());
            column_values.insert(i, BTreeSet::new());
            block_values.insert((i % 3, i / 3), BTreeSet::new());
        }

        Redoku {
            cells: [None; 81],
            row_values: row_values,
            column_values: column_values,
            block_values: block_values,
        }
    }

    pub fn place_if_valid(&mut self, x: usize, y: usize, value: Option<CellValue>) -> bool {
        let original_value = self[(x, y)];

        let mut column_values = self.column_values.get_mut(&x).unwrap();
        let mut row_values = self.row_values.get_mut(&y).unwrap();
        let mut block_values = self.block_values.get_mut(&(x / 3, y / 3)).unwrap();

        match value {
            Some(val) => {
                if column_values.contains(&val) || row_values.contains(&val) || block_values.contains(&val) {
                    return false;
                }

                column_values.insert(val);
                row_values.insert(val);
                block_values.insert(val);

                self.cells[9 * y + x] = Some(val);

                true
            },
            None => {
                if let Some(val) = original_value {
                    column_values.remove(&val);
                    row_values.remove(&val);
                    block_values.remove(&val);

                    self.cells[9 * y + x] = None;
                }

                true
            }
        }
    }

    pub fn empty_cells(&self) -> usize {
        let mut cells = 81;

        for i in 0..9 {
            cells -= self.row_values.get(&i).unwrap().len();
        }

        cells
    }

    pub fn row_values(&self, row: &usize) -> &BTreeSet<CellValue> {
        if *row > 8 {
            panic!("No such row {} to get values for.", row);
        }

        self.row_values.get(row).unwrap()
    }

    pub fn column_values(&self, column: &usize) -> &BTreeSet<CellValue> {
        if *column > 8 {
            panic!("No such column {} to get values for.", column);
        }

        self.column_values.get(column).unwrap()
    }
}

// Clone and PartialEq need to be manually implemented because
// [T; n] has issues for n > 32
impl Clone for Redoku {
    fn clone(&self) -> Redoku {
        Redoku {
            cells: self.cells,
            row_values: self.row_values.clone(),
            column_values: self.column_values.clone(),
            block_values: self.block_values.clone(),
        }
    }
}

impl PartialEq for Redoku {
    fn eq(&self, other: &Redoku) -> bool {
        for x in 0..9 {
            for y in 0..9 {
                if self.cells[9 * y + x] != other.cells[9 * y + x] {
                    return false;
                }
            }
        }

        self.row_values == other.row_values &&
        self.column_values == other.column_values &&
        self.block_values == other.block_values
    }

    fn ne(&self, other: &Redoku) -> bool {
        !self.eq(other)
    }
}

impl Index<(usize, usize)> for Redoku {
    type Output = Option<CellValue>;

    fn index(&self, index: (usize, usize)) -> &Option<CellValue> {
        let (x, y) = index;

        &self.cells[y * 9 + x]
    }
}

impl fmt::Debug for Redoku {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::char;

        let mut string = String::new();

        string.push_str("/‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\\");

        for y in 0..9 {
            string.push_str("\n");

            for x in 0..9 {
                string.push_str("|");

                if x == 3 || x == 6 {
                    string.push_str(" |");
                }

                string.push_str(&format!("{}", match self[(x, y)] {
                    Some(val) => char::from_digit(val as u32, 10).unwrap(),
                    None => '?',
                }));
            }

            string.push_str("|");

            if y == 2 || y == 5 {
                string.push_str("\n|                     |");
            }
        }

        string.push_str("\n\\_____________________/");

        write!(f, "{}", string)
    }
}

#[test]
fn test_indexing() {
    let mut redoku = Redoku::new();

    for x in 0..9 {
        for y in 0..9 {
            assert!(redoku[(x, y)] == None);

            redoku.cells[9 * y + x] = Some(CellValue::from_usize(y + 1));

            assert!(redoku[(x, y)] == Some(CellValue::from_usize(y + 1)));

        }
    }
}

#[test]
fn test_place_if_valid() {
    use self::CellValue::*;

    let mut redoku = Redoku::new();

    // Test column
    assert!(redoku.place_if_valid(1, 1, Some(One)));

    assert!(!redoku.place_if_valid(8, 1, Some(One)));

    // Test row
    assert!(!redoku.place_if_valid(1, 8, Some(One)));

    // Test block
    assert!(redoku.place_if_valid(0, 7, Some(One)));

    assert!(!redoku.place_if_valid(2, 8, Some(One)));

    assert!(redoku.place_if_valid(1, 1, None));
}
