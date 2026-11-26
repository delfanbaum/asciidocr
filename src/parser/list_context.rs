//! Keeps track of the lists that are open while the [`Parser`](super::Parser) works.
//!
//! Lists are held here, and not on the parser's block stack, until they are complete. An item's
//! text and the blocks attached to it (with a `+`) then go to that item by name, rather than to
//! whatever block happens to be on top of the block stack by the time the item ends.
//!
//! Lists live in scopes. A scope belongs to one level of the block stack: the lists at the top
//! level of a section or of a delimited block. A scope is active
//! only while the block stack is at that depth; a delimited block attached to an item pushes the
//! stack deeper, which suspends the scope until that block is complete. The parser must close a
//! scope before the block stack shrinks below its depth.
//!
//! Description lists live here too, so that lists nest in their items, and theirs in list items,
//! in the same way.

use crate::graph::{blocks::Block, inlines::Inline};

use crate::errors::BlockError;

/// A list that is still open, and the item currently being added to it
#[derive(Debug)]
struct OpenList {
    /// what items must have in common to belong to this list (see [`marker_key`])
    key: String,
    /// a `Block::List` or `Block::DList`, holding the items before the current one
    list: Block,
    /// the item being added to (a `Block::ListItem` or `Block::DListItem`), which joins the list
    /// when the next item starts or the list ends
    item: Block,
}

/// The lists open at one level of the block stack
#[derive(Debug)]
pub(super) struct ListScope {
    /// the length of the parser's block stack when the list started
    depth: usize,
    /// outermost first: each list after the first is nested in the current item of the one
    /// before it
    lists: Vec<OpenList>,
    /// the current item still takes lines of text: its principal text runs until the next item,
    /// a `+`, a blank line or the start of another block
    pub principal_open: bool,
    /// a `+` was seen, so the next block belongs to the current item
    pub attach_next: bool,
    /// a blank line was seen. A blank line doesn't end a list by itself: the next line does,
    /// unless it is an item or a `+`
    pub after_blank: bool,
}

impl ListScope {
    /// A scope is only created with a list in it, and is dropped when it closes, so there is
    /// always a current item
    fn current_item(&mut self) -> Result<&mut Block, BlockError> {
        self.lists
            .last_mut()
            .map(|open| &mut open.item)
            .ok_or(BlockError::NoOpenList)
    }

    /// Adds an inline to the current item's principal text
    pub fn push_principal(&mut self, inline: Inline) -> Result<(), BlockError> {
        self.current_item()?.push_inline(inline)
    }

    /// Attaches a block to the current item (a block that followed a `+`)
    pub fn attach_block(&mut self, block: Block) -> Result<(), BlockError> {
        self.attach_next = false;
        self.current_item()?.push_block(block)
    }

    /// Adds an item to the open list with its key, closing the lists nested deeper than that one.
    /// If no open list has the key, the item is handed back, to start a list nested in the
    /// current item (see [`nest`](Self::nest)). Lists nest by marker, not by marker length: after
    /// `**`, a `*` item closes back to the `*` list only if one is open, and otherwise nests
    /// deeper.
    pub fn add_item(&mut self, key: &str, item: Block) -> Result<Option<Block>, BlockError> {
        let Some(level) = self.lists.iter().position(|open| open.key == key) else {
            return Ok(Some(item));
        };
        self.close_deeper_than(level)?;
        let open = &mut self.lists[level];
        let finished = std::mem::replace(&mut open.item, item);
        open.list.push_block(finished)?;
        self.start_item();
        Ok(None)
    }

    /// Starts a list, with its first item, nested in the current item
    pub fn nest(&mut self, key: String, list: Block, item: Block) {
        self.lists.push(OpenList { key, list, item });
        self.start_item();
    }

    /// A new item takes the text that follows its marker
    fn start_item(&mut self) {
        self.principal_open = true;
        self.attach_next = false;
        self.after_blank = false;
    }

    /// Adds a term to the current item, if it is a description list item with no description
    /// yet: like Asciidoctor, `A::` directly followed by `B:: b` (even after a blank line) is one
    /// item with two terms. Otherwise the term is handed back, to start an item.
    pub fn add_term(&mut self, term: Vec<Inline>) -> Option<Vec<Inline>> {
        match self.lists.last_mut() {
            Some(OpenList {
                item: Block::DListItem(item),
                ..
            }) if item.principal.is_empty() && item.blocks.is_empty() => {
                item.add_term(term);
                self.start_item();
                None
            }
            _ => Some(term),
        }
    }

    /// A blank line ends the current item's text (and cancels a `+` with nothing attached yet)
    pub fn blank_line(&mut self) {
        self.principal_open = false;
        self.attach_next = false;
        self.after_blank = true;
    }

    /// A `+`: the next block belongs to the current item. After a blank line, it belongs to the
    /// top-level item instead (as in Asciidoctor), so the nested lists are complete.
    pub fn continuation(&mut self) -> Result<(), BlockError> {
        if self.after_blank {
            self.close_deeper_than(0)?;
            self.after_blank = false;
        }
        self.principal_open = false;
        self.attach_next = true;
        Ok(())
    }

    /// Closes the lists nested deeper than `level`, innermost first, each into the current item
    /// of the list it is nested in
    fn close_deeper_than(&mut self, level: usize) -> Result<(), BlockError> {
        while self.lists.len() > level + 1
            && let Some(mut open) = self.lists.pop()
        {
            open.list.push_block(open.item)?;
            let Some(parent) = self.lists.last_mut() else {
                return Err(BlockError::NoOpenList);
            };
            parent.item.push_block(open.list)?;
        }
        Ok(())
    }

    /// Closes every open list, and returns the outermost one
    fn close(mut self) -> Result<Option<Block>, BlockError> {
        self.close_deeper_than(0)?;
        let Some(open) = self.lists.pop() else {
            return Ok(None);
        };
        let mut list = open.list;
        list.push_block(open.item)?;
        Ok(Some(list))
    }
}

/// The open lists, by block stack depth
#[derive(Debug, Default)]
pub(super) struct ListContext {
    /// shallowest first
    scopes: Vec<ListScope>,
}

impl ListContext {
    /// The scope at this depth of the block stack, if it has open lists
    pub fn active(&mut self, depth: usize) -> Option<&mut ListScope> {
        self.scopes.last_mut().filter(|scope| scope.depth == depth)
    }

    /// Whether no list is open
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    /// Like [`active`](Self::active), but only to look
    pub fn get(&self, depth: usize) -> Option<&ListScope> {
        self.scopes.last().filter(|scope| scope.depth == depth)
    }

    /// Starts a new list (with its first item) at this depth of the block stack. The caller
    /// closes any list already open at this depth first.
    pub fn start_list(&mut self, depth: usize, key: String, list: Block, item: Block) {
        debug_assert!(self.active(depth).is_none());
        self.scopes.push(ListScope {
            depth,
            lists: vec![OpenList { key, list, item }],
            principal_open: true,
            attach_next: false,
            after_blank: false,
        })
    }

    /// Closes the lists at this depth of the block stack, returning the (outermost) list
    pub fn close_active(&mut self, depth: usize) -> Result<Option<Block>, BlockError> {
        if self.active(depth).is_none() {
            return Ok(None);
        }
        match self.scopes.pop() {
            Some(scope) => scope.close(),
            None => Ok(None),
        }
    }

    /// Closes every scope deeper than `depth`, deepest first, returning each list with the
    /// depth it started at
    pub fn close_deeper_than(&mut self, depth: usize) -> Result<Vec<(usize, Block)>, BlockError> {
        let mut closed = vec![];
        while let Some(scope) = self.scopes.pop_if(|scope| scope.depth > depth) {
            let scope_depth = scope.depth;
            if let Some(list) = scope.close()? {
                closed.push((scope_depth, list));
            }
        }
        Ok(closed)
    }
}

/// The key of description lists. Only `::` is recognized for now, so they don't nest in each
/// other (`;;` and `:::` would)
pub(super) const DLIST_KEY: &str = "::";

/// What a list item's marker must share with the list's other items. Numbered items (`1.`,
/// `2.`) and callouts (`<1>`, `<2>`) belong together whatever their number; every other marker
/// only matches itself, so `*` and `**`, or `.` and `1.`, are different lists.
pub(super) fn marker_key(marker: &str) -> String {
    if let Some(number) = marker.strip_suffix('.')
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
    {
        return "1.".to_string();
    }
    if let Some(number) = marker
        .strip_prefix('<')
        .and_then(|marker| marker.strip_suffix('>'))
        && !number.is_empty()
    {
        return "<1>".to_string();
    }
    marker.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::lists::{List, ListItem, ListVariant};
    use rstest::rstest;

    #[rstest]
    #[case::star("*", "*")]
    #[case::double_star("**", "**")]
    #[case::dash("-", "-")]
    #[case::dot(".", ".")]
    #[case::numbered("12.", "1.")]
    #[case::callout("<3>", "<1>")]
    #[case::dots_are_not_numbers("..", "..")]
    #[case::lower_alpha("c.", "a.")]
    #[case::upper_alpha("C.", "A.")]
    #[case::lower_roman("iv)", "i)")]
    #[case::upper_roman("IX)", "I)")]
    fn marker_keys(#[case] marker: &str, #[case] expected: &str) {
        assert_eq!(marker_key(marker), expected);
    }

    fn item(marker: &str, text: &str) -> Block {
        let mut item = ListItem::new(marker.to_string(), vec![]);
        item.add_inline(Inline::InlineLiteral(
            crate::graph::inlines::InlineLiteral::new(
                crate::graph::inlines::InlineLiteralName::Text,
                text.to_string(),
                vec![],
            ),
        ));
        Block::ListItem(item)
    }

    fn list(marker: &str) -> Block {
        Block::List(List::new(
            ListVariant::Unordered,
            marker.to_string(),
            vec![],
        ))
    }

    #[test]
    fn scopes_are_only_active_at_their_depth() {
        let mut lists = ListContext::default();
        lists.start_list(1, "*".to_string(), list("*"), item("*", "a"));
        assert!(lists.active(0).is_none());
        assert!(lists.active(2).is_none());
        assert!(lists.active(1).is_some());
    }

    #[test]
    fn closing_a_scope_returns_its_items_in_order() {
        let mut lists = ListContext::default();
        lists.start_list(0, "*".to_string(), list("*"), item("*", "a"));
        let handed_back = lists.active(0).unwrap().add_item("*", item("*", "b"));
        assert!(handed_back.unwrap().is_none());
        let Some(Block::List(list)) = lists.close_active(0).unwrap() else {
            panic!("Expected a list")
        };
        assert_eq!(list.items.len(), 2);
        assert!(lists.active(0).is_none());
    }

    #[test]
    fn an_item_with_an_unopened_marker_is_handed_back() {
        let mut lists = ListContext::default();
        lists.start_list(0, "*".to_string(), list("*"), item("*", "a"));
        let scope = lists.active(0).unwrap();
        assert!(scope.add_item("**", item("**", "b")).unwrap().is_some());
    }

    /// `* a`, `** b`, `*** c`, `* d`: the `*` item closes both nested lists into "a"
    #[test]
    fn an_open_marker_closes_the_lists_nested_deeper() {
        let mut lists = ListContext::default();
        lists.start_list(0, "*".to_string(), list("*"), item("*", "a"));
        let scope = lists.active(0).unwrap();
        scope.nest("**".to_string(), list("**"), item("**", "b"));
        scope.nest("***".to_string(), list("***"), item("***", "c"));
        assert!(scope.add_item("*", item("*", "d")).unwrap().is_none());
        assert_eq!(scope.lists.len(), 1);

        let Some(Block::List(list)) = lists.close_active(0).unwrap() else {
            panic!("Expected a list")
        };
        assert_eq!(list.items.len(), 2);
        let Block::ListItem(a) = &list.items[0] else {
            panic!("Expected a list item")
        };
        let [Block::List(b_list)] = a.blocks.as_slice() else {
            panic!("Expected one nested list in \"a\"")
        };
        let Block::ListItem(b) = &b_list.items[0] else {
            panic!("Expected a list item")
        };
        assert!(matches!(b.blocks.as_slice(), [Block::List(_)]));
    }
}
