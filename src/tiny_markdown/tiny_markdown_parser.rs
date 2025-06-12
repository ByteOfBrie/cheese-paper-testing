#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item<'a> {
    Newline,
    Text(Style, &'a str),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    pub strong: bool,
    pub italic: bool,
}

pub struct Parser<'a> {
    s: &'a str,

    style: Style,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            style: Style::default(),
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.s.is_empty() {
                return None;
            }

            if self.s.starts_with('\n') {
                self.s = &self.s[1..];
                self.style = Style::default();
                return Some(Item::Newline);
            }

            if let Some(rest) = self.s.strip_prefix("**") {
                self.s = rest;
                self.style.strong = !self.style.strong;
                continue;
            }

            if let Some(rest) = self.s.strip_prefix("*") {
                self.s = rest;
                self.style.italic = !self.style.italic;
                continue;
            }

            let end = self
                .s
                .find(&['*', '\n'][..])
                .map_or_else(|| self.s.len(), |special| special.max(1));

            let item = Item::Text(self.style, &self.s[..end]);
            self.s = &self.s[end..];
            return Some(item);
        }
    }
}

#[test]
fn test_basic_text_parser() {
    let items: Vec<_> = Parser::new("*italic* **bold**").collect();
    assert_eq!(
        items,
        vec![
            Item::Text(
                Style {
                    italic: true,
                    ..Default::default()
                },
                "italic"
            ),
            Item::Text(
                Style {
                    ..Default::default()
                },
                " "
            ),
            Item::Text(
                Style {
                    strong: true,
                    ..Default::default()
                },
                "bold"
            ),
        ]
    );
}

#[test]
fn test_complex_markdown() {
    let items: Vec<_> = Parser::new(
        r#"none**bold*both**
*italic***bold"#,
    )
    .collect();
    assert_eq!(
        items,
        vec![
            Item::Text(
                Style {
                    ..Default::default()
                },
                "none"
            ),
            Item::Text(
                Style {
                    strong: true,
                    ..Default::default()
                },
                "bold"
            ),
            Item::Text(
                Style {
                    strong: true,
                    italic: true,
                    ..Default::default()
                },
                "both"
            ),
            Item::Newline,
            Item::Text(
                Style {
                    italic: true,
                    ..Default::default()
                },
                "italic"
            ),
            Item::Text(
                Style {
                    strong: true,
                    ..Default::default()
                },
                "bold"
            ),
        ]
    );
}
