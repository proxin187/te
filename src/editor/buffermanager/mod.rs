use crate::editor::*;


#[derive(Debug)]
pub struct Buffer {
    matches: Matches,
    filename:  String,

    clamp: usize,
    cursor: Cursor,
    screen: Screen,
    syntax: highlight::Syntax,
}

#[derive(Debug)]
pub struct BufferManager {
    pub buffers: Vec<Buffer>,
    pub current: usize,
}

impl BufferManager {
    pub fn new(editor: &Editor) -> BufferManager {
        let mut manager = BufferManager {
            buffers: Vec::new(),
            current: 0,
        };

        manager.load_buffer(editor);

        manager
    }

    pub fn load_buffer(&mut self, editor: &Editor) {
        self.buffers.push(Buffer {
            matches:  editor.matches.clone(),
            filename: editor.filename.clone(),

            clamp:  editor.clamp,
            cursor: editor.cursor,
            screen: editor.screen,
            syntax: editor.syntax.clone(),
        });
    }

    pub fn close_buffer(&mut self, editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
        if self.buffers.len() > 1 {
            let _ = self.buffers.remove(self.current);
            self.previous_buffer(editor)?;
        }
        Ok(())
    }

    pub fn save_buffer(&mut self, editor: &Editor) {
        self.buffers[self.current] = Buffer {
            matches:  editor.matches.clone(),
            filename: editor.filename.clone(),

            clamp:  editor.clamp,
            cursor: editor.cursor,
            screen: editor.screen,
            syntax: editor.syntax.clone(),
        };
    }

    pub fn reload(&mut self, editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
        editor.open_file(&self.buffers[self.current].filename)?;

        editor.cursor = self.buffers[self.current].cursor;
        editor.screen = self.buffers[self.current].screen;

        editor.matches = self.buffers[self.current].matches.clone();

        editor.clamp = self.buffers[self.current].clamp;
        editor.syntax = self.buffers[self.current].syntax.clone();

        editor.refresh = true;

        Ok(())
    }

    pub fn next_buffer(&mut self, editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
        if self.current < self.buffers.len() - 1 {
            self.current += 1;
        }

        self.reload(editor)?;
        Ok(())
    }

    pub fn previous_buffer(&mut self, editor: &mut Editor) -> Result<(), Box<dyn std::error::Error>> {
        if self.current > 0 {
            self.current -= 1;
        }

        self.reload(editor)?;
        Ok(())
    }
}


