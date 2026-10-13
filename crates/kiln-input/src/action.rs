//! Vim grammar grammar elements: operators, motions, text objects, and parsed actions.

/// Vim operator applied to a motion or text object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    /// Delete (`d`).
    Delete,
    /// Change (`c`).
    Change,
    /// Yank / copy (`y`).
    Yank,
}

/// Vim motion specifying a target destination or range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Motion {
    /// Left (`h`).
    Left,
    /// Right (`l`).
    Right,
    /// Up (`k`).
    Up,
    /// Down (`j`).
    Down,
    /// Forward to start of next word (`w`).
    WordForward,
    /// Forward to start of next WORDS (`W`).
    WordForwardBig,
    /// Backward to start of previous word (`b`).
    WordBackward,
    /// Backward to start of previous WORDS (`B`).
    WordBackwardBig,
    /// Forward to end of word (`e`).
    WordEnd,
    /// Forward to end of WORDS (`E`).
    WordEndBig,
    /// Start of current line (`0`).
    LineStart,
    /// End of current line (`$`).
    LineEnd,
    /// First non-blank character of current line (`^`).
    FirstNonBlank,
    /// Start of buffer (`gg`).
    BufferStart,
    /// End of buffer (`G`).
    BufferEnd,
    /// Find character forward on current line (`f{char}`).
    FindForward(char),
    /// Find character backward on current line (`F{char}`).
    FindBackward(char),
    /// Till character forward on current line (`t{char}`).
    TillForward(char),
    /// Till character backward on current line (`T{char}`).
    TillBackward(char),
    /// Repeat last find on line (`;`).
    RepeatFind,
    /// Repeat last find in reverse direction (`,`).
    RepeatFindReverse,
    /// Matching bracket or parenthesis (`%`).
    MatchPair,
    /// Entire current line (target for doubled operators `dd`, `cc`, `yy`).
    CurrentLine,
}

/// Vim text object (inner or around).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextObject {
    /// Inner word (`iw`).
    InnerWord,
    /// Around / a word (`aw`).
    AroundWord,
    /// Inner quote (`i"`, `i'`, `i\``).
    InnerQuote(char),
    /// Around quote (`a"`, `a'`, `a\``).
    AroundQuote(char),
    /// Inner bracket/parenthesis (`i(`, `i[`, `i{`, `i<`).
    InnerPair(char),
    /// Around bracket/parenthesis (`a(`, `a[`, `a{`, `a<`).
    AroundPair(char),
}

/// Target of an operator: either a motion or a text object.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    /// Motion target.
    Motion(Motion),
    /// Text object target.
    TextObject(TextObject),
}

/// A fully parsed Vim grammar action: `[register][count]operator[target]` or standalone motion/command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VimAction {
    /// Target register (e.g. `"a`).
    pub register: Option<char>,
    /// Count multiplier (e.g. `3`).
    pub count: Option<usize>,
    /// Operator (e.g. `d`).
    pub operator: Option<Operator>,
    /// Target motion or text object.
    pub target: Option<Target>,
}
