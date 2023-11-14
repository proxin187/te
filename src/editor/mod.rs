mod highlight;
mod buffermanager;

use std::process;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::{self, Write};

use console::{Term, Key};
use buffermanager::BufferManager;

const BOTTOM_BAR: usize = 2;
const TAB_SIZE:   usize = 4;
const PARAGRAPH:  usize = 47;

#[derive(Debug, PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
}

impl Mode {
    fn as_string(&self) -> String {
        format!(" {:?} ", self)
    }
}

struct Visual {
    x: usize,
    y: usize,
    select_line: bool,
}

impl Visual {
    fn new(cursor: &Cursor, select_line: bool) -> Visual {
        Visual {
            x: cursor.x,
            y: cursor.y,
            select_line,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug)]
pub struct Screen {
    x: usize,
    y: usize,
    height: usize,
    width: usize,
}

impl Screen {
    fn new() -> Screen {
        Screen {
            x: 0,
            y: 0,
            height: 0,
            width: 0,
        }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.y = 0;
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    x: usize,
    y: usize,
}

impl Cursor {
    fn new() -> Cursor {
        Cursor {
            x: 0,
            y: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Matches {
    matches: Vec<Cursor>,
    index: usize,
}

impl Matches {
    fn new() -> Matches {
        Matches {
            matches: Vec::new(),
            index: 0,
        }
    }
}

pub struct Editor {
    buffer:    Vec<Vec<char>>,
    clipboard: Vec<Vec<char>>,
    matches: Matches,
    filename:  String,

    clamp: usize,
    cursor: Cursor,
    term:   Term,
    screen: Screen,
    syntax: highlight::Syntax,

    refresh: bool,

    mode: Mode,
    log:  String,
}

impl Editor {
    pub fn new(filename: &str) -> Result<Editor, Box<dyn std::error::Error>> {
        Ok(Editor {
            buffer:    Vec::new(),
            clipboard: Vec::new(),
            matches:   Matches::new(),
            filename:  String::from("*New Buffer*"),

            clamp: 0,
            cursor: Cursor::new(),
            term:   Term::stdout(),
            screen: Screen::new(),
            syntax: highlight::Syntax::new(filename)?,

            refresh: true,

            mode:   Mode::Normal,
            log:    String::new(),
        })
    }

    pub fn reset(&mut self) {
        self.cursor = Cursor::new();
        self.screen.reset();
    }

    pub fn open_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let fd = File::open(file_path)?;
        let buf = BufReader::new(fd);

        self.buffer.drain(..);

        for line in buf.lines() {
            self.buffer.push(line?.chars().collect::<Vec<char>>());
        }

        if self.buffer.is_empty() {
            self.buffer.push(Vec::new());
        }

        self.filename = file_path.to_string();
        self.syntax = highlight::Syntax::new(&self.filename)?;
        Ok(())
    }

    pub fn log(&mut self, message: &str) {
        self.log = message.to_string();
        self.refresh = true;
    }

    fn render_bar(&self, manager: &BufferManager) -> String {
        let mut bar = String::new();
        let mut size = 0;

        // Mode
        bar += &self.syntax.colors.mode.apply_to(self.mode.as_string().to_uppercase()).to_string();
        size += self.mode.as_string().len();

        // Filename
        bar += &self.syntax.colors.bar.apply_to(" ".to_string()  + &self.filename).to_string();
        size += self.filename.len() + 1;

        // Filetype
        bar += &self.syntax.colors.bar.apply_to(&format!(" [{}]", self.syntax.filetype)).to_string();
        size += self.syntax.filetype.len() + 3;

        // Buffer number
        let buf = format!(" [{}/{}] ", manager.current + 1, manager.buffers.len());
        size += buf.len();

        // Line number
        let line = format!(" {}:{} ", self.cursor.y + 1, self.cursor.x + 1);
        size += line.len();

        // Middle padding
        bar += &self.syntax.colors.bar.apply_to((size..self.screen.width).map(|_| " ").collect::<String>()).to_string();

        // Buffer number
        bar += &self.syntax.colors.bar.apply_to(&buf).to_string();

        // Line number
        bar += &self.syntax.colors.bar.apply_to(&line).to_string();

        bar
    }

    fn refresh_bar(&mut self, manager: &BufferManager) -> Result<(), Box<dyn std::error::Error>> {
        self.term.move_cursor_to(0, self.screen.height - 2)?;
        self.term.write_line(&self.render_bar(manager))?;
        Ok(())
    }

    fn render_log(&self) -> String {
        let mut output = String::new();

        // Message
        output += &self.syntax.colors.default.apply_to(&self.log).to_string();

        // Padding
        output += &self.syntax.colors.default.apply_to((self.log.len()..self.screen.width).map(|_| " ").collect::<String>()).to_string();

        output
    }

    fn render_line_number(&self, index: usize) -> String {
        let cursor_position = self.cursor.y - self.screen.y;

        return if index > cursor_position {
            // UNDER
            self.syntax.colors.line_numbers.apply_to(format!("{:02} ", index - cursor_position)).to_string()
        } else if index < cursor_position {
            // OVER
            self.syntax.colors.line_numbers.apply_to(format!("{:02} ", cursor_position - index)).to_string()
        } else {
            self.syntax.colors.line_numbers.apply_to("-> ").to_string()
        };
    }

    fn refresh_line_numbers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.term.move_cursor_to(0, 0)?;

        for index in 0..self.screen.height - BOTTOM_BAR {
            self.term.write_line(&self.render_line_number(index))?;
        }

        Ok(())
    }

    fn empty_line(&self, index: usize) -> bool {
        if self.buffer.len() <= index {
            return true;
        } else if self.buffer[index].len() == 0 {
            return true;
        }
        return false;
    }

    fn render(&mut self, manager: &BufferManager) -> Result<(), Box<dyn std::error::Error>> {
        if !self.refresh {
            self.refresh_line_numbers()?;
            self.refresh_bar(manager)?;
            self.term.move_cursor_to(self.cursor.x + 3, self.cursor.y - self.screen.y)?;
            return Ok(()); // return if refresh is false
        }

        self.term.hide_cursor()?;
        self.term.move_cursor_to(0, 0)?;

        for index in self.screen.y..self.screen.y + self.screen.height - BOTTOM_BAR {
            if self.empty_line(index) {
                // fill the empty space with background color
                let mut line = (0..self.screen.width - 3 /* Length of line number */).map(|_| " ").collect::<String>();

                if self.buffer.len() <= index {
                    line.pop();
                    line = format!("{}{}", self.render_line_number(index - self.screen.y), console::style(String::from("~") + &line).on_color256(self.syntax.colors.background));
                } else {
                    line = format!("{}{}", self.render_line_number(index - self.screen.y), console::style(line).on_color256(self.syntax.colors.background));
                }

                self.term.write_line(&line)?;
            } else {
                let length = self.buffer[index].len();
                let line = &self.buffer[index].clone()[self.screen.x..length.clamp(self.screen.x, self.screen.x + self.screen.width)];

                let mut line = match self.syntax.highlight(line.into_iter().collect()) {
                    Ok(line) => line,
                    Err(_) => {
                        return Err("syntax highlighting has failed".into());
                    },
                };

                // relative line numbers
                line = self.render_line_number(index - self.screen.y) + &line;

                // add padding
                let padding = &(0..self.screen.width - self.buffer[index].len() - 4 /* Length of line number */).map(|_| " ").collect::<String>();
                line = line + &format!("{}", console::style(padding).on_color256(self.syntax.colors.background));

                self.term.write_line(&line)?;
            }
        }

        // bar
        self.term.write_line(&self.render_bar(manager))?;

        // log
        // NOTE: print is used here because we dont want a newline
        self.term.clear_line()?;
        print!("{}", self.render_log());
        io::stdout().flush()?;

        self.term.show_cursor()?;
        self.term.move_cursor_to(self.cursor.x + 3, self.cursor.y - self.screen.y)?;

        self.refresh = false;
        Ok(())
    }

    fn move_cursor(&mut self, direction: Direction) {
        match direction {
            Direction::Left => {
                if self.cursor.x > 0 {
                    self.cursor.x -= 1;
                }

                self.clamp = self.cursor.x;
            },
            Direction::Right => {
                if self.cursor.x < self.buffer[self.cursor.y].len() {
                    self.cursor.x += 1;
                }

                self.clamp = self.cursor.x;
            },
            Direction::Up => {
                if self.cursor.y > 0 {
                    if self.cursor.y <= self.screen.y {
                        self.screen.y -= 1;
                        self.refresh = true;
                    }
                    self.cursor.y -= 1;
                }
            },
            Direction::Down => {
                if self.cursor.y < self.buffer.len() - 1 {
                    if self.cursor.y >= self.screen.height + self.screen.y - BOTTOM_BAR - 1 {
                        self.screen.y += 1;
                        self.refresh = true;
                    }
                    self.cursor.y += 1;
                }
            },
        }
    }

    fn clamp_cursor(&mut self) {
        if self.cursor.x > self.buffer[self.cursor.y].len() || self.clamp > self.buffer[self.cursor.y].len() {
            self.cursor.x = self.buffer[self.cursor.y].len();
        } else if self.clamp < self.buffer[self.cursor.y].len() {
            self.cursor.x = self.clamp;
        }
    }

    fn insert(&mut self, character: char) {
        self.buffer[self.cursor.y].insert(self.cursor.x, character);
        self.move_cursor(Direction::Right);
        self.refresh = true;
    }

    fn remove(&mut self) {
        if self.cursor.x != 0 {
            // delete char
            self.move_cursor(Direction::Left);
            self.buffer[self.cursor.y].remove(self.cursor.x);
        } else if self.cursor.y != 0 {
            let line_len = self.buffer[self.cursor.y - 1].len();

            // append line under onto the line over
            let line = self.buffer[self.cursor.y].clone();
            self.buffer[self.cursor.y - 1].extend(line);

            // remove line under
            self.buffer.remove(self.cursor.y);
            self.move_cursor(Direction::Up);

            // move cursor to where the old length of the line over used to be
            self.cursor.x = line_len;
        }
        self.clamp = self.cursor.x;
        self.refresh = true;
    }

    fn indentation(&self) -> usize {
        for (index, character) in self.buffer[self.cursor.y - 1].iter().enumerate() {
            if *character != ' ' {
                return index;
            }
        }

        return 0;
    }

    fn newline(&mut self, cut: bool) {
        if self.buffer.len() == 1 || self.cursor.y >= self.buffer.len() - 1 {
            self.buffer.push(Vec::new());
            self.move_cursor(Direction::Down);
        } else {
            self.move_cursor(Direction::Down);
            self.buffer.insert(self.cursor.y, Vec::new());
        }

        if cut {
            self.buffer[self.cursor.y] = self.buffer[self.cursor.y - 1][self.cursor.x..self.buffer[self.cursor.y - 1].len()].to_vec();
            self.buffer[self.cursor.y - 1] = self.buffer[self.cursor.y - 1][0..self.cursor.x].to_vec();
            self.cursor.x = 0;
        } else {
            let indentation = self.indentation();
            self.buffer[self.cursor.y] = " ".repeat(indentation)
                .as_bytes()
                .iter()
                .map(|e| *e as char)
                .collect::<Vec<char>>();

            self.cursor.x = indentation;
        }

        self.clamp = self.cursor.x;
        self.refresh = true;
    }

    fn range(&self, visual: usize, cursor: usize) -> (std::ops::Range<usize>, Direction) {
        return if visual < cursor {
            if cursor == visual {
                (visual..cursor + 1, Direction::Down)
            } else {
                (visual..cursor, Direction::Down)
            }
        } else {
            if cursor == visual {
                (cursor..visual + 1, Direction::Up)
            } else {
                (cursor..visual, Direction::Up)
            }
        };
    }

    fn delete(&mut self, visual: &Visual) {
        if visual.select_line {
            if self.buffer.len() == 1 {
                self.buffer[0] = Vec::new();
            } else if self.cursor.y >= self.buffer.len() - 1 {
                self.move_cursor(Direction::Up);
                self.buffer.pop();
            } else {
                let range = self.range(visual.y, self.cursor.y);
                let deleted = self.buffer.drain(range.0).collect::<Vec<Vec<char>>>();

                if range.1 != Direction::Up {
                    for _ in deleted {
                        self.move_cursor(Direction::Up);
                    }
                }
            }
        } else {
            let range = self.range(visual.x, self.cursor.x);

            let deleted = self.buffer[self.cursor.y].drain(range.0).collect::<Vec<char>>();

            if range.1 != Direction::Up {
                for _ in deleted {
                    self.move_cursor(Direction::Left);
                }
            }
        }
    }

    fn paste(&mut self, visual: &Visual) {
        if visual.select_line {
            self.move_cursor(Direction::Down);
            for line in self.clipboard.iter().rev() {
                self.buffer.insert(self.cursor.y, line.clone());
            }
        } else {
            let content = self.clipboard[0].clone();
            for character in &content {
                self.buffer[self.cursor.y].insert(self.cursor.x, *character);
                self.move_cursor(Direction::Right);
            }
        }
    }

    fn copy(&mut self, visual: &Visual) {
        if visual.select_line {
            self.clipboard = self.buffer[self.range(visual.y, self.cursor.y).0].to_vec();
        } else {
            self.clipboard = vec![self.buffer[self.cursor.y][self.range(visual.x, self.cursor.x).0].to_vec()];
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut fd = File::create(&self.filename)?;
        for line in &self.buffer {
            fd.write_all(&line.iter().map(|e| *e as u8).collect::<Vec<u8>>())?;
            fd.write_all(b"\n")?;
        }

        Ok(())
    }

    fn log_save(&mut self) {
        if let Err(_) = self.save() {
            self.log(&format!("failed to write to `{}`", self.filename));
        } else {
            self.log(&format!("wrote to `{}`", self.filename));
        }
    }

    fn search(&mut self, query: &str) {
        self.matches.matches = Vec::new();

        for (y, line) in self.buffer.iter().enumerate() {
            let line = line.iter().collect::<String>();
            if let Some(x) = line.find(query) {
                self.matches.matches.push(Cursor {
                    y,
                    x,
                });
            }
        }

        self.goto_match();
    }

    fn next_match(&mut self) {
        if self.matches.index < self.matches.matches.len() {
            self.goto_match();
            self.matches.index += 1;
        }
    }

    fn previous_match(&mut self) {
        if self.matches.index > 0 {
            self.matches.index -= 1;
            self.goto_match();
        }
    }

    fn goto_match(&mut self) {
        if self.matches.index < self.matches.matches.len() {
            self.cursor = self.matches.matches[self.matches.index];
            self.screen.y = self.cursor.y;
            self.screen.x = self.cursor.x;
            self.refresh = true;
        }
    }

    fn command(&mut self, cmd: String, manager: &mut BufferManager) -> Result<(), Box<dyn std::error::Error>> {
        match cmd.as_str() {
            ":E" => {
                self.log_save();
            },
            ":EQ" => {
                self.log_save();
                process::exit(0);
            },
            ":q" => {
                process::exit(0);
            },
            ":qb" => {
                manager.close_buffer(self);
            },
            _ => {
                if cmd.starts_with(":/") {
                    self.search(&cmd[2..]);
                } else if cmd.starts_with(":O") {
                    self.open_file(&cmd[3..])?;
                    self.reset();

                    manager.load_buffer(&self);
                    manager.next_buffer(self)?;
                } else {
                    self.log(&format!("Unknown command: `{cmd}`"));
                }
            },
        }

        Ok(())
    }

    fn move_by_paragraph(&mut self, direction: Direction) {
        if direction == Direction::Up {
            if self.cursor.y < PARAGRAPH {
                self.cursor.y = 0;
                self.screen.y = 0;
            } else {
                self.cursor.y -= PARAGRAPH;
                self.screen.y = self.cursor.y;
            }
        } else if direction == Direction::Down {
            if self.cursor.y + PARAGRAPH >= self.buffer.len() {
                self.cursor.y = self.buffer.len() - 1;
                self.screen.y = self.buffer.len() - 1;
            } else {
                self.cursor.y += PARAGRAPH;
                self.screen.y += PARAGRAPH;
            }
        }
    }

    fn handle_escape(&mut self, manager: &mut BufferManager) -> Result<(), Box<dyn std::error::Error>> {
        let modifier = self.term.read_key()?;
        let arrow = self.term.read_key()?;

        match modifier {
            Key::Char('2') => {
                // Key: Shift
                if arrow == Key::Char('C') {
                    // Key: Right Arrow
                    self.cursor.x = self.syntax.next_token(self.buffer[self.cursor.y].iter().collect::<String>(), self.cursor.x)?;
                    self.clamp = self.cursor.x;
                } else if arrow == Key::Char('D') {
                    // Key: Left Arrow
                    self.cursor.x = self.syntax.previous_token(self.buffer[self.cursor.y].iter().collect::<String>(), self.cursor.x)?;
                    self.clamp = self.cursor.x;
                } else if arrow == Key::Char('A') {
                    // Key: Up Arrow
                    self.move_by_paragraph(Direction::Up);
                    self.refresh = true;
                } else if arrow == Key::Char('B') {
                    // Key: Down Arrow
                    self.move_by_paragraph(Direction::Down);
                    self.refresh = true;
                }
            },
            Key::Char('5') => {
                // Key: Ctrl
                manager.save_buffer(&self);
                let old = manager.current;

                if arrow == Key::Char('C') {
                    // Key: Right Arrow
                    manager.next_buffer(self)?;
                } else if arrow == Key::Char('D') {
                    // Key: Left Arrow
                    manager.previous_buffer(self)?;
                }

                if manager.current != old {
                    manager.reload(self)?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.term.clear_screen()?;

        let mut command: Vec<char> = Vec::new();
        let mut visual = Visual {
            x: 0,
            y: 0,
            select_line: false,
        };


        let size = self.term.size();

        self.screen.height = size.0 as usize;
        self.screen.width = size.1 as usize;

        let mut manager = BufferManager::new(&self);

        loop {
            if self.mode == Mode::Command {
                self.log(&command.iter().collect::<String>());

                self.clamp_cursor();
                self.render(&manager)?;

                self.term.move_cursor_to(command.len(), self.screen.height - 1)?;
            } else {
                self.clamp_cursor();
                self.render(&manager)?;
            }

            let key = self.term.read_key()?;

            match key {
                Key::ArrowUp => {
                    self.move_cursor(Direction::Up);
                },
                Key::ArrowDown => {
                    self.move_cursor(Direction::Down);
                },
                Key::ArrowLeft => {
                    self.move_cursor(Direction::Left);
                },
                Key::ArrowRight => {
                    self.move_cursor(Direction::Right);
                },
                Key::Enter => {
                    if self.mode == Mode::Insert {
                        self.newline(true);
                    } else if self.mode == Mode::Command {
                        if let Err(err) = self.command(command.iter().collect::<String>(), &mut manager) {
                            self.log(&err.to_string());
                        }
                        self.mode = Mode::Normal;
                        command = Vec::new();
                    }
                },
                Key::Backspace => {
                    if self.mode == Mode::Insert {
                        self.remove();
                    } else if self.mode == Mode::Command {
                        command.pop();
                    }
                },
                Key::Tab => {
                    if self.mode == Mode::Insert {
                        for _ in 0..TAB_SIZE {
                            self.buffer[self.cursor.y].insert(self.cursor.x, ' ');
                            self.move_cursor(Direction::Right);
                        }
                        self.refresh = true;
                    }
                },
                Key::Escape => {
                    command = Vec::new();

                    self.mode = Mode::Normal;
                    self.refresh = true;
                },
                Key::UnknownEscSeq(_) => {
                    self.handle_escape(&mut manager)?;
                },
                Key::Char(character) => {
                    if self.mode == Mode::Insert {
                        /* -- INSERT -- */
                        self.insert(character);
                    } else if self.mode == Mode::Command {
                        /* -- COMMAND -- */
                        command.push(character);
                    } else if self.mode == Mode::Visual {
                        /* -- VISUAL -- */
                        match character {
                            'y' => {
                                self.copy(&visual);

                                self.mode = Mode::Normal;
                                self.refresh = true;
                            },
                            'd' => {
                                self.copy(&visual);
                                self.delete(&visual);

                                self.mode = Mode::Normal;
                                self.refresh = true;
                            },
                            _ => {},
                        }
                    } else if self.mode == Mode::Normal {
                        /* -- NORMAL -- */
                        match character {
                            'v' | 'V' => {
                                visual = Visual::new(&self.cursor, if character == 'v' { false } else { true });
                                self.mode = Mode::Visual;
                            },
                            'd' | 'y' => {
                                visual = Visual::new(&self.cursor, true);
                                self.mode = Mode::Visual;
                            },
                            'p' => {
                                self.paste(&visual);
                            },
                            'o' => {
                                self.newline(false);
                                self.mode = Mode::Insert;
                            },
                            'i' => {
                                self.mode = Mode::Insert;
                            },
                            'n' => {
                                self.next_match();
                            },
                            'b' => {
                                self.previous_match();
                            },
                            ':' => {
                                command.push(character);
                                self.mode = Mode::Command;
                            },
                            _ => {},
                        }
                        self.refresh = true;
                    }
                },
                _ => {},
            }
        }
    }
}


