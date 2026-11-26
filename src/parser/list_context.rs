//! Keeps track of the lists that are open while the [`Parser`](super::Parser) works.
//!
//! Lists are held here, and not on the parser's block stack, until they are complete. An item's
//! text and the blocks attached to it (with a `+`) then go to that item by name, rather than to
//! whatever block happens to be on top of the block stack by the time the item ends.
//!
//! Lists live in scopes. A scope belongs to one level of the block stack: the lists at the top
//! level of a section, of a delimited block, or of a description list item. A scope is active
//! only while the block stack is at that depth; a delimited block attached to an item pushes the
//! stack deeper, which suspends the scope until that block is complete. The parser must close a
//! scope before the block stack shrinks below its depth.

use crate::graph::{
    blocks::Block,
    inlines::Inline,
    lists::{List, ListItem},
};

use crate::errors::BlockError;

/// A list that is still open, and the item currently being added to it
#[derive(Debug)]
struct OpenList {
    /// what items must have in common to belong to this list (see [`marker_key`])
    key: String,
    /// a `Block::List`, holding the items before the current one
    list: Block,
    /// the item being added to, which joins the list when the next item starts or the list ends
    item: ListItem,
}

/// The lists open at one level of the block stack
#[derive(Debug)]
pub(super) struct ListScope {
    /// the length of the parser's block stack when the list started
    depth: usize,
    /// outermost first. For now there is only ever one; nested lists will add more
    lists: Vec<OpenList>,
    /// the current item still takes lines of text: its principal text runs until the next item,
    /// a `+`, a blank line or the start of another block
    pub principal_open: bool,
    /// a `+` was seen, so the next block belongs to the current item
    pub attach_next: bool,
}

impl ListScope {
    fn current_item(&mut self) -> &mut ListItem {
        // a scope is only created with a list in it, and is dropped when it closes
        &mut self
            .lists
            .last_mut()
            .expect("A list scope always has an open list")
            .item
    }

    /// Adds an inline to the current item's principal text
    pub fn push_principal(&mut self, inline: Inline) {
        self.current_item().add_inline(inline)
    }

    /// Attaches a block to the current item (a block that followed a `+`)
    pub fn attach_block(&mut self, block: Block) {
        self.current_item().blocks.push(block);
        self.attach_next = false;
    }

    /// The key of the innermost open list
    pub fn key(&self) -> Option<&str> {
        self.lists.last().map(|open| open.key.as_str())
    }

    /// Starts a new item in the innermost list, adding the current item to that list
    pub fn add_item(&mut self, item: ListItem) -> Result<(), BlockError> {
        let open = self
            .lists
            .last_mut()
            .expect("A list scope always has an open list");
        let finished = std::mem::replace(&mut open.item, item);
        open.list.push_block(Block::ListItem(finished))?;
        self.principal_open = true;
        self.attach_next = false;
        Ok(())
    }

    /// Folds the open lists, innermost first, into the outermost one, and returns it
    fn close(mut self) -> Result<Option<Block>, BlockError> {
        let mut finished: Option<Block> = None;
        while let Some(mut open) = self.lists.pop() {
            if let Some(child_list) = finished.take() {
                open.item.blocks.push(child_list);
            }
            open.list.push_block(Block::ListItem(open.item))?;
            finished = Some(open.list);
        }
        Ok(finished)
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

    /// Starts a new list (with its first item) at this depth of the block stack. The caller
    /// closes any list already open at this depth first.
    pub fn start_list(&mut self, depth: usize, key: String, list: List, item: ListItem) {
        debug_assert!(self.active(depth).is_none());
        self.scopes.push(ListScope {
            depth,
            lists: vec![OpenList {
                key,
                list: Block::List(list),
                item,
            }],
            principal_open: true,
            attach_next: false,
        })
    }

    /// Closes the lists at this depth of the block stack, returning the (outermost) list
    pub fn close_active(&mut self, depth: usize) -> Result<Option<Block>, BlockError> {
        match self.active(depth) {
            Some(_) => self.scopes.pop().unwrap().close(),
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
    use crate::graph::lists::ListVariant;
    use rstest::rstest;

    #[rstest]
    #[case::star("*", "*")]
    #[case::double_star("**", "**")]
    #[case::dash("-", "-")]
    #[case::dot(".", ".")]
    #[case::numbered("12.", "1.")]
    #[case::callout("<3>", "<1>")]
    #[case::dots_are_not_numbers("..", "..")]
    fn marker_keys(#[case] marker: &str, #[case] expected: &str) {
        assert_eq!(marker_key(marker), expected);
    }

    fn item(marker: &str, text: &str) -> ListItem {
        let mut item = ListItem::new(marker.to_string(), vec![]);
        item.add_inline(Inline::InlineLiteral(
            crate::graph::inlines::InlineLiteral::new(
                crate::graph::inlines::InlineLiteralName::Text,
                text.to_string(),
                vec![],
            ),
        ));
        item
    }

    #[test]
    fn scopes_are_only_active_at_their_depth() {
        let mut lists = ListContext::default();
        let list = List::new(ListVariant::Unordered, "*".to_string(), vec![]);
        lists.start_list(1, "*".to_string(), list, item("*", "a"));
        assert!(lists.active(0).is_none());
        assert!(lists.active(2).is_none());
        assert!(lists.active(1).is_some());
    }

    #[test]
    fn closing_a_scope_returns_its_items_in_order() {
        let mut lists = ListContext::default();
        let list = List::new(ListVariant::Unordered, "*".to_string(), vec![]);
        lists.start_list(0, "*".to_string(), list, item("*", "a"));
        lists.active(0).unwrap().add_item(item("*", "b")).unwrap();
        let Some(Block::List(list)) = lists.close_active(0).unwrap() else {
            panic!("Expected a list")
        };
        assert_eq!(list.items.len(), 2);
        assert!(lists.active(0).is_none());
    }
}
