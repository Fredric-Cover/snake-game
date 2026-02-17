//! This is a snake game designed by Freddie, and written in Rust (obviously).
//! It is designed to run in a terminal window, and be fun to play
//! as long as you aren't constantly resizing the window. Enjoy!

use bincode::config;
use crossterm::{
    cursor, // For MoveTo, Hide, Show
    event::{self, KeyCode},
    execute, // The macro used to dispatch commands to the terminal
    style::{self, Color, Print, Stylize}, // For colored drawing
    terminal::{self, ClearType}, // For enable/disable raw mode and Clear command
};
use rand::Rng;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::panic;
use std::time::Duration;

// --- DATA STRUCTURES ---

pub struct TickRate {
    pub vertical: u8,
    pub horizontal: u8,
    decrease_horizontal: bool, // a value to ensure that horizontal is only decreased every other call to decrease()
}

impl TickRate {
    pub fn new(vertical_rate: u8, horizontal_rate: u8) -> Self {
        return TickRate {
            vertical: vertical_rate,
            horizontal: horizontal_rate,
            decrease_horizontal: false,
        };
    }

    fn get_correct(&self, direction: &Direction) -> u8 {
        if *direction == Direction::Up || *direction == Direction::Down {
            return self.vertical;
        } else {
            return self.horizontal;
        }
    }

    fn decrease(&mut self) {
        if self.decrease_horizontal {
            self.horizontal = self.horizontal.saturating_sub(1).max(20);
            self.decrease_horizontal = false; // don't decrease vertical next call
        } else {
            self.decrease_horizontal = true; // decrease vertical next call
        }
        self.vertical = self.vertical.saturating_sub(1).max(10);
    }
}

/// Represents a 2D coordinate for the snake and food
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn to_screen(&self, h_offset: u16, v_offset: u16) -> (u16, u16) {
        let draw_x = self.x.wrapping_add(h_offset + 1);
        let draw_y = self.y.wrapping_add(v_offset + 1);

        (draw_x, draw_y)
    }
}

/// Represents which type and where the snake food is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Food {
    pub pos: Position,
    pub is_special: bool,
}

/// Represents the direction of movement
#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// The Snake itself, represented by a list of positions and its current direction.
pub struct Snake {
    pub food_array: Vec<Food>,
    pub body: Vec<Position>,
    pub direction: Direction,
    pub is_rainbow: bool,
}

impl Snake {
    /// Creates a new snake of length 3 in the center of the board, moving right.
    pub fn new(width: u16, height: u16) -> Self {
        let start_x = width / 2;
        let start_y = height / 2;

        // Start with a 3-segment snake moving right (head is at body[0])
        let body = vec![
            Position {
                x: start_x,
                y: start_y,
            },
            Position {
                x: start_x - 1,
                y: start_y,
            },
            Position {
                x: start_x - 2,
                y: start_y,
            },
        ];

        let mut rng = rand::rng();
        let food_array = vec![
            Food {
                pos: Position {
                    x: rng.random_range(1..=(width - 3)),  // -2 for left/right walls
                    y: rng.random_range(1..=(height - 2)), // -1 for top/bottom walls (since 0-indexed)
                },
                is_special: rng.random_range(0..10) == 0,
            },
            Food {
                pos: Position {
                    x: rng.random_range(1..=(width - 3)),  // -2 for left/right walls
                    y: rng.random_range(1..=(height - 2)), // -1 for top/bottom walls (since 0-indexed)
                },
                is_special: rng.random_range(0..10) == 0,
            },
            Food {
                pos: Position {
                    x: rng.random_range(1..=(width - 3)),  // -2 for left/right walls
                    y: rng.random_range(1..=(height - 2)), // -1 for top/bottom walls (since 0-indexed)
                },
                is_special: rng.random_range(0..10) == 0,
            },
        ];

        Snake {
            food_array,
            body,
            direction: Direction::Right,
            is_rainbow: false,
        }
    }

    /// Calculates the next position of the snake's head based on its current direction.
    pub fn get_next_head_pos(&self) -> Position {
        let head = self.body[0];
        let mut next_pos = head;

        // Note: For terminal drawing, Y increases DOWNWARDS
        match self.direction {
            // Note 2: using wrapping_sub so that you can go out of bounds for Easter Egg #2
            Direction::Up => next_pos.y = next_pos.y.wrapping_sub(1),
            Direction::Down => next_pos.y = next_pos.y.wrapping_add(1),
            Direction::Left => next_pos.x = next_pos.x.wrapping_sub(1),
            Direction::Right => next_pos.x = next_pos.x.wrapping_add(1),
        }
        next_pos
    }

    /// Moves the snake one step in its current direction.
    /// Returns the tail position that was removed (which we need later for drawing).
    pub fn update(&mut self, remove_very_end: bool) -> Position {
        let (cols, rows) = terminal::size().unwrap();
        let game_width = cols.min(100); // 100 is MAX_GAME_WIDTH
        const TOP_BORDER_Y: u16 = 3;
        let horizontal_offset = (cols.wrapping_sub(game_width)) / 2; // Why all these variables/constants? They are needed for conversion between absolute and game coordinates, a big pain.

        let mut new_head_pos = self.get_next_head_pos();

        let mut draw_x = new_head_pos.x.wrapping_add(horizontal_offset + 1); // used for determining screen wrapping, need to know real pos on screen
        let mut draw_y = new_head_pos.y.wrapping_add(TOP_BORDER_Y + 1); // same as x

        if draw_x > cols {
            if self.direction == Direction::Right {
                // hit right terminal border
                draw_x = 0;
            } else {
                // hit left terminal border
                draw_x = cols;
            }
            new_head_pos.x = draw_x.wrapping_sub(horizontal_offset + 1);
        }

        if draw_y > rows {
            if self.direction == Direction::Down {
                // hit bottom
                draw_y = 0;
            } else {
                // hit top
                draw_y = rows;
            }

            new_head_pos.y = draw_y.wrapping_sub(TOP_BORDER_Y + 1);
        }

        // Add the new head position
        self.body.insert(0, new_head_pos);

        // Remove the tail to simulate movement if removing the back (pop_back returns the removed element)
        if remove_very_end {
            return self.body.pop().unwrap();
        }

        return Position { x: 0, y: 0 }; // remove nothing
    }

    /// Checks for self-collision.

    pub fn check_for_self_collision(&self) -> bool {
        let next_head_position = self.get_next_head_pos();
        if self.body.contains(&next_head_position) {
            return true; // attempted to eat self
        }

        false
    }

    /// Checks for wall collision.
    pub fn check_for_wall_collision(
        &self,
        game_width: u16,
        game_height: u16,
        game_score: u32,
    ) -> bool {
        if self.is_rainbow && game_score == 32 {
            return false;
        }
        let next_head_position = self.get_next_head_pos();
        if next_head_position.x == game_width - 2 || next_head_position.y == game_height {
            return true;
        }

        if next_head_position.x == u16::MAX || next_head_position.y == u16::MAX {
            // head needs to stick into the boundry itself to kill snake, otherwise no way to move alongside the wall
            return true;
        }

        false
    }

    /// Attempts to change the snake's direction, preventing immediate reversal (e.g., Up to Down).
    pub fn change_direction(&mut self, new_dir: Direction) {
        let is_opposite = match (self.direction, new_dir) {
            (Direction::Up, Direction::Down) => true,
            (Direction::Down, Direction::Up) => true,
            (Direction::Left, Direction::Right) => true,
            (Direction::Right, Direction::Left) => true,
            _ => false,
        };

        // Only change direction if it's not the exact opposite of the current direction
        if !is_opposite {
            self.direction = new_dir;
        }
    }
}

/// Holds the main game state and configuration
pub struct Game {
    pub width: u16,
    pub height: u16,
    pub snake: Snake,
    pub score: u32,
    pub high_score: u32,
    pub is_game_over: bool,
    pub is_won: bool,
    pub star_is_double: bool
}

impl Game {
    pub fn new(width: u16, height: u16, high_score: u32, star_double: bool) -> Self {
        Game {
            width,
            height,
            snake: Snake::new(width, height), // Initialize the snake!
            score: 0,
            high_score,
            is_game_over: false,
            is_won: false,
            star_is_double: star_double
        }
    }

    /// Checks for food collision, and handles eating
    pub fn check_for_food_collision(&mut self) -> bool {
        let next_head_position = self.snake.get_next_head_pos();
        let star_is_double = self.star_is_double;

        // 1. Find the index of the food we hit (Immutable borrow)
        let food_index = self.snake.food_array.iter().position(|food| {
            (next_head_position == food.pos) ||
            check_for_special_food_collision(food, &next_head_position, star_is_double)
        });

        // 2. If we found a collision, handle it
        if let Some(idx) = food_index {
            let is_special = self.snake.food_array[idx].is_special;

            // calculate new positon
            let new_food_item = valid_pos(
                &self.snake.body,
                self.width,
                self.height,
                &self.snake.food_array
            );

            // Update the specific food item
            self.snake.food_array[idx] = new_food_item;

            // Update score
            self.score += if is_special { 3 } else { 1 };

            return false;
        }

        true
    }

    /// Prints a message to the screen, and waits for a single character input.
    pub fn print_message(&self, what: &str, offset_from_wall: u16) -> io::Result<KeyCode> {
        let mut stdout = io::stdout();
        // 1. Determine the maximum number of printable characters per line.
        let line_capacity: usize = (self.width as usize)
            .saturating_sub(2)
            .saturating_sub(2 * offset_from_wall as usize);
        if what.len() > line_capacity {
            // Handle wrapping for messages that overflow
            // Ensure line_capacity is not zero to prevent division by zero
            let line_capacity = line_capacity.max(1);
            // Calculate total lines needed. Add line_capacity - 1 before division
            // to correctly round up: (total + divisor - 1) / divisor
            let total_lines = (what.len() + line_capacity - 1) / line_capacity;
            let mut start_index = 0;
            // Loop from line 5 up to (total_lines + 5 - 1)
            for line_number in 5..(5 + total_lines as u16) {
                // 2. Calculate the end index for the current slice
                let end_index = (start_index + line_capacity).min(what.len());
                // 3. Get the substring
                let this_ln_what = &what[start_index..end_index];
                // 4. Print the line
                execute!(stdout, cursor::MoveTo(offset_from_wall + 1, line_number))?;
                let _ = execute!(stdout, Print(this_ln_what))?;
                // 5. Update the starting index for the next line
                start_index = end_index;
                // Break if we've reached the end of the string
                if start_index >= what.len() {
                    break;
                }
            }
        } else {
            let (cols, _rows) = terminal::size().unwrap();
            let game_width = cols.min(100); // MAX_GAME_WIDTH = 100
            let horizontal_offset = (cols.wrapping_sub(game_width)) / 2;
            execute!(
                stdout,
                cursor::MoveTo(offset_from_wall + 1 + horizontal_offset, 5)
            )?; // move cursor to offset_from_wall distance from screen boundry
            let _ = execute!(stdout, Print(what))?;
        }
        loop {
            match event::read()? {
                event::Event::Key(key_event) => {
                    return Ok(key_event.code);
                }
                _ => {} // Ignore other events (like mouse or resize)
            }
        }
    }
}

/// Checks for collision with special food with corrections for size
pub fn check_for_special_food_collision(food: &Food, next_head_pos: &Position, star_is_double: bool) -> bool {
    // assume that caller already checked for normal collision, skip that
    // if the food is not special or only one char wide, nothing to check
    if !food.is_special || !star_is_double {
        return false;
    }

    // check for the snake hitting one char to the right of the snake head
    next_head_pos == &Position {
        x: food.pos.x + 1,  // x one to the right
        y: food.pos.y, // same y
    }
}

/// Helper function to get the actual width of a character in the terminal
fn get_actual_width(stdout: &mut io::Stdout, c: &'static str) -> u16 {
    // Get starting position
    let (start_x, _) = cursor::position().unwrap_or((0, 0));

    // Print the character
    print!("{}", c);
    let _ = stdout.flush();

    // Get ending position
    let (end_x, _) = cursor::position().unwrap_or((start_x + 1, 0));

    // Calculate delta
    if end_x > start_x {
        end_x - start_x
    } else {
        2 // Fallback for weird wrapping or errors
    }
}

// -- Helper functions for saving high score --

/// Saves the game's high score
fn save(data: u32) {
    let config = config::standard();
    let encoded: Vec<u8> = bincode::encode_to_vec(data, config).expect("Failed to serialize data");
    let mut file = File::create("data.dat").expect("Failed to overwrite / create data.dat");
    file.write_all(&encoded)
        .expect("Failed to write to data.dat");
}

/// Loads the game's high score
fn load() -> u32 {
    let buffer: Vec<u8> = fs::read("data.dat").unwrap_or(vec![0, 0, 0, 0]);
    let config = config::standard();
    let (decoded, _len): (u32, usize) =
        bincode::decode_from_slice(&buffer[..], config).unwrap_or((0, 0));
    return decoded;
}

/// Calculates a valid positon for food to appear
fn valid_pos(body: &Vec<Position>, width: u16, height: u16, food_array: &Vec<Food>) -> Food {
    let mut rng = rand::rng();
    loop {
        let pos = Position {
            x: rng.random_range(1..=(width.wrapping_sub(3))), // -2 for left/right walls
            y: rng.random_range(1..=(height.wrapping_sub(1))), // -1 for top/bottom walls (since 0-indexed)
        };
        let is_special = rng.random_range(0..10) == 0; // one in ten chance of being special food displayed with ✴️

        if body.contains(&pos) {
            // check if the snake's body contains the position generated
            continue;
        }

        for food in food_array.iter() {
            if food.pos == pos {
                continue;
            }

            if food.is_special || is_special && (food.pos.y == pos.y) {
                // if the food is special, it takes up two spaces. This block checks if it overlaps.
                if food.pos.x + 1 == pos.x {
                    continue;
                }

                if pos.x + 1 == food.pos.x {
                    continue;
                }
            }

            if pos.x == width.wrapping_sub(3) && is_special {
                // if the food is special and it is right up next to the wall, it will "share a space" with the wall.
                continue;
            }
        }

        /*if (snake.food.is_special == true) && (snake.body.contains(&Position{x: &pos.x + 1, y: pos.y})){  // if the food is special, the food takes TWO chars, and this checks if the body contains the OTHER char.
            continue;
        }*/
        return Food {
            pos: pos,
            is_special: is_special,
        };
    }
}

/// Redraw the entire playing field.
fn draw_playfield(stdout: &mut io::Stdout, game: &Game, h_offset: u16, v_offset: u16) -> io::Result<()> {
    let bottom_border_y = v_offset + game.height + 2;

    // 1. Draw top and bottom boundaries
    execute!(
        stdout,
        cursor::MoveTo(h_offset, v_offset),
        Print("#".repeat(game.width as usize).with(Color::DarkYellow)),
        cursor::MoveTo(h_offset, bottom_border_y - 1),
        Print("#".repeat(game.width as usize).with(Color::DarkYellow))
    )?;

    // 2. Clear the inside and draw side walls
    for y in 0..game.height {
        let current_y = v_offset + 1 + y;
        execute!(
            stdout,
            cursor::MoveTo(h_offset, current_y),
            Print("#"),                                   // Left Wall
            Print(" ".repeat((game.width - 2) as usize)), // Clear the middle
            cursor::MoveTo(h_offset + game.width - 1, current_y),
            Print("#") // Right Wall
        )?;
    }
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), cursor::Show)?;
    println!("\n\n");
    Ok(())
}

// --- MAIN FUNCTION ---

/// Main program logic
fn main() -> io::Result<()> {
    let default_panic_hook = panic::take_hook(); // set up panic hook
    panic::set_hook(Box::new(move |panic_info| {
        _ = restore_terminal();
        // Call the default hook to print the standard panic message/backtrace
        default_panic_hook(panic_info);
    }));

    println!("Finding sizes of chars, please wait...");
    let mut stdout = io::stdout();
    let star_is_double: bool = get_actual_width(&mut stdout, "✴️") == 2;

    // 1. Get Terminal Size and Define Game Area
    let (mut cols, mut rows) = terminal::size()?;

    const HEADER_HEIGHT: u16 = 4;
    const MAX_GAME_WIDTH: u16 = 100;

    let mut game_width = cols.min(MAX_GAME_WIDTH);
    let mut game_height = rows.wrapping_sub(HEADER_HEIGHT).max(10) - 1; // size of game, - 1 for bottom row

    let mut game = Game::new(game_width, game_height, load(), star_is_double); // Make 'game' mutable

    // Terminal coordinates for the drawing area
    const TOP_BORDER_Y: u16 = 3;
    let horizontal_offset = (cols.wrapping_sub(game.width)) / 2;

    // Define the Position struct for the apple decoration
    let apple_decoration_pos = Position {
        x: (game_width / 2) + 10,
        y: u16::MAX - 3,
    };

    // 2. Initial Setup: Enable raw mode, clear screen, and hide cursor
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::Clear(ClearType::All), cursor::Hide)?;

    // --- GAME LOOP BEGINS HERE ---

    let mut tick_rate_ms = TickRate::new(100, 50);
    let mut next_direction: Direction = Direction::Left;
    let mut tail_pos = Position { x: 0, y: 0 };
    'main: loop {
        // Stop movement if game is over, but still allow 'q' to quit
        while game.is_game_over {
            // will print message forever until 'q' or 'r' is pressed
            let input: KeyCode;
            if game.high_score < game.score {
                input =
                    game.print_message("New high score! Press 'r' to restart, or 'q' to quit", 3)?;
            } else {
                input = game.print_message("Game Over! Press 'r' to restart, or 'q' to quit", 3)?;
            }
            if input == KeyCode::Char('q') {
                if game.high_score < game.score {
                    game.high_score = game.score;
                }
                break 'main;
            } else if input == KeyCode::Char('r') {
                let high_score = if game.high_score < game.score {
                    game.score
                } else {
                    game.high_score
                };
                tick_rate_ms.vertical = 100;
                tick_rate_ms.horizontal = 50;

                // reset the terminal size variables in case the terminal window was resized

                (cols, rows) = terminal::size()?;
                game_width = cols.min(MAX_GAME_WIDTH);
                game_height = rows.wrapping_sub(HEADER_HEIGHT).max(10) - 1; // size of game, - 1 for bottom row

                game = Game::new(game_width, game_height, high_score, game.star_is_double); // reset the game
            }
            draw_playfield(&mut stdout, &game, horizontal_offset, TOP_BORDER_Y)?;
        }

        if game.is_won {
            // user found both Easter Eggs
            game.print_message("Congratulations! You beat this unbeatable game!", 3)?;
            execute!(stdout, cursor::MoveTo(0, rows))?;
            game.high_score = game.score;
            break;
        }

        // --- 1. INPUT (Non-blocking) ---
        if event::poll(Duration::from_millis(
            tick_rate_ms.get_correct(&game.snake.direction).into(),
        ))? {
            if let event::Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q') {
                    break;
                } else if key_event.code == KeyCode::Char('p') {
                    loop {
                        let input = game.print_message("Press 'r' to resume, or 'q' to quit", 3)?;
                        if input == KeyCode::Char('q') {
                            break 'main;
                        } else if input == KeyCode::Char('r') {
                            break;
                        } else if input == KeyCode::Char('~') {
                            game.print_message(
                                "Game created by Freddie. Press any key to continue.",
                                3,
                            )?;
                            game.snake.is_rainbow = true;
                            break;
                        }
                    }
                    draw_playfield(&mut stdout, &game, horizontal_offset, TOP_BORDER_Y)?;
                }

                // Determine new direction based on input keys (WASD or Arrows)
                let new_direction = match key_event.code {
                    KeyCode::Up | KeyCode::Char('w') => Some(Direction::Up),
                    KeyCode::Down | KeyCode::Char('s') => Some(Direction::Down),
                    KeyCode::Left | KeyCode::Char('a') => Some(Direction::Left),
                    KeyCode::Right | KeyCode::Char('d') => Some(Direction::Right),
                    _ => None,
                };

                if let Some(dir) = new_direction {
                    next_direction = dir;
                }
            }
            game.snake.change_direction(next_direction);
        }

        // --- 2. LOGIC (Update game state) ---
        if !game.is_game_over {
            let mut remove_end_of_tail: bool = true;
            if !game.check_for_food_collision() {
                remove_end_of_tail = false;
                tick_rate_ms.decrease();
            }

            if game.snake.get_next_head_pos() == apple_decoration_pos {
                game.is_won = true; // user wins
                game.score = u32::MAX;
            }

            game.is_game_over =
                game.snake
                    .check_for_wall_collision(game.width, game.height, game.score); // Check for wall collision

            game.is_game_over = game.is_game_over || game.snake.check_for_self_collision(); // Check for self collision. If the game is already over, checking for self-collision doesn't matter

            tail_pos = game.snake.update(remove_end_of_tail); // Move the snake!

            if !remove_end_of_tail {
                // if the the last char need not be removed, then the snake must have just eaten
                draw_playfield(&mut stdout, &game, horizontal_offset, TOP_BORDER_Y)?; // redraw the board to make sure all of the food is overwritten
            }
        }

        // --- 3. DRAWING ---

        // Always reset the cursor to the very top-left corner (0, 0)
        execute!(stdout, cursor::MoveTo(0, 0))?;

        // Draw Centered Header and Score (Score is now dynamic)
        let header_text = format!("🐍 RUST SNAKE GAME ({}) ", game.width);

        let padding = (game.width.wrapping_sub(header_text.len() as u16)) / 2;

        execute!(
            stdout,
            cursor::MoveTo(horizontal_offset + padding, 0),
            Print(header_text.with(Color::Green))
        )?;

        execute!(
            stdout,
            cursor::MoveTo(horizontal_offset, 1),
            Print(
                format!(
                    " {} Game Area: {}x{} {}",
                    "=".repeat((game.width / 3) as usize),
                    game.width,
                    game.height,
                    "=".repeat((game.width / 3) as usize)
                )
                .with(Color::DarkGrey)
            )
        )?;

        execute!(
            stdout,
            cursor::MoveTo(horizontal_offset, 2),
            Print(
                format!(
                    "Score: {} | High Score: {} | Press 'p' to pause, 'q' to quit   ",
                    game.score, game.high_score
                )
                .with(Color::White)
            )
        )?;

        // Draw fixed playfield area with boundaries
        let bottom_border_y = TOP_BORDER_Y + game.height + 2;

        // Draw top and bottom boundaries
        execute!(
            stdout,
            cursor::MoveTo(horizontal_offset, TOP_BORDER_Y),
            Print("#".repeat(game.width as usize).with(Color::DarkYellow))
        )?;

        execute!(
            stdout,
            cursor::MoveTo(horizontal_offset, bottom_border_y - 1),
            Print("#".repeat(game.width as usize).with(Color::DarkYellow))
        )?;
        // Draw side boundaries and empty space
        for y in 0..game.height {
            let current_y = TOP_BORDER_Y + 1 + y;

            // Move to the start of the line inside the boundary
            execute!(
                stdout,
                cursor::MoveTo(horizontal_offset, current_y),
                Print("#".with(Color::DarkYellow)) // Left Wall
            )?;

            /*// Fill the middle empty space
            execute!(stdout,
                cursor::MoveTo(horizontal_offset + 1, current_y),
                Print(" ".repeat((game.width - 1) as usize))
            )?; */

            // Move to the end of the line
            execute!(
                stdout,
                cursor::MoveTo(horizontal_offset + game.width - 1, current_y),
                Print("#".with(Color::DarkYellow)) // Right Wall
            )?;
        }

        // --- 4. DRAW SNAKE ---
        //let mut times = 1;
        for (i, pos) in game.snake.body.iter().enumerate() {
            //times += 1;
            // Translate game coordinates (pos.x, pos.y) to screen coordinates
            let draw_x = pos.x.wrapping_add(horizontal_offset + 1);
            let draw_y = pos.y.wrapping_add(TOP_BORDER_Y + 1);

            let (symbol, mut color) = if i == 0 {
                // Head
                ("◈", Color::DarkRed) // was ⦿
            } else {
                // Body
                ("█", Color::DarkRed)
            };

            if game.snake.is_rainbow == true && symbol != "◈" {
                color = match (pos.x + pos.y) % 6 {
                    0 => Color::Red,
                    1 => Color::Rgb {
                        r: 255,
                        g: 130,
                        b: 0,
                    }, // orange
                    2 => Color::Yellow,
                    3 => Color::Green,
                    4 => Color::Blue,
                    5 => Color::Magenta,
                    _ => panic!("Out of range 1-6"),
                }
            }

            execute!(
                stdout,
                cursor::MoveTo(draw_x, draw_y),
                Print(symbol.with(color))
            )?;
        }

        let (tail_x, tail_y) = tail_pos.to_screen(horizontal_offset, TOP_BORDER_Y);
        execute!(stdout, cursor::MoveTo(tail_x, tail_y))?;
        write!(stdout, " ")?;

        let apple_draw_x = apple_decoration_pos.x.wrapping_add(horizontal_offset + 1);
        let apple_draw_y = apple_decoration_pos.y.wrapping_add(TOP_BORDER_Y + 1);
        execute!(
            stdout,
            cursor::MoveTo(apple_draw_x, apple_draw_y),
            Print('🍎')
        )?;

        // --- 5. Draw food ---

        for food in game.snake.food_array.iter() {
            let food_pos = food.pos;
            let draw_x = horizontal_offset + 1 + food_pos.x; // Horizontal offset + Left wall (#) offset + Game X
            let draw_y = TOP_BORDER_Y + 1 + food_pos.y; // Top Border Y + Top wall (#) offset + Game Y
            let (symbol, color) = if food.is_special {
                ("✴️", Color::Yellow)
            } else {
                ("●", Color::White)
            };

            execute!(
                stdout,
                cursor::MoveTo(draw_x, draw_y),
                Print(symbol.with(color))
            )?;
        }

        // Always put the cursor way down to prevent terminal scrolling
        execute!(stdout, cursor::MoveTo(0, rows))?;

        // Flush the buffer to ensure all output is sent to the terminal immediately
        stdout.flush()?;
    }
    // --- GAME LOOP ENDS HERE ---

    // 4. Cleanup: Show the cursor and exit raw mode before the program ends.
    execute!(stdout, cursor::Show, style::ResetColor)?;
    terminal::disable_raw_mode()?;
    println!(); // print a newline
    save(game.high_score);
    Ok(())
}
