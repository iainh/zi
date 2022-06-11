use zi::{
    components::{
        border::{Border, BorderProperties},
        text::{Text, TextAlign, TextProperties},
    },
    prelude::*,
};
use zi_term::Result;

// Message type handled by the `Counter` component.
#[derive(Clone, Copy)]
enum Message {
    Increment,
    Decrement,
}

// Properties or the `Counter` component, in this case the initial value.
struct Properties {
    initial_count: usize,
}

// The `Counter` component.
struct Counter {
    // The state of the component -- the current value of the counter.
    count: usize,

    // A `ComponentLink` allows us to send messages to the component in reaction
    // to user input as well as to gracefully exit.
    link: ComponentLink<Self>,
}

// Components implement the `Component` trait and are the building blocks of the
// UI in Zi. The trait describes stateful components and their lifecycle.
impl Component for Counter {
    // Messages are used to make components dynamic and interactive. For simple
    // or pure components, this will be `()`. Complex, stateful ones will
    // typically use an enum to declare multiple Message types. In this case, we
    // will emit two kinds of message (`Increment` or `Decrement`) in reaction
    // to user input.
    type Message = Message;

    // Properties are the inputs to a Component passed in by their parent.
    type Properties = Properties;

    // Creates ("mounts") a new `Counter` component.
    fn create(properties: Self::Properties, _frame: Rect, link: ComponentLink<Self>) -> Self {
        Self {
            count: properties.initial_count,
            link,
        }
    }

    // Returns the current visual layout of the component.
    fn view(&self) -> Layout {
        let count = self.count;
        let text = move || {
            Text::with(
                TextProperties::new()
                    .align(TextAlign::Centre)
                    .style(STYLE)
                    .content(format!(
                        "\nCounter: {:>3}  [+ to increment | - to decrement | C-c to exit]",
                        count
                    )),
            )
        };
        Border::with(BorderProperties::new(text).style(STYLE))
    }

    // Components handle messages in their `update` method and commonly use this
    // method to update their state and (optionally) re-render themselves.
    fn update(&mut self, message: Self::Message) -> ShouldRender {
        let new_count = match message {
            Message::Increment => self.count.saturating_add(1),
            Message::Decrement => self.count.saturating_sub(1),
        };
        if new_count != self.count {
            self.count = new_count;
            ShouldRender::Yes
        } else {
            ShouldRender::No
        }
    }

    // Updates the key bindings of the component.
    //
    // This method will be called after the component lifecycle methods. It is
    // used to specify how to react in response to keyboard events, typically
    // by sending a message.
    fn bindings(&self, bindings: &mut Bindings<Self>) {
        // If we already initialised the bindings, nothing to do -- they never
        // change in this example
        if !bindings.is_empty() {
            return;
        }

        // Set focus to `true` in order to react to key presses
        bindings.set_focus(true);

        // Increment, when pressing + or =
        bindings
            .command("increment", || Message::Increment)
            .with([KeyEvent::from(KeyCode::Char('+'))])
            .with([KeyEvent::from(KeyCode::Char('='))]);

        // Decrement, when pressing -
        bindings.add("decrement", [KeyEvent::from(KeyCode::Char('-'))], || {
            Message::Decrement
        });

        // Exit, when pressing Esc or Ctrl-c
        bindings
            .command("exit", |this: &Self| this.link.exit())
            .with([KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)])
            .with([KeyEvent::from(KeyCode::Esc)]);
    }
}

const BACKGROUND: Colour = Colour::rgb(50, 48, 47);
const FOREGROUND: Colour = Colour::rgb(213, 196, 161);
const STYLE: Style = Style::bold(BACKGROUND, FOREGROUND);

fn main() -> Result<()> {
    env_logger::init();
    let counter = Counter::with(Properties { initial_count: 0 });
    zi_term::incremental()?.run_event_loop(counter)
}
