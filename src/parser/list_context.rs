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
    /// outermost first: each list after the first is nested in the current item of the one
    /// before it
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

    /// Adds an item to the open list with its key, closing the lists nested deeper than that one.
    /// If no open list has the key, the item is handed back, to start a list nested in the
    /// current item (see [`nest`](Self::nest)). Lists nest by marker, not by marker length: after
    /// `**`, a `*` item closes back to the `*` list only if one is open, and otherwise nests
    /// deeper.
    pub fn add_item(&mut self, key: &str, item: ListItem) -> Result<Option<ListItem>, BlockError> {
        let Some(level) = self.lists.iter().position(|open| open.key == key) else {
            return Ok(Some(item));
        };
        self.close_deeper_than(level)?;
        let open = &mut self.lists[level];
        let finished = std::mem::replace(&mut open.item, item);
        open.list.push_block(Block::ListItem(finished))?;
        self.principal_open = true;
        self.attach_next = false;
        Ok(None)
    }

    /// Starts a list, with its first item, nested in the current item
    pub fn nest(&mut self, key: String, list: List, item: ListItem) {
        self.lists.push(OpenList {
            key,
            list: Block::List(list),
            item,
        });
        self.principal_open = true;
        self.attach_next = false;
    }

    /// Closes the lists nested deeper than `level`, innermost first, each into the current item
    /// of the list it is nested in
    fn close_deeper_than(&mut self, level: usize) -> Result<(), BlockError> {
        while self.lists.len() > level + 1 {
            let mut open = self.lists.pop().expect("Checked by the loop condition");
            open.list.push_block(Block::ListItem(open.item))?;
            self.lists
                .last_mut()
                .expect("Checked by the loop condition")
                .item
                .blocks
                .push(open.list);
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
        list.push_block(Block::ListItem(open.item))?;
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
        let list = List::new(ListVariant::Unordered, "*".to_string(), vec![]);
        lists.start_list(0, "*".to_string(), list, item("*", "a"));
        let scope = lists.active(0).unwrap();
        assert!(scope.add_item("**", item("**", "b")).unwrap().is_some());
    }

    /// `* a`, `** b`, `*** c`, `* d`: the `*` item closes both nested lists into "a"
    #[test]
    fn an_open_marker_closes_the_lists_nested_deeper() {
        let mut lists = ListContext::default();
        let new_list = |marker: &str| List::new(ListVariant::Unordered, marker.to_string(), vec![]);
        lists.start_list(0, "*".to_string(), new_list("*"), item("*", "a"));
        let scope = lists.active(0).unwrap();
        scope.nest("**".to_string(), new_list("**"), item("**", "b"));
        scope.nest("***".to_string(), new_list("***"), item("***", "c"));
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
