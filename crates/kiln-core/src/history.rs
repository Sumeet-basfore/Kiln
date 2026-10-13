//! Undo tree history data structure.
//!
//! A tree (not a stack) of editing transactions.
//! Provides branching undo, preferred redo navigation, sequence numbers,
//! and common ancestor jump navigation.

use crate::error::CoreError;
use crate::transaction::Transaction;

/// An opaque identifier for a node in the `UndoTree`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// A single node in the `UndoTree`.
#[derive(Debug, Clone)]
pub struct HistoryNode {
    id: NodeId,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    preferred_child: Option<NodeId>,
    forward: Transaction,
    inverse: Transaction,
    seq: u64,
}

impl HistoryNode {
    /// Returns the node's unique ID.
    #[must_use]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns the parent node ID, if any.
    #[must_use]
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Returns a slice of child node IDs.
    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Returns the sequence number of this node.
    #[must_use]
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Returns the forward transaction.
    #[must_use]
    pub fn forward(&self) -> &Transaction {
        &self.forward
    }

    /// Returns the inverse transaction.
    #[must_use]
    pub fn inverse(&self) -> &Transaction {
        &self.inverse
    }
}

/// A tree of editing history allowing non-linear undo and redo.
#[derive(Debug, Clone, Default)]
pub struct UndoTree {
    nodes: Vec<HistoryNode>,
    current: Option<NodeId>,
    next_seq: u64,
}

impl UndoTree {
    /// Creates an empty `UndoTree`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            current: None,
            next_seq: 1,
        }
    }

    /// Returns the current active node ID, or `None` if at the root/initial state.
    #[must_use]
    pub fn current(&self) -> Option<NodeId> {
        self.current
    }

    /// Returns a reference to a node by its ID.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&HistoryNode> {
        self.nodes.get(id.0)
    }

    /// Returns a node's sequence number by its ID.
    #[must_use]
    pub fn get_seq(&self, id: NodeId) -> Option<u64> {
        self.nodes.get(id.0).map(|n| n.seq)
    }

    /// Commits a forward and inverse transaction as a new history node.
    ///
    /// Attaches the new node as a child of the current node, sets it as preferred
    /// child of the parent, and advances current to it.
    pub fn commit(&mut self, forward: Transaction, inverse: Transaction) -> NodeId {
        let new_id = NodeId(self.nodes.len());
        let seq = self.next_seq;
        self.next_seq += 1;

        let node = HistoryNode {
            id: new_id,
            parent: self.current,
            children: Vec::new(),
            preferred_child: None,
            forward,
            inverse,
            seq,
        };

        if let Some(parent_id) = self.current {
            if let Some(parent_node) = self.nodes.get_mut(parent_id.0) {
                parent_node.children.push(new_id);
                parent_node.preferred_child = Some(new_id);
            }
        }

        self.nodes.push(node);
        self.current = Some(new_id);
        new_id
    }

    /// Undoes one step up the tree to the parent node.
    ///
    /// Returns a reference to the inverse transaction to apply, and updates
    /// the parent's preferred child to the undone node so redo can retrace it.
    pub fn undo(&mut self) -> Option<&Transaction> {
        let cur_id = self.current?;
        let parent_id = self.nodes[cur_id.0].parent;

        if let Some(p_id) = parent_id {
            self.nodes[p_id.0].preferred_child = Some(cur_id);
        }

        self.current = parent_id;
        // Return reference to the inverse transaction stored on cur_id
        Some(&self.nodes[cur_id.0].inverse)
    }

    /// Redoes one step down the tree following the preferred child.
    ///
    /// Returns a reference to the forward transaction to apply.
    pub fn redo(&mut self) -> Option<&Transaction> {
        let next_child = match self.current {
            Some(cur_id) => {
                let cur_node = &self.nodes[cur_id.0];
                cur_node
                    .preferred_child
                    .or_else(|| cur_node.children.first().copied())
            }
            None => {
                // If at root, find a root node (parent == None).
                // Redo preference at root: the most recently committed or first root node.
                self.nodes
                    .iter()
                    .rfind(|n| n.parent.is_none())
                    .map(|n| n.id)
            }
        }?;

        self.current = Some(next_child);
        Some(&self.nodes[next_child.0].forward)
    }

    /// Returns the path of transactions to navigate from `current` to `target`.
    ///
    /// Calculates the common ancestor between current and target:
    /// - Collects inverse transactions moving up from `current` to common ancestor.
    /// - Collects forward transactions moving down from common ancestor to `target`.
    /// - Updates `current` to `target`.
    ///
    /// # Errors
    /// Returns `CoreError::HistorySeqNotFound` if target node does not exist.
    pub fn jump_to(&mut self, target: Option<NodeId>) -> Result<Vec<Transaction>, CoreError> {
        if self.current == target {
            return Ok(Vec::new());
        }

        if let Some(t_id) = target {
            if t_id.0 >= self.nodes.len() {
                return Err(CoreError::HistorySeqNotFound { seq: 0 });
            }
        }

        // Trace path from current to root
        let mut from_path = Vec::new();
        let mut curr = self.current;
        while let Some(id) = curr {
            from_path.push(id);
            curr = self.nodes[id.0].parent;
        }

        // Trace path from target to root
        let mut to_path = Vec::new();
        let mut curr_t = target;
        while let Some(id) = curr_t {
            to_path.push(id);
            curr_t = self.nodes[id.0].parent;
        }

        // Find lowest common ancestor
        let mut lca: Option<NodeId> = None;
        for id in &from_path {
            if to_path.contains(id) {
                lca = Some(*id);
                break;
            }
        }

        let mut txs = Vec::new();

        // 1. Undo up to LCA
        let mut walk = self.current;
        while walk != lca {
            if let Some(id) = walk {
                txs.push(self.nodes[id.0].inverse.clone());
                let parent = self.nodes[id.0].parent;
                if let Some(p) = parent {
                    self.nodes[p.0].preferred_child = Some(id);
                }
                walk = parent;
            } else {
                break;
            }
        }

        // 2. Redo down from LCA to target
        let mut down_nodes = Vec::new();
        let mut walk_t = target;
        while walk_t != lca {
            if let Some(id) = walk_t {
                down_nodes.push(id);
                walk_t = self.nodes[id.0].parent;
            } else {
                break;
            }
        }
        down_nodes.reverse();

        for id in down_nodes {
            txs.push(self.nodes[id.0].forward.clone());
            if let Some(p) = self.nodes[id.0].parent {
                self.nodes[p.0].preferred_child = Some(id);
            }
        }

        self.current = target;
        Ok(txs)
    }

    /// Jumps to the node with the given sequence number.
    ///
    /// # Errors
    /// Returns `CoreError::HistorySeqNotFound` if sequence number is not found.
    pub fn jump_to_seq(&mut self, seq: u64) -> Result<Vec<Transaction>, CoreError> {
        let node_id = self
            .nodes
            .iter()
            .find(|n| n.seq == seq)
            .map(|n| n.id)
            .ok_or(CoreError::HistorySeqNotFound { seq })?;
        self.jump_to(Some(node_id))
    }

    /// Returns a list of all branch leaf nodes (nodes with no children).
    #[must_use]
    pub fn branch_heads(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|n| n.children.is_empty())
            .map(|n| n.id)
            .collect()
    }

    /// Estimates the memory usage of the undo tree in bytes.
    #[must_use]
    pub fn approx_bytes(&self) -> usize {
        let mut bytes = std::mem::size_of::<Self>()
            + self.nodes.capacity() * std::mem::size_of::<HistoryNode>();
        for node in &self.nodes {
            for op in node.forward.changes.ops() {
                if let crate::transaction::Operation::Insert(ref s) = op {
                    bytes += s.len();
                }
            }
            for op in node.inverse.changes.ops() {
                if let crate::transaction::Operation::Insert(ref s) = op {
                    bytes += s.len();
                }
            }
        }
        bytes
    }
}
