use ropey::Rope;
use unicode_width::UnicodeWidthStr;

use crate::{
    layout::Layout,
    text::{cursor, CharIndex, TextStorage},
    AnyCharacter, Bindings, Callback, Canvas, Colour, Component, ComponentLink, KeyCode, KeyEvent,
    KeyModifiers, Rect, ShouldRender, Style,
};

pub use crate::text::Cursor;

#[derive(Clone, PartialEq)]
pub struct InputProperties {
    pub style: InputStyle,
    pub content: Rope,
    pub cursor: Cursor,
    pub on_change: Option<Callback<InputChange>>,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputStyle {
    pub content: Style,
    pub cursor: Style,
}

impl Default for InputStyle {
    fn default() -> Self {
        const DARK0_SOFT: Colour = Colour::rgb(50, 48, 47);
        const LIGHT2: Colour = Colour::rgb(213, 196, 161);
        const BRIGHT_BLUE: Colour = Colour::rgb(131, 165, 152);

        Self {
            content: Style::normal(DARK0_SOFT, LIGHT2),
            cursor: Style::normal(BRIGHT_BLUE, DARK0_SOFT),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InputChange {
    pub content: Option<Rope>,
    pub cursor: Cursor,
}

pub struct Input {
    properties: InputProperties,
    frame: Rect,
}

impl Component for Input {
    type Message = Message;
    type Properties = InputProperties;

    fn create(properties: Self::Properties, frame: Rect, _link: ComponentLink<Self>) -> Self {
        let mut content = properties.content.clone();
        cursor::ensure_trailing_newline_with_content(&mut content);
        Self { properties, frame }
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        if self.properties != properties {
            self.properties = properties;
            ShouldRender::Yes
        } else {
            ShouldRender::No
        }
    }

    fn resize(&mut self, frame: Rect) -> ShouldRender {
        self.frame = frame;
        ShouldRender::Yes
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        let mut cursor = self.properties.cursor.clone();
        let mut content_change = None;
        match message {
            Message::CursorLeft => {
                cursor.move_left(&self.properties.content);
            }
            Message::CursorRight => {
                cursor.move_right(&self.properties.content);
            }
            Message::StartOfLine => {
                cursor.move_to_start_of_line(&self.properties.content);
            }
            Message::EndOfLine => {
                cursor.move_to_end_of_buffer(&self.properties.content);
            }
            Message::InsertChar(character) => {
                let mut new_content = self.properties.content.clone();
                cursor.insert_char(&mut new_content, character);
                cursor.move_right(&new_content);
                content_change = Some(new_content);
            }
            Message::DeleteBackward => {
                let mut new_content = self.properties.content.clone();
                cursor.backspace(&mut new_content);
                content_change = Some(new_content);
            }
            Message::DeleteForward => {
                let mut new_content = self.properties.content.clone();
                cursor.delete(&mut new_content);
                content_change = Some(new_content);
            }
        }

        if let Some(on_change) = self.properties.on_change.as_mut() {
            on_change.emit(InputChange {
                cursor,
                content: content_change,
            });
        }

        ShouldRender::Yes
    }

    fn view(&self) -> Layout {
        let Self {
            properties:
                InputProperties {
                    ref content,
                    ref cursor,
                    ref style,
                    ..
                },
            ..
        } = *self;

        let mut canvas = Canvas::new(self.frame.size);
        canvas.clear(style.content);

        let mut char_offset = 0;
        let mut visual_offset = 0;
        for grapheme in content.graphemes() {
            let len_chars = grapheme.len_chars();
            // TODO: don't unwrap (need to be able to create a smallstring from a rope slice)
            let grapheme = grapheme.as_str().unwrap();
            let grapheme_width = UnicodeWidthStr::width(grapheme);

            canvas.draw_str(
                visual_offset,
                0,
                if cursor.range().contains(&CharIndex(char_offset)) {
                    style.cursor
                } else {
                    style.content
                },
                if grapheme_width > 0 { grapheme } else { " " },
            );
            visual_offset += grapheme_width;
            char_offset += len_chars;
        }

        canvas.into()
    }

    fn bindings(&self, bindings: &mut Bindings<Self>) {
        bindings.set_focus(self.properties.focused);
        if !bindings.is_empty() {
            return;
        }

        bindings
            .command("left", || Message::CursorLeft)
            .with([KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::Left)]);
        bindings
            .command("right", || Message::CursorRight)
            .with([KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::Right)]);
        bindings
            .command("start-of-line", || Message::StartOfLine)
            .with([KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::Home)]);
        bindings
            .command("end-of-line", || Message::EndOfLine)
            .with([KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::End)]);
        bindings
            .command("delete-forward", || Message::DeleteForward)
            .with([KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::Delete)]);
        bindings.add(
            "delete-backward",
            [KeyEvent::from(KeyCode::Backspace)],
            || Message::DeleteBackward,
        );
        bindings.add(
            "insert-character",
            AnyCharacter,
            |keys: &[KeyEvent]| match keys {
                &[KeyEvent {
                    code: KeyCode::Char(character),
                    modifiers: _,
                }] if character != '\n' && character != '\r' && character != '\t' => {
                    Some(Message::InsertChar(character))
                }
                _ => None,
            },
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    CursorLeft,
    CursorRight,
    InsertChar(char),
    DeleteBackward,
    DeleteForward,
    StartOfLine,
    EndOfLine,
}
