//! This crate is a simple implementation of minesweeper. It is carefully documented to encourage
//! newbies to add new games to the repository.

#![feature(iter_arith)]

extern crate libterm;

use libterm::{IntoRawMode, TermWrite};

use std::env;
use std::io::{self, Read, Write};
use std::process;

/// A cell in the grid.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
struct Cell {
    /// Does it contain a mine?
    mine: bool,
    /// Is it revealed?
    ///
    /// That is, is it showed or chosen previously by the player?
    revealed: bool,
}

/// The string printed for flagged cells.
const FLAGGED: &'static str = "▓";
/// The string printed for mines in the game over revealing.
const MINE: &'static str = "█";
/// The string printed for concealed cells.
const CONCEALED: &'static str = "▒";

/// The help page.
const HELP: &'static str = r#"
mine ~ a simple minesweeper implementation.

rules:
    Select a cell to reveal, printing the number of adjacent cells holding a mine.
    If no adjacent cells hold a mine, the cell is called free. Free cell will recursively
    reveal their neighboring cells. If a mine is revealed, you loose. The grid wraps.

flags:
    -r | --height N ~ set the height of the grid.
    -c | --width N  ~ set the width of the grid.
    -h | --help     ~ this help page.

controls:
    ---selection-------------------
    space ~ reveal the current cell.
    ---movement---------------------
    h     ~ move left.
    j     ~ move down.
    k     ~ move up.
    l     ~ move right.
    ---flags------------------------
    f     ~ set flag.
    F     ~ remove flag.
    ---control----------------------
    q     ~ quit game.
    r     ~ restart game.

author:
    ticki.
"#;

/// The game state.
struct Game<R, W: Write> {
    /// Width of the grid.
    width: u16,
    /// The grid.
    ///
    /// The cells are enumerated like you would read a book. Left to right, until you reach the
    /// line ending.
    grid: Box<[Cell]>,
    /// The x coordinate.
    x: u16,
    /// The y coordinate.
    y: u16,
    /// The randomizer state.
    ///
    /// This will be modified when a random value is read or written.
    seed: usize,
    /// Standard output.
    stdout: W,
    /// Standard input.
    stdin: R,
}

/// Initialize the game.
fn init<W: Write, R: Read>(mut stdout: W, mut stdin: R, w: u16, h: u16) {
    // Collect entropy pool.
    stdout.write(b"type 10 random characters: ").unwrap();
    stdout.flush().unwrap();
    let mut buf = [0; 10];
    stdin.read_exact(&mut buf).unwrap();

    stdout.clear().unwrap();

    // Set the initial game state.
    let mut game = Game {
        x: 0,
        y: 0,
        seed: 0,
        width: w,
        grid: vec![Cell {
            mine: false,
            revealed: false,
        }; w as usize * h as usize].into_boxed_slice(),
        stdin: stdin,
        stdout: stdout,
    };

    // Write the entropy into the randomizer.
    for &i in buf.iter() {
        game.write_rand(i);
    }

    // Reset that game.
    game.reset();

    // Start the event loop.
    game.start();
}

impl<R, W: Write> Drop for Game<R, W> {
    fn drop(&mut self) {
        // When done, restore the defaults to avoid messing with the terminal.
        self.stdout.restore().unwrap();
    }
}

impl<R: Read, W: Write> Game<R, W> {
    /// Get the grid position of a given coordinate.
    fn pos(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    /// Get the cell at (x, y).
    fn get(&self, x: u16, y: u16) -> Cell {
        self.grid[self.pos(x, y)]
    }

    /// Get a mutable reference to the cell at (x, y).
    fn get_mut(&mut self, x: u16, y: u16) -> &mut Cell {
        &mut self.grid[self.pos(x, y)]
    }

    /// Start the game loop.
    ///
    /// This will listen to events and do the appropriate actions.
    fn start(&mut self) {
        loop {
            // Read a single byte from stdin.
            let mut b = [0];
            self.stdin.read(&mut b).unwrap();

            match b[0] {
                b'h' => self.x = self.left(self.x),
                b'j' => self.y = self.down(self.y),
                b'k' => self.y = self.up(self.y),
                b'l' => self.x = self.right(self.x),
                b' ' => {
                    // Check if it was a mine.
                    if self.get(self.x, self.y).mine {
                        self.game_over();
                        return;
                    }

                    let x = self.x;
                    let y = self.y;

                    // Reveal the cell.
                    self.reveal(x, y);
                },
                b'f' => self.set_flag(),
                b'F' => self.remove_flag(),
                b'r' => self.restart(),
                b'q' => return,
                _ => {},
            }

            // Make sure the cursor is placed on the current position.
            self.stdout.goto(self.x + 1, self.y + 1).unwrap();
            self.stdout.flush().unwrap();
        }
    }

    /// Read a number from the randomizer.
    fn read_rand(&mut self) -> usize {
        self.seed ^= self.seed.rotate_right(4).wrapping_add(0x25A45B35C4FD3DF2);
        self.seed ^= self.seed >> 7;
        self.seed
    }

    /// Write a number into the randomizer.
    ///
    /// This is used for collecting entropy to the randomizer.
    fn write_rand(&mut self, b: u8) {
        self.seed ^= b as usize;
        self.read_rand();
    }

    /// Set a flag on the current cell.
    fn set_flag(&mut self) {
        self.stdout.write(FLAGGED.as_bytes()).unwrap();
    }
    /// Remove a flag on the current cell.
    fn remove_flag(&mut self) {
        self.stdout.write(CONCEALED.as_bytes()).unwrap();
    }

    /// Reset the game.
    ///
    /// This will display the starting grid, and fill the old grid with random mines.
    fn reset(&mut self) {
        // Reset the cursor.
        self.stdout.goto(0, 0).unwrap();

        // Write the upper part of the frame.
        self.stdout.write("┌".as_bytes()).unwrap();
        for _ in 0..self.width {
            self.stdout.write("─".as_bytes()).unwrap();
        }
        self.stdout.write("┐\n\r".as_bytes()).unwrap();

        // Conceal all the cells.
        for _ in 0..self.height() {
            // The left part of the frame
            self.stdout.write("│".as_bytes()).unwrap();

            for _ in 0..self.width {
                self.stdout.write_all(CONCEALED.as_bytes()).unwrap();
            }

            // The right part of the frame.
            self.stdout.write("│".as_bytes()).unwrap();
            self.stdout.write(b"\n\r").unwrap();
        }

        // Write the lower part of the frame.
        self.stdout.write("└".as_bytes()).unwrap();
        for _ in 0..self.width {
            self.stdout.write("─".as_bytes()).unwrap();
        }
        self.stdout.write("┘".as_bytes()).unwrap();

        // Reset the grid.
        for i in 0..self.grid.len() {
            // Fill it with random, concealed fields.
            self.grid[i] = Cell {
                mine: self.read_rand() % 6 == 0,
                revealed: false,
            };
        }
    }

    /// Get the value of a cell.
    ///
    /// The value represent the sum of adjacent cells containing mines. A cell of value, 0, is
    /// called "free".
    fn val(&self, x: u16, y: u16) -> u8 {
        self.adjacent(x, y).iter().map(|&(x, y)| self.get(x, y).mine as u8).sum()
    }

    /// Reveal the cell, _c_.
    ///
    /// This will recursively reveal free cells, until non-free cell is reached, terminating the
    /// current recursion descendant.
    fn reveal(&mut self, x: u16, y: u16) {
        let v = self.val(x, y);

        self.get_mut(x, y).revealed = true;

        self.stdout.goto(x + 1, y + 1).unwrap();

        if v == 0 {
            // If the cell is free, simply put a space on the position.
            self.stdout.write(b" ").unwrap();

            // Recursively reveal adjacent cells until a non-free cel is reached.
            for &(x, y) in self.adjacent(x, y).iter() {
                if !self.get(x, y).revealed && !self.get(x, y).mine {
                    self.reveal(x, y);
                }
            }
        } else {
            // Aww. The cell was not free. Print the value instead.
            self.stdout.write(&[b'0' + v]).unwrap();
        }
    }

    /// Reveal all the fields, printing where the mines were.
    fn reveal_all(&mut self) {
        self.stdout.goto(0, 0).unwrap();

        for y in 0..self.height() {
            for x in 0..self.width {
                self.stdout.goto(x + 1, y + 1).unwrap();
                if self.get(x, y).mine {
                    self.stdout.write(MINE.as_bytes()).unwrap();
                }
            }
        }
    }

    /// Game over!
    fn game_over(&mut self) {
        // Reveal all cells, showing the player where the mines were.
        self.reveal_all();

        self.stdout.goto(0, 0).unwrap();

        // Hide the cursor.
        self.stdout.hide_cursor().unwrap();

        self.stdout.write("╔═════════════════╗\n\r\
                           ║───┬Game over────║\n\r\
                           ║ r ┆ replay      ║\n\r\
                           ║ q ┆ quit        ║\n\r\
                           ╚═══╧═════════════╝\
                          ".as_bytes()).unwrap();
        self.stdout.flush().unwrap();

        loop {
            // Repeatedly read a single byte.
            let mut buf = [0];
            self.stdin.read(&mut buf).unwrap();

            match buf[0] {
                b'r' => {
                    // Replay!
                    self.stdout.show_cursor().unwrap();
                    self.restart();
                    return;
                },
                b'q' => return,
                _ => {},
            }
        }
    }

    /// Restart (replay) the game.
    fn restart(&mut self) {
        self.reset();
        self.start();
    }

    /// Calculate the adjacent cells.
    fn adjacent(&self, x: u16, y: u16) -> [(u16, u16); 8] {
        let left = self.left(x);
        let right = self.right(x);
        let up = self.up(y);
        let down = self.down(y);

        [
            // Left-up
            (left, up),
            // Up
            (x, up),
            // Right-up
            (right, up),
            // Left
            (left, y),
            // Right
            (right, y),
            // Left-down
            (left, down),
            // Down
            (x, down),
            // Right-down
            (right, down)
        ]
    }

    /// Calculate the height (number of rows) of the grid.
    fn height(&self) -> u16 {
        (self.grid.len() / self.width as usize) as u16
    }

    /// Calculate the y coordinate of the cell "above" a given y coordinate.
    ///
    /// This wraps when _y = 0_.
    fn up(&self, y: u16) -> u16 {
        if y == 0 {
            // Upper bound reached. Wrap around.
            self.height() - 1
        } else {
            y - 1
        }
    }
    /// Calculate the y coordinate of the cell "below" a given y coordinate.
    ///
    /// This wraps when _y = h - 1_.
    fn down(&self, y: u16) -> u16 {
        if y + 1 == self.height() {
            // Lower bound reached. Wrap around.
            0
        } else {
            y + 1
        }
    }
    /// Calculate the x coordinate of the cell "left to" a given x coordinate.
    ///
    /// This wraps when _x = 0_.
    fn left(&self, x: u16) -> u16 {
        if x == 0 {
            // Lower bound reached. Wrap around.
            self.width - 1
        } else {
            x - 1
        }
    }
    /// Calculate the x coordinate of the cell "left to" a given x coordinate.
    ///
    /// This wraps when _x = w - 1_.
    fn right(&self, x: u16) -> u16 {
        if x + 1 == self.width {
            // Upper bound reached. Wrap around.
            0
        } else {
            x + 1
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let mut width = None;
    let mut height = None;

    // Get and lock the stdios.
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();
    let stdin = stdin.lock();
    let stderr = io::stderr();
    let mut stderr = stderr.lock();

    loop {
        // Read the arguments.

        let arg = if let Some(x) = args.next() {
            x
        } else {
            break;
        };

        match arg.as_str() {
            "-r" | "--height" => if height.is_none() {
                height = Some(args.next().unwrap_or_else(|| {
                    stderr.write(b"no height given.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }).parse().unwrap_or_else(|_| {
                    stderr.write(b"invalid integer given.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }));
            } else {
                stderr.write(b"you may only input one height.\n").unwrap();
                stderr.flush().unwrap();
                process::exit(1);
            },
            "-c" | "--width" => if width.is_none() {
                width = Some(args.next().unwrap_or_else(|| {
                    stderr.write(b"no width given.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }).parse().unwrap_or_else(|_| {
                    stderr.write(b"invalid integer given.\n").unwrap();
                    stderr.flush().unwrap();
                    process::exit(1);
                }));
            } else {
                stderr.write(b"you may only input one width.\n").unwrap();
                stderr.flush().unwrap();
                process::exit(1);
            },
            "-h" | "--help" => {
                // Print the help page.
                stdout.write(HELP.as_bytes()).unwrap();
                stdout.flush().unwrap();
                process::exit(0);
            },
            _ => {
                stderr.write(b"Unknown argument.\n").unwrap();
                stderr.flush().unwrap();
                process::exit(1);
            }
        }
    }

    // We go to raw mode to make the control over the terminal more fine-grained.
    let stdout = stdout.into_raw_mode().unwrap();

    // Initialize the game!
    init(stdout, stdin, width.unwrap_or(70), height.unwrap_or(40));
}