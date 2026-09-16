//! Tokenize inline HTML with the same HTML5 parser used by whole-document conversion.
use html5ever::tokenizer::{BufferQueue, Tag, Token, TokenSink, TokenSinkResult, Tokenizer};
use std::cell::RefCell;

#[derive(Default)]
struct Sink(RefCell<Option<Tag>>);
impl TokenSink for Sink {
    type Handle = ();
    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        if let Token::TagToken(tag) = token {
            *self.0.borrow_mut() = Some(tag);
        }
        TokenSinkResult::Continue
    }
}

pub(super) fn tag(raw: &str) -> Option<Tag> {
    let input = BufferQueue::default();
    input.push_back(raw.into());
    let parser = Tokenizer::new(Sink::default(), Default::default());
    let _ = parser.feed(&input);
    parser.end();
    parser.sink.0.into_inner()
}

pub(super) fn attr(tag: &Tag, name: &str) -> String {
    tag.attrs
        .iter()
        .find(|a| a.name.local.as_ref() == name)
        .map(|a| crate::markdown::sanitize(&a.value))
        .unwrap_or_default()
}
