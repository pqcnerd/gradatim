//! Structured hint extraction from English instructions.
//!
//! This module parses English/pseudocode and extracts structured hints
//! that help the AI generate more accurate code. The hints describe
//! the *intent* of the instruction without being language-specific.

use serde::Serialize;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::models::TranslateLineRequest;
use crate::func_matcher::{FunctionMatch, match_function_call};
use crate::stdlib_db::FunctionDatabase;

/// A structured hint extracted from an English instruction.
/// These hints are language-agnostic and describe programmer intent.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatementHint {
    /// Variable declaration: "declare x", "create an integer called count"
    Declaration {
        names: Vec<String>,
        type_hint: Option<String>,
        qualifiers: Vec<String>,
        initial_value: Option<String>,
        is_array: bool,
        array_size: Option<String>,
    },

    /// Assignment: "set x to 5", "set a, b, c 0"
    /// Can handle multiple targets with the same value
    Assignment {
        targets: Vec<String>,
        value: String,
    },

    /// Loop construct: "loop from 0 to 10", "for i 0 to n"
    Loop {
        iterator: Option<String>,
        start: Option<String>,
        end: Option<String>,
        collection: Option<String>,
        body_action: Option<String>,
    },

    /// While loop: "while i < n do ..."
    While {
        condition: String,
        body_action: Option<String>,
    },

    /// Conditional: "if x > 5", "when count equals zero"
    Conditional {
        condition: String,
        then_action: Option<String>,
        else_action: Option<String>,
        is_else_if: bool,
    },

    /// Else branch: "else", "otherwise"
    Else,

    /// End block: "end", "end if", "end loop"
    EndBlock,

    /// Print/output: "print hello", "output the result"
    Print {
        content: String,
        is_literal: bool,
    },

    /// Read input: "read n", "input x"
    Read {
        variables: Vec<String>,
    },

    /// Return statement: "return x", "return 0"
    Return {
        value: Option<String>,
    },

    /// Arithmetic operation: "add a and b", "multiply x by 2"
    Arithmetic {
        operation: ArithmeticOp,
        left: String,
        right: String,
        target: Option<String>,
    },

    /// Increment/decrement: "increment x", "decrement counter"
    Modify {
        target: String,
        delta: i32, // +1 or -1
    },

    /// Pre/post increment/decrement: "pre increment x", "post decrement i"
    PrePostModify {
        target: String,
        delta: i32, // +1 or -1
        position: IncDecPosition,
    },

    /// Function definition: "function foo taking int x"
    FunctionDef {
        name: String,
        parameters: Vec<(String, String)>, // (type, name)
        return_type: Option<String>,
    },

    /// Struct definition: "struct Point with x and y"
    StructDef {
        name: String,
        fields: Vec<(String, String)>, // (type, name)
    },

    /// Bitfield declaration: "flag 1 bit unsigned"
    BitfieldDecl {
        name: String,
        type_hint: Option<String>,
        width: String,
    },

    /// Main entry point: "create main", "entry point"
    MainFunction,

    /// Switch statement: "switch a"
    Switch {
        expression: String,
    },

    /// Case in switch: "case 1", "case 2 do X"
    Case {
        value: String,
        action: Option<String>,
    },

    /// Default case: "default", "default do X"
    Default {
        action: Option<String>,
    },

    /// Do-while loop start: "do"
    DoWhileStart,

    /// Do-while loop end: "end do while condition"
    DoWhileEnd {
        condition: String,
    },

    /// Break statement: "break", "break out"
    Break,

    /// Continue statement: "continue", "skip iteration"
    Continue,

    /// Function call: "call foo with x, y"
    FunctionCall {
        name: String,
        arguments: Vec<String>,
    },

    /// Include directive: "include stdio"
    Include {
        header: String,
        is_system: bool,
    },

    /// Define macro: "define MAX 100"
    Define {
        name: String,
        value: String,
    },

    /// Pointer declaration: "pointer to int x"
    PointerDecl {
        base_type: String,
        name: String,
    },

    /// Dereference: "dereference p"
    Dereference {
        target: String,
    },

    /// Address of: "address of x"
    AddressOf {
        target: String,
    },

    /// Malloc: "allocate n integers"
    Malloc {
        count: String,
        element_type: String,
    },

    /// Free: "free p"
    Free {
        target: String,
    },

    /// Enum definition: "enum Color with red, green, blue"
    EnumDef {
        name: String,
        values: Vec<String>,
    },

    /// Typedef: "typedef int Number"
    Typedef {
        original_type: String,
        new_name: String,
    },

    /// String declaration: "string s equals hello"
    StringDecl {
        name: String,
        initial_value: Option<String>,
        size: Option<String>,
    },

    /// Comment: "comment this does X"
    Comment {
        text: String,
        is_block: bool,
    },

    /// Bitwise operation: "x bitwise and y"
    Bitwise {
        operation: BitwiseOp,
        left: String,
        right: Option<String>,
        target: Option<String>,
    },

    /// Type cast: "cast x to int"
    Cast {
        expression: String,
        target_type: String,
    },

    /// Array access: "get a at i", "set a at 0 to 5"
    ArrayAccess {
        array: String,
        index: String,
        value: Option<String>, // None for read, Some for write
    },

    /// Sizeof: "size of a"
    SizeOf {
        target: String,
    },

    /// Ternary: "if x then y else z" (inline)
    Ternary {
        condition: String,
        true_value: String,
        false_value: String,
    },

    /// Compound assignment: "add 5 to x", "multiply x by 2"
    CompoundAssign {
        target: String,
        operator: CompoundOp,
        value: String,
    },

    /// Logical operation: "a and b", "a or b", "not a"
    Logical {
        operation: LogicalOp,
        left: String,
        right: Option<String>,
        target: Option<String>,
    },

    /// Preprocessor controls: #ifdef/#ifndef/#endif/#undef/#pragma
    IfDef { symbol: String },
    IfNDef { symbol: String },
    EndIf,
    Undef { symbol: String },
    Pragma { value: String },
    MacroFunction {
        name: String,
        params: Vec<String>,
        body: String,
    },

    /// Memory helpers: realloc/calloc/pointer math/null
    Realloc {
        pointer: String,
        count: String,
        element_type: String,
    },
    Calloc {
        count: String,
        element_type: String,
        target: Option<String>,
    },
    PointerArithmetic {
        pointer: String,
        offset: String,
        direction: PointerDir,
    },
    FunctionPointer {
        return_type: String,
        name: String,
        params: Vec<String>,
    },
    DoublePointer {
        base_type: String,
        name: String,
    },
    NullAssign {
        target: String,
    },

    /// Multidimensional arrays and initialization
    MultiArrayDecl {
        type_hint: Option<String>,
        name: String,
        dimensions: Vec<String>,
    },
    ArrayInit {
        type_hint: Option<String>,
        name: String,
        values: Vec<String>,
    },
    DesignatedInit {
        type_hint: Option<String>,
        name: String,
        designators: Vec<(String, String)>,
    },
    CompoundLiteral {
        type_hint: String,
        values: Vec<String>,
        fields: Vec<(String, String)>,
        is_array: bool,
    },
    MultiDimAccess {
        array: String,
        indices: Vec<String>,
        value: Option<String>,
    },

    /// Struct/union helpers
    StructAccess {
        object: String,
        field: String,
    },
    StructArrow {
        pointer: String,
        field: String,
    },
    StructInit {
        struct_name: String,
        var_name: String,
        fields: Vec<(String, String)>,
    },
    AnonymousStruct {
        parent: Option<String>,
        fields: Vec<(String, String)>,
    },
    UnionDef {
        name: String,
        fields: Vec<(String, String)>,
    },
    AnonymousUnion {
        parent: Option<String>,
        fields: Vec<(String, String)>,
    },
    StructArray {
        struct_name: String,
        var_name: String,
        size: String,
    },

    /// Additional control flow
    Goto { label: String },
    Label { name: String },
    InfiniteLoop,
    ForEver,

    /// Function qualifiers / prototypes
    FunctionPrototype {
        name: String,
        parameters: Vec<(String, String)>,
        return_type: Option<String>,
    },
    QualifiedFunction {
        qualifier: String,
        name: String,
        parameters: Vec<(String, String)>,
        return_type: Option<String>,
    },

    /// File IO helpers
    FileOpen {
        var_name: String,
        path: String,
        mode: String,
    },
    FileClose {
        var_name: String,
    },
    FileRead {
        var_name: String,
        buffer: String,
        size: String,
    },
    FileWrite {
        var_name: String,
        buffer: String,
        size: String,
    },
    Fgets {
        buffer: String,
        size: String,
        var_name: String,
    },
    Fputs {
        content: String,
        var_name: String,
    },
    Fprintf {
        var_name: String,
        format: String,
        args: Vec<String>,
    },
    Fscanf {
        var_name: String,
        args: Vec<String>,
    },

    /// String helpers
    Strcpy { dest: String, src: String },
    Strncpy { dest: String, src: String, count: String },
    Strcat { dest: String, src: String },
    Strcmp { left: String, right: String, target: Option<String> },
    Strlen { target: String, store_in: Option<String> },
    Sprintf { buffer: String, format: String, args: Vec<String> },

    /// Stdlib helpers
    Memcpy { dest: String, src: String, size: String },
    Memset { dest: String, value: String, size: String },
    Exit { code: String },
    Rand { store_in: Option<String> },
    MathFunc {
        func: MathFuncKind,
        args: Vec<String>,
        store_in: Option<String>,
    },

    /// Error handling
    Assert { expression: String },
    Perror { message: Option<String> },
    ErrnoCheck,

    /// Extended error handling
    TryBlock,
    ExceptBlock { exception_type: Option<String>, variable: Option<String> },
    FinallyBlock,
    RaiseException { exception_type: String, message: Option<String> },
    ErrorCheck { function_call: String, error_variable: Option<String> },
    SetJmp { buffer: String },
    LongJmp { buffer: String, value: String },

    /// Testing
    TestFunction { name: String, description: Option<String> },
    TestAssert { expression: String, message: Option<String> },
    TestAssertEqual { left: String, right: String, message: Option<String> },
    TestAssertNotEqual { left: String, right: String },
    TestAssertTrue { expression: String },
    TestAssertFalse { expression: String },
    TestSetup { name: String },
    TestTeardown { name: String },
    MockFunction { name: String, return_value: String },

    /// Advanced control flow
    LabeledBreak { label: String },
    LabeledContinue { label: String },
    LabeledLoop { label: String, loop_hint: Box<StatementHint> },
    MatchBlock { expression: String },
    MatchCase { pattern: String, guard: Option<String>, action: Option<String> },
    MatchWildcard { action: Option<String> },
    GuardClause { condition: String, action: String },
    ConditionalChain { conditions: Vec<(String, String)>, else_action: Option<String> },

    /// Data structures
    LinkedListCreate { name: String, element_type: Option<String> },
    LinkedListInsert { list: String, value: String, position: Option<String> },
    LinkedListRemove { list: String, position: String },
    LinkedListTraverse { list: String, iterator: String },
    StackCreate { name: String, element_type: Option<String> },
    StackPush { stack: String, value: String },
    StackPop { stack: String, target: Option<String> },
    StackPeek { stack: String, target: Option<String> },
    StackIsEmpty { stack: String },
    QueueCreate { name: String, element_type: Option<String> },
    QueueEnqueue { queue: String, value: String },
    QueueDequeue { queue: String, target: Option<String> },
    MapCreate { name: String, key_type: Option<String>, value_type: Option<String> },
    MapPut { map: String, key: String, value: String },
    MapGet { map: String, key: String, target: Option<String> },
    MapRemove { map: String, key: String },
    MapContainsKey { map: String, key: String },
    MapKeys { map: String, target: Option<String> },
    MapValues { map: String, target: Option<String> },
    SetCreate { name: String, element_type: Option<String> },
    SetAdd { set: String, value: String },
    SetRemove { set: String, value: String },
    SetContains { set: String, value: String },
    SetUnion { set1: String, set2: String, target: String },
    SetIntersection { set1: String, set2: String, target: String },
    TreeNode { name: String, value: String, left: Option<String>, right: Option<String> },
    TreeInsert { tree: String, value: String },
    TreeSearch { tree: String, value: String },
    TreeTraverse { tree: String, order: String },

    /// OOP constructs
    ClassDef { name: String, parent: Option<String>, interfaces: Vec<String>, is_abstract: bool },
    ClassField { name: String, type_hint: Option<String>, visibility: Visibility, is_static: bool, initial_value: Option<String> },
    ClassMethod { name: String, parameters: Vec<(String, String)>, return_type: Option<String>, visibility: Visibility, is_static: bool, is_abstract: bool, is_virtual: bool },
    Constructor { parameters: Vec<(String, String)>, body: Option<String> },
    Destructor { body: Option<String> },
    InterfaceDef { name: String, extends: Vec<String> },
    InterfaceMethod { name: String, parameters: Vec<(String, String)>, return_type: Option<String> },
    ObjectCreate { class_name: String, variable: String, arguments: Vec<String> },
    MethodCall { object: String, method: String, arguments: Vec<String> },
    PropertyAccess { object: String, property: String },
    PropertyAssign { object: String, property: String, value: String },
    SuperCall { method: Option<String>, arguments: Vec<String> },
    ThisReference,

    /// Lambdas and higher-order functions
    Lambda { parameters: Vec<String>, body: String, captures: Vec<String> },
    HigherOrderFunction { function: String, callback: String, collection: Option<String> },
    MapFunction { collection: String, transform: String, target: Option<String> },
    FilterFunction { collection: String, predicate: String, target: Option<String> },
    ReduceFunction { collection: String, reducer: String, initial: Option<String>, target: Option<String> },
    ForEachFunction { collection: String, action: String },

    /// Concurrency primitives
    ThreadCreate { name: Option<String>, function: String, arguments: Vec<String> },
    ThreadJoin { thread: String },
    ThreadDetach { thread: String },
    ThreadSleep { duration: String, unit: String },
    MutexCreate { name: String },
    MutexLock { mutex: String },
    MutexUnlock { mutex: String },
    MutexTryLock { mutex: String },
    SemaphoreCreate { name: String, initial: String },
    SemaphoreWait { semaphore: String },
    SemaphoreSignal { semaphore: String },
    ConditionCreate { name: String },
    ConditionWait { condition: String, mutex: String },
    ConditionSignal { condition: String },
    ConditionBroadcast { condition: String },
    AtomicCreate { name: String, initial: String },
    AtomicLoad { atomic: String, target: Option<String> },
    AtomicStore { atomic: String, value: String },
    AtomicCompareExchange { atomic: String, expected: String, desired: String },
    AtomicIncrement { atomic: String },
    AtomicDecrement { atomic: String },

    /// Async/await patterns
    AsyncFunction { name: String, parameters: Vec<(String, String)>, return_type: Option<String> },
    AwaitExpression { expression: String, target: Option<String> },
    PromiseCreate { name: String, executor: String },
    PromiseThen { promise: String, handler: String },
    PromiseCatch { promise: String, handler: String },
    PromiseAll { promises: Vec<String>, target: String },
    PromiseRace { promises: Vec<String>, target: String },

    /// Documentation
    DocComment { text: String, doc_type: DocType },
    DocFunction { brief: String, params: Vec<(String, String)>, returns: Option<String>, throws: Vec<String>, examples: Vec<String> },
    DocClass { brief: String, detailed: Option<String>, author: Option<String>, version: Option<String> },

    /// Standard library call resolved via include + function registry
    StdLibCall {
        name: String,
        args: Vec<String>,
    },

    /// Static assertion: "static assert x == y"
    StaticAssert {
        condition: String,
        message: Option<String>,
    },

    /// Comma operator expression sequence
    CommaExpression {
        expressions: Vec<String>,
    },

    /// Unknown - AI handles without specific hints
    Unknown {
        original: String,
    },
}

/// Bitwise operation types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BitwiseOp {
    And,
    Or,
    Xor,
    Not,
    ShiftLeft,
    ShiftRight,
}

/// Arithmetic operation types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

/// Pre/post increment/decrement position.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncDecPosition {
    Pre,
    Post,
}

/// Compound assignment types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompoundOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    ShlAssign,
    ShrAssign,
    AndAssign,
    OrAssign,
    XorAssign,
}

/// Logical operation types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

/// Pointer arithmetic direction.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PointerDir {
    Forward,
    Backward,
}

/// Math helper functions.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MathFuncKind {
    Sqrt,
    Pow,
    Abs,
    Sin,
    Cos,
    Tan,
    Exp,
    Log,
}

/// Visibility modifiers.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

/// Documentation comment types.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    Brief,
    Detailed,
    Param,
    Return,
    Throws,
}

impl StatementHint {
    /// Returns true if this hint represents a trivial/simple construct
    /// that can be handled without AI.
    pub fn is_trivial(&self) -> bool {
        match self {
            // Simple declarations without arrays
            StatementHint::Declaration { is_array: false, .. } => true,
            // Basic operations
            StatementHint::Assignment { .. } => true,
            StatementHint::Modify { .. } => true,
            StatementHint::PrePostModify { .. } => true,
            StatementHint::Return { .. } => true,
            StatementHint::Print { .. } => true,
            StatementHint::Arithmetic { .. } => true,
            StatementHint::Read { .. } => true,
            // Control flow markers
            StatementHint::Else => true,
            StatementHint::EndBlock => true,
            StatementHint::MainFunction => true,
            // Simple conditionals (rule-based generates complete blocks)
            StatementHint::Conditional { .. } => true,
            // While loops
            StatementHint::While { .. } => true,
            // Simple loops (rule-based generates complete blocks with braces)
            StatementHint::Loop { .. } => true,
            // New control flow
            StatementHint::Switch { .. } => true,
            StatementHint::Case { .. } => true,
            StatementHint::Default { .. } => true,
            StatementHint::DoWhileStart => true,
            StatementHint::DoWhileEnd { .. } => true,
            StatementHint::Break => true,
            StatementHint::Continue => true,
            // Function calls
            StatementHint::FunctionCall { .. } => true,
            // Preprocessor
            StatementHint::Include { .. } => true,
            StatementHint::Define { .. } => true,
            // Pointers and memory
            StatementHint::PointerDecl { .. } => true,
            StatementHint::Dereference { .. } => true,
            StatementHint::AddressOf { .. } => true,
            StatementHint::Malloc { .. } => true,
            StatementHint::Free { .. } => true,
            // Types
            StatementHint::EnumDef { .. } => true,
            StatementHint::Typedef { .. } => true,
            StatementHint::StringDecl { .. } => true,
            StatementHint::BitfieldDecl { .. } => true,
            // Comments
            StatementHint::Comment { .. } => true,
            // Operations
            StatementHint::Bitwise { .. } => true,
            StatementHint::Cast { .. } => true,
            StatementHint::ArrayAccess { .. } => true,
            StatementHint::SizeOf { .. } => true,
            StatementHint::Ternary { .. } => true,
            // New helpers
            StatementHint::CompoundAssign { .. } => true,
            StatementHint::Logical { .. } => true,
            StatementHint::IfDef { .. } => true,
            StatementHint::IfNDef { .. } => true,
            StatementHint::EndIf => true,
            StatementHint::Undef { .. } => true,
            StatementHint::Pragma { .. } => true,
            StatementHint::MacroFunction { .. } => true,
            StatementHint::Realloc { .. } => true,
            StatementHint::Calloc { .. } => true,
            StatementHint::PointerArithmetic { .. } => true,
            StatementHint::FunctionPointer { .. } => true,
            StatementHint::DoublePointer { .. } => true,
            StatementHint::NullAssign { .. } => true,
            StatementHint::MultiArrayDecl { .. } => true,
            StatementHint::ArrayInit { .. } => true,
            StatementHint::DesignatedInit { .. } => true,
            StatementHint::CompoundLiteral { .. } => true,
            StatementHint::MultiDimAccess { .. } => true,
            StatementHint::StructAccess { .. } => true,
            StatementHint::StructArrow { .. } => true,
            StatementHint::StructInit { .. } => true,
            StatementHint::AnonymousStruct { .. } => true,
            StatementHint::UnionDef { .. } => true,
            StatementHint::AnonymousUnion { .. } => true,
            StatementHint::StructArray { .. } => true,
            StatementHint::Goto { .. } => true,
            StatementHint::Label { .. } => true,
            StatementHint::InfiniteLoop => true,
            StatementHint::ForEver => true,
            StatementHint::FunctionPrototype { .. } => true,
            StatementHint::QualifiedFunction { .. } => true,
            StatementHint::FileOpen { .. } => true,
            StatementHint::FileClose { .. } => true,
            StatementHint::FileRead { .. } => true,
            StatementHint::FileWrite { .. } => true,
            StatementHint::Fgets { .. } => true,
            StatementHint::Fputs { .. } => true,
            StatementHint::Fprintf { .. } => true,
            StatementHint::Fscanf { .. } => true,
            StatementHint::Strcpy { .. } => true,
            StatementHint::Strncpy { .. } => true,
            StatementHint::Strcat { .. } => true,
            StatementHint::Strcmp { .. } => true,
            StatementHint::Strlen { .. } => true,
            StatementHint::Sprintf { .. } => true,
            StatementHint::Memcpy { .. } => true,
            StatementHint::Memset { .. } => true,
            StatementHint::Exit { .. } => true,
            StatementHint::Rand { .. } => true,
            StatementHint::MathFunc { .. } => true,
            StatementHint::Assert { .. } => true,
            StatementHint::Perror { .. } => true,
            StatementHint::ErrnoCheck => true,
            StatementHint::StdLibCall { .. } => true,
            StatementHint::StaticAssert { .. } => true,
            StatementHint::CommaExpression { .. } => true,
            StatementHint::TryBlock => true,
            StatementHint::ExceptBlock { .. } => true,
            StatementHint::FinallyBlock => true,
            StatementHint::RaiseException { .. } => true,
            StatementHint::ErrorCheck { .. } => true,
            StatementHint::SetJmp { .. } => true,
            StatementHint::LongJmp { .. } => true,
            StatementHint::TestFunction { .. } => true,
            StatementHint::TestAssert { .. } => true,
            StatementHint::TestAssertEqual { .. } => true,
            StatementHint::TestAssertNotEqual { .. } => true,
            StatementHint::TestAssertTrue { .. } => true,
            StatementHint::TestAssertFalse { .. } => true,
            StatementHint::TestSetup { .. } => true,
            StatementHint::TestTeardown { .. } => true,
            StatementHint::MockFunction { .. } => true,
            StatementHint::LabeledBreak { .. } => true,
            StatementHint::LabeledContinue { .. } => true,
            StatementHint::LabeledLoop { .. } => true,
            StatementHint::MatchBlock { .. } => true,
            StatementHint::MatchCase { .. } => true,
            StatementHint::MatchWildcard { .. } => true,
            StatementHint::GuardClause { .. } => true,
            StatementHint::ConditionalChain { .. } => true,
            StatementHint::LinkedListCreate { .. } => true,
            StatementHint::LinkedListInsert { .. } => true,
            StatementHint::LinkedListRemove { .. } => true,
            StatementHint::LinkedListTraverse { .. } => true,
            StatementHint::StackCreate { .. } => true,
            StatementHint::StackPush { .. } => true,
            StatementHint::StackPop { .. } => true,
            StatementHint::StackPeek { .. } => true,
            StatementHint::StackIsEmpty { .. } => true,
            StatementHint::QueueCreate { .. } => true,
            StatementHint::QueueEnqueue { .. } => true,
            StatementHint::QueueDequeue { .. } => true,
            StatementHint::MapCreate { .. } => true,
            StatementHint::MapPut { .. } => true,
            StatementHint::MapGet { .. } => true,
            StatementHint::MapRemove { .. } => true,
            StatementHint::MapContainsKey { .. } => true,
            StatementHint::MapKeys { .. } => true,
            StatementHint::MapValues { .. } => true,
            StatementHint::SetCreate { .. } => true,
            StatementHint::SetAdd { .. } => true,
            StatementHint::SetRemove { .. } => true,
            StatementHint::SetContains { .. } => true,
            StatementHint::SetUnion { .. } => true,
            StatementHint::SetIntersection { .. } => true,
            StatementHint::TreeNode { .. } => true,
            StatementHint::TreeInsert { .. } => true,
            StatementHint::TreeSearch { .. } => true,
            StatementHint::TreeTraverse { .. } => true,
            StatementHint::ClassDef { .. } => true,
            StatementHint::ClassField { .. } => true,
            StatementHint::ClassMethod { .. } => true,
            StatementHint::Constructor { .. } => true,
            StatementHint::Destructor { .. } => true,
            StatementHint::InterfaceDef { .. } => true,
            StatementHint::InterfaceMethod { .. } => true,
            StatementHint::ObjectCreate { .. } => true,
            StatementHint::MethodCall { .. } => true,
            StatementHint::PropertyAccess { .. } => true,
            StatementHint::PropertyAssign { .. } => true,
            StatementHint::SuperCall { .. } => true,
            StatementHint::ThisReference => true,
            StatementHint::Lambda { .. } => true,
            StatementHint::HigherOrderFunction { .. } => true,
            StatementHint::MapFunction { .. } => true,
            StatementHint::FilterFunction { .. } => true,
            StatementHint::ReduceFunction { .. } => true,
            StatementHint::ForEachFunction { .. } => true,
            StatementHint::ThreadCreate { .. } => true,
            StatementHint::ThreadJoin { .. } => true,
            StatementHint::ThreadDetach { .. } => true,
            StatementHint::ThreadSleep { .. } => true,
            StatementHint::MutexCreate { .. } => true,
            StatementHint::MutexLock { .. } => true,
            StatementHint::MutexUnlock { .. } => true,
            StatementHint::MutexTryLock { .. } => true,
            StatementHint::SemaphoreCreate { .. } => true,
            StatementHint::SemaphoreWait { .. } => true,
            StatementHint::SemaphoreSignal { .. } => true,
            StatementHint::ConditionCreate { .. } => true,
            StatementHint::ConditionWait { .. } => true,
            StatementHint::ConditionSignal { .. } => true,
            StatementHint::ConditionBroadcast { .. } => true,
            StatementHint::AtomicCreate { .. } => true,
            StatementHint::AtomicLoad { .. } => true,
            StatementHint::AtomicStore { .. } => true,
            StatementHint::AtomicCompareExchange { .. } => true,
            StatementHint::AtomicIncrement { .. } => true,
            StatementHint::AtomicDecrement { .. } => true,
            StatementHint::AsyncFunction { .. } => true,
            StatementHint::AwaitExpression { .. } => true,
            StatementHint::PromiseCreate { .. } => true,
            StatementHint::PromiseThen { .. } => true,
            StatementHint::PromiseCatch { .. } => true,
            StatementHint::PromiseAll { .. } => true,
            StatementHint::PromiseRace { .. } => true,
            StatementHint::DocComment { .. } => true,
            StatementHint::DocFunction { .. } => true,
            StatementHint::DocClass { .. } => true,
            // Everything else goes to AI
            _ => false,
        }
    }

    /// Format hints as a human-readable string for the AI prompt.
    pub fn to_prompt_hints(&self) -> String {
        match self {
            StatementHint::Declaration {
                names,
                type_hint,
                qualifiers,
                initial_value,
                is_array,
                array_size,
            } => {
                let mut parts = vec![format!("- Intent: Declaration")];
                parts.push(format!("- Variables: {}", names.join(", ")));
                if let Some(ty) = type_hint {
                    parts.push(format!("- Type: {}", ty));
                }
                if !qualifiers.is_empty() {
                    parts.push(format!("- Qualifiers: {}", qualifiers.join(" ")));
                }
                if *is_array {
                    parts.push("- Is array: yes".to_string());
                    if let Some(size) = array_size {
                        parts.push(format!("- Array size: {}", size));
                    }
                }
                if let Some(val) = initial_value {
                    parts.push(format!("- Initial value: {}", val));
                }
                parts.join("\n")
            }

            StatementHint::Assignment { targets, value } => {
                format!("- Intent: Assignment\n- Targets: {}\n- Value: {}", targets.join(", "), value)
            }

            StatementHint::Loop {
                iterator,
                start,
                end,
                collection,
                body_action,
            } => {
                let mut parts = vec!["- Intent: Loop".to_string()];
                if let Some(iter) = iterator {
                    parts.push(format!("- Iterator: {}", iter));
                }
                if let Some(s) = start {
                    parts.push(format!("- Start: {}", s));
                }
                if let Some(e) = end {
                    parts.push(format!("- End: {}", e));
                }
                if let Some(col) = collection {
                    parts.push(format!("- Collection: {}", col));
                }
                if let Some(action) = body_action {
                    parts.push(format!("- Body action: {}", action));
                }
                parts.join("\n")
            }

            StatementHint::Conditional {
                condition,
                then_action,
                else_action,
                is_else_if,
            } => {
                let mut parts = vec![
                    format!("- Intent: {}", if *is_else_if { "Else-If" } else { "If" }),
                    format!("- Condition: {}", condition),
                ];
                if let Some(then) = then_action {
                    parts.push(format!("- Then: {}", then));
                }
                if let Some(els) = else_action {
                    parts.push(format!("- Else: {}", els));
                }
                parts.join("\n")
            }

            StatementHint::Else => "- Intent: Else branch".to_string(),

            StatementHint::EndBlock => "- Intent: End block (closing brace)".to_string(),

            StatementHint::Print { content, is_literal } => {
                format!(
                    "- Intent: Print\n- Content: {}\n- Is literal string: {}",
                    content,
                    if *is_literal { "yes" } else { "no" }
                )
            }

            StatementHint::Return { value } => {
                if let Some(val) = value {
                    format!("- Intent: Return\n- Value: {}", val)
                } else {
                    "- Intent: Return (void)".to_string()
                }
            }

            StatementHint::Arithmetic {
                operation,
                left,
                right,
                target,
            } => {
                let op_str = match operation {
                    ArithmeticOp::Add => "Add",
                    ArithmeticOp::Subtract => "Subtract",
                    ArithmeticOp::Multiply => "Multiply",
                    ArithmeticOp::Divide => "Divide",
                    ArithmeticOp::Modulo => "Modulo",
                };
                let mut parts = vec![
                    format!("- Intent: Arithmetic ({})", op_str),
                    format!("- Left operand: {}", left),
                    format!("- Right operand: {}", right),
                ];
                if let Some(t) = target {
                    parts.push(format!("- Store result in: {}", t));
                }
                parts.join("\n")
            }

            StatementHint::Modify { target, delta } => {
                let action = if *delta > 0 { "Increment" } else { "Decrement" };
                format!("- Intent: {}\n- Target: {}", action, target)
            }

            StatementHint::PrePostModify { target, delta, position } => {
                let action = if *delta > 0 { "Increment" } else { "Decrement" };
                let pos = match position {
                    IncDecPosition::Pre => "Pre",
                    IncDecPosition::Post => "Post",
                };
                format!("- Intent: {} {}\n- Target: {}", pos, action, target)
            }

            StatementHint::FunctionDef {
                name,
                parameters,
                return_type,
            } => {
                let mut parts = vec![
                    "- Intent: Function definition".to_string(),
                    format!("- Name: {}", name),
                ];
                if !parameters.is_empty() {
                    let params: Vec<String> = parameters
                        .iter()
                        .map(|(ty, n)| format!("{} {}", ty, n))
                        .collect();
                    parts.push(format!("- Parameters: {}", params.join(", ")));
                }
                if let Some(ret) = return_type {
                    parts.push(format!("- Return type: {}", ret));
                }
                parts.join("\n")
            }

            StatementHint::StructDef { name, fields } => {
                let mut parts = vec![
                    "- Intent: Struct definition".to_string(),
                    format!("- Name: {}", name),
                ];
                if !fields.is_empty() {
                    let flds: Vec<String> = fields
                        .iter()
                        .map(|(ty, n)| format!("{} {}", ty, n))
                        .collect();
                    parts.push(format!("- Fields: {}", flds.join(", ")));
                }
                parts.join("\n")
            }

            StatementHint::BitfieldDecl { name, type_hint, width } => {
                let mut parts = vec![
                    "- Intent: Bitfield declaration".to_string(),
                    format!("- Name: {}", name),
                    format!("- Width: {}", width),
                ];
                if let Some(ty) = type_hint {
                    parts.push(format!("- Type: {}", ty));
                }
                parts.join("\n")
            }

            StatementHint::MainFunction => "- Intent: Main function entry point".to_string(),

            StatementHint::While { condition, body_action } => {
                let mut parts = vec![
                    "- Intent: While loop".to_string(),
                    format!("- Condition: {}", condition),
                ];
                if let Some(action) = body_action {
                    parts.push(format!("- Body action: {}", action));
                }
                parts.join("\n")
            }

            StatementHint::Read { variables } => {
                format!("- Intent: Read input\n- Variables: {}", variables.join(", "))
            }

            StatementHint::Switch { expression } => {
                format!("- Intent: Switch statement\n- Expression: {}", expression)
            }

            StatementHint::Case { value, action } => {
                let mut s = format!("- Intent: Case\n- Value: {}", value);
                if let Some(a) = action {
                    s.push_str(&format!("\n- Action: {}", a));
                }
                s
            }

            StatementHint::Default { action } => {
                let mut s = "- Intent: Default case".to_string();
                if let Some(a) = action {
                    s.push_str(&format!("\n- Action: {}", a));
                }
                s
            }

            StatementHint::DoWhileStart => "- Intent: Do-while loop start".to_string(),

            StatementHint::DoWhileEnd { condition } => {
                format!("- Intent: Do-while loop end\n- Condition: {}", condition)
            }

            StatementHint::Break => "- Intent: Break statement".to_string(),

            StatementHint::Continue => "- Intent: Continue statement".to_string(),

            StatementHint::FunctionCall { name, arguments } => {
                if arguments.is_empty() {
                    format!("- Intent: Function call\n- Name: {}", name)
                } else {
                    format!("- Intent: Function call\n- Name: {}\n- Arguments: {}", name, arguments.join(", "))
                }
            }

            StatementHint::Include { header, is_system } => {
                format!("- Intent: Include\n- Header: {}\n- System header: {}", header, is_system)
            }

            StatementHint::Define { name, value } => {
                format!("- Intent: Define macro\n- Name: {}\n- Value: {}", name, value)
            }

            StatementHint::PointerDecl { base_type, name } => {
                format!("- Intent: Pointer declaration\n- Type: {} *\n- Name: {}", base_type, name)
            }

            StatementHint::Dereference { target } => {
                format!("- Intent: Dereference\n- Target: {}", target)
            }

            StatementHint::AddressOf { target } => {
                format!("- Intent: Address of\n- Target: {}", target)
            }

            StatementHint::Malloc { count, element_type } => {
                format!("- Intent: Allocate memory\n- Count: {}\n- Type: {}", count, element_type)
            }

            StatementHint::Free { target } => {
                format!("- Intent: Free memory\n- Target: {}", target)
            }

            StatementHint::EnumDef { name, values } => {
                format!("- Intent: Enum definition\n- Name: {}\n- Values: {}", name, values.join(", "))
            }

            StatementHint::Typedef { original_type, new_name } => {
                format!("- Intent: Typedef\n- Original: {}\n- New name: {}", original_type, new_name)
            }

            StatementHint::StringDecl { name, initial_value, size } => {
                let mut s = format!("- Intent: String declaration\n- Name: {}", name);
                if let Some(v) = initial_value {
                    s.push_str(&format!("\n- Initial value: {}", v));
                }
                if let Some(sz) = size {
                    s.push_str(&format!("\n- Size: {}", sz));
                }
                s
            }

            StatementHint::Comment { text, is_block } => {
                format!("- Intent: Comment\n- Text: {}\n- Block: {}", text, is_block)
            }

            StatementHint::Bitwise { operation, left, right, target } => {
                let op_str = match operation {
                    BitwiseOp::And => "AND",
                    BitwiseOp::Or => "OR",
                    BitwiseOp::Xor => "XOR",
                    BitwiseOp::Not => "NOT",
                    BitwiseOp::ShiftLeft => "Shift Left",
                    BitwiseOp::ShiftRight => "Shift Right",
                };
                let mut s = format!("- Intent: Bitwise {}\n- Left: {}", op_str, left);
                if let Some(r) = right {
                    s.push_str(&format!("\n- Right: {}", r));
                }
                if let Some(t) = target {
                    s.push_str(&format!("\n- Store in: {}", t));
                }
                s
            }

            StatementHint::Cast { expression, target_type } => {
                format!("- Intent: Type cast\n- Expression: {}\n- To type: {}", expression, target_type)
            }

            StatementHint::ArrayAccess { array, index, value } => {
                let mut s = format!("- Intent: Array access\n- Array: {}\n- Index: {}", array, index);
                if let Some(v) = value {
                    s.push_str(&format!("\n- Set value: {}", v));
                }
                s
            }

            StatementHint::SizeOf { target } => {
                format!("- Intent: Size of\n- Target: {}", target)
            }

            StatementHint::Ternary { condition, true_value, false_value } => {
                format!("- Intent: Ternary\n- Condition: {}\n- True: {}\n- False: {}", condition, true_value, false_value)
            }

            StatementHint::CompoundAssign { target, operator, value } => {
                let op = match operator {
                    CompoundOp::AddAssign => "+=",
                    CompoundOp::SubAssign => "-=",
                    CompoundOp::MulAssign => "*=",
                    CompoundOp::DivAssign => "/=",
                    CompoundOp::ModAssign => "%=",
                    CompoundOp::ShlAssign => "<<=",
                    CompoundOp::ShrAssign => ">>=",
                    CompoundOp::AndAssign => "&=",
                    CompoundOp::OrAssign => "|=",
                    CompoundOp::XorAssign => "^=",
                };
                format!("- Intent: Compound assignment\n- Target: {}\n- Operator: {}\n- Value: {}", target, op, value)
            }

            StatementHint::Logical { operation, left, right, target } => {
                let op = match operation {
                    LogicalOp::And => "&&",
                    LogicalOp::Or => "||",
                    LogicalOp::Not => "!",
                };
                let mut s = vec!["- Intent: Logical op".to_string(), format!("- Operator: {}", op)];
                s.push(format!("- Left: {}", left));
                if let Some(r) = right {
                    s.push(format!("- Right: {}", r));
                }
                if let Some(t) = target {
                    s.push(format!("- Store in: {}", t));
                }
                s.join("\n")
            }

            StatementHint::IfDef { symbol } => format!("- Intent: IfDef\n- Symbol: {}", symbol),
            StatementHint::IfNDef { symbol } => format!("- Intent: IfNDef\n- Symbol: {}", symbol),
            StatementHint::EndIf => "- Intent: EndIf".to_string(),
            StatementHint::Undef { symbol } => format!("- Intent: Undef\n- Symbol: {}", symbol),
            StatementHint::Pragma { value } => format!("- Intent: Pragma\n- Value: {}", value),
            StatementHint::MacroFunction { name, params, body } => {
                format!("- Intent: Macro function\n- Name: {}\n- Params: {}\n- Body: {}", name, params.join(", "), body)
            }

            StatementHint::Realloc { pointer, count, element_type } => {
                format!("- Intent: Realloc\n- Pointer: {}\n- Count: {}\n- Type: {}", pointer, count, element_type)
            }
            StatementHint::Calloc { count, element_type, target } => {
                let mut s = vec![
                    "- Intent: Calloc".to_string(),
                    format!("- Count: {}", count),
                    format!("- Type: {}", element_type),
                ];
                if let Some(t) = target {
                    s.push(format!("- Store in: {}", t));
                }
                s.join("\n")
            }
            StatementHint::PointerArithmetic { pointer, offset, direction } => {
                let dir = match direction {
                    PointerDir::Forward => "forward",
                    PointerDir::Backward => "backward",
                };
                format!("- Intent: Pointer arithmetic\n- Pointer: {}\n- Offset: {}\n- Direction: {}", pointer, offset, dir)
            }
            StatementHint::FunctionPointer { return_type, name, params } => {
                format!("- Intent: Function pointer\n- Return: {}\n- Name: {}\n- Params: {}", return_type, name, params.join(", "))
            }
            StatementHint::DoublePointer { base_type, name } => {
                format!("- Intent: Double pointer\n- Type: {} **\n- Name: {}", base_type, name)
            }
            StatementHint::NullAssign { target } => {
                format!("- Intent: Null assignment\n- Target: {}", target)
            }

            StatementHint::MultiArrayDecl { type_hint, name, dimensions } => {
                let mut s = vec![
                    "- Intent: Multidimensional array".to_string(),
                    format!("- Name: {}", name),
                    format!("- Dimensions: {}", dimensions.join(" x ")),
                ];
                if let Some(ty) = type_hint {
                    s.push(format!("- Type: {}", ty));
                }
                s.join("\n")
            }
            StatementHint::ArrayInit { type_hint, name, values } => {
                let mut s = vec![
                    "- Intent: Array initialization".to_string(),
                    format!("- Name: {}", name),
                    format!("- Values: {}", values.join(", ")),
                ];
                if let Some(ty) = type_hint {
                    s.push(format!("- Type: {}", ty));
                }
                s.join("\n")
            }
            StatementHint::DesignatedInit { type_hint, name, designators } => {
                let mut s = vec![
                    "- Intent: Designated initializer".to_string(),
                    format!("- Name: {}", name),
                ];
                if !designators.is_empty() {
                    let items: Vec<String> = designators
                        .iter()
                        .map(|(d, v)| format!("{}={}", d, v))
                        .collect();
                    s.push(format!("- Designators: {}", items.join(", ")));
                }
                if let Some(ty) = type_hint {
                    s.push(format!("- Type: {}", ty));
                }
                s.join("\n")
            }
            StatementHint::CompoundLiteral { type_hint, values, fields, is_array } => {
                let mut s = vec![
                    "- Intent: Compound literal".to_string(),
                    format!("- Type: {}", type_hint),
                    format!("- Is array: {}", if *is_array { "yes" } else { "no" }),
                ];
                if !values.is_empty() {
                    s.push(format!("- Values: {}", values.join(", ")));
                }
                if !fields.is_empty() {
                    let items: Vec<String> = fields
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    s.push(format!("- Fields: {}", items.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::MultiDimAccess { array, indices, value } => {
                let mut s = vec![
                    "- Intent: Multidimensional access".to_string(),
                    format!("- Array: {}", array),
                    format!("- Indices: {}", indices.join(", ")),
                ];
                if let Some(v) = value {
                    s.push(format!("- Set value: {}", v));
                }
                s.join("\n")
            }

            StatementHint::StructAccess { object, field } => {
                format!("- Intent: Struct access\n- Object: {}\n- Field: {}", object, field)
            }
            StatementHint::StructArrow { pointer, field } => {
                format!("- Intent: Struct arrow access\n- Pointer: {}\n- Field: {}", pointer, field)
            }
            StatementHint::StructInit { struct_name, var_name, fields } => {
                let mut s = vec![
                    "- Intent: Struct init".to_string(),
                    format!("- Type: {}", struct_name),
                    format!("- Variable: {}", var_name),
                ];
                if !fields.is_empty() {
                    let f: Vec<String> = fields.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                    s.push(format!("- Fields: {}", f.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::AnonymousStruct { parent, fields } => {
                let mut s = vec!["- Intent: Anonymous struct".to_string()];
                if let Some(p) = parent {
                    s.push(format!("- Parent: {}", p));
                }
                if !fields.is_empty() {
                    let f: Vec<String> = fields.iter().map(|(ty, n)| format!("{} {}", ty, n)).collect();
                    s.push(format!("- Fields: {}", f.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::UnionDef { name, fields } => {
                let mut s = vec![
                    "- Intent: Union definition".to_string(),
                    format!("- Name: {}", name),
                ];
                if !fields.is_empty() {
                    let f: Vec<String> = fields.iter().map(|(ty, n)| format!("{} {}", ty, n)).collect();
                    s.push(format!("- Fields: {}", f.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::AnonymousUnion { parent, fields } => {
                let mut s = vec!["- Intent: Anonymous union".to_string()];
                if let Some(p) = parent {
                    s.push(format!("- Parent: {}", p));
                }
                if !fields.is_empty() {
                    let f: Vec<String> = fields.iter().map(|(ty, n)| format!("{} {}", ty, n)).collect();
                    s.push(format!("- Fields: {}", f.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::StructArray { struct_name, var_name, size } => {
                format!("- Intent: Struct array\n- Type: {}\n- Name: {}\n- Size: {}", struct_name, var_name, size)
            }

            StatementHint::Goto { label } => format!("- Intent: Goto\n- Label: {}", label),
            StatementHint::Label { name } => format!("- Intent: Label\n- Name: {}", name),
            StatementHint::InfiniteLoop => "- Intent: Infinite loop".to_string(),
            StatementHint::ForEver => "- Intent: Forever loop".to_string(),

            StatementHint::FunctionPrototype { name, parameters, return_type } => {
                let mut s = vec![
                    "- Intent: Function prototype".to_string(),
                    format!("- Name: {}", name),
                ];
                if !parameters.is_empty() {
                    let p: Vec<String> = parameters.iter().map(|(t, n)| format!("{} {}", t, n)).collect();
                    s.push(format!("- Parameters: {}", p.join(", ")));
                }
                if let Some(ret) = return_type {
                    s.push(format!("- Return type: {}", ret));
                }
                s.join("\n")
            }
            StatementHint::QualifiedFunction { qualifier, name, parameters, return_type } => {
                let mut s = vec![
                    "- Intent: Qualified function".to_string(),
                    format!("- Qualifier: {}", qualifier),
                    format!("- Name: {}", name),
                ];
                if !parameters.is_empty() {
                    let p: Vec<String> = parameters.iter().map(|(t, n)| format!("{} {}", t, n)).collect();
                    s.push(format!("- Parameters: {}", p.join(", ")));
                }
                if let Some(ret) = return_type {
                    s.push(format!("- Return type: {}", ret));
                }
                s.join("\n")
            }

            StatementHint::FileOpen { var_name, path, mode } => {
                format!("- Intent: File open\n- Var: {}\n- Path: {}\n- Mode: {}", var_name, path, mode)
            }
            StatementHint::FileClose { var_name } => {
                format!("- Intent: File close\n- Var: {}", var_name)
            }
            StatementHint::FileRead { var_name, buffer, size } => {
                format!("- Intent: File read\n- File: {}\n- Buffer: {}\n- Size: {}", var_name, buffer, size)
            }
            StatementHint::FileWrite { var_name, buffer, size } => {
                format!("- Intent: File write\n- File: {}\n- Buffer: {}\n- Size: {}", var_name, buffer, size)
            }
            StatementHint::Fgets { buffer, size, var_name } => {
                format!("- Intent: fgets\n- Buffer: {}\n- Size: {}\n- File: {}", buffer, size, var_name)
            }
            StatementHint::Fputs { content, var_name } => {
                format!("- Intent: fputs\n- Content: {}\n- File: {}", content, var_name)
            }
            StatementHint::Fprintf { var_name, format: fmt, args } => {
                let mut s = vec![
                    "- Intent: fprintf".to_string(),
                    format!("- File: {}", var_name),
                    format!("- Format: {}", fmt),
                ];
                if !args.is_empty() {
                    s.push(format!("- Args: {}", args.join(", ")));
                }
                s.join("\n")
            }
            StatementHint::Fscanf { var_name, args } => {
                let mut s = vec![
                    "- Intent: fscanf".to_string(),
                    format!("- File: {}", var_name),
                ];
                if !args.is_empty() {
                    s.push(format!("- Args: {}", args.join(", ")));
                }
                s.join("\n")
            }

            StatementHint::Strcpy { dest, src } => {
                format!("- Intent: strcpy\n- Dest: {}\n- Src: {}", dest, src)
            }
            StatementHint::Strncpy { dest, src, count } => {
                format!("- Intent: strncpy\n- Dest: {}\n- Src: {}\n- Count: {}", dest, src, count)
            }
            StatementHint::Strcat { dest, src } => {
                format!("- Intent: strcat\n- Dest: {}\n- Src: {}", dest, src)
            }
            StatementHint::Strcmp { left, right, target } => {
                let mut s = vec![
                    "- Intent: strcmp".to_string(),
                    format!("- Left: {}", left),
                    format!("- Right: {}", right),
                ];
                if let Some(t) = target {
                    s.push(format!("- Store in: {}", t));
                }
                s.join("\n")
            }
            StatementHint::Strlen { target, store_in } => {
                let mut s = vec![
                    "- Intent: strlen".to_string(),
                    format!("- Target: {}", target),
                ];
                if let Some(t) = store_in {
                    s.push(format!("- Store in: {}", t));
                }
                s.join("\n")
            }
            StatementHint::Sprintf { buffer, format: fmt, args } => {
                let mut s = vec![
                    "- Intent: sprintf".to_string(),
                    format!("- Buffer: {}", buffer),
                    format!("- Format: {}", fmt),
                ];
                if !args.is_empty() {
                    s.push(format!("- Args: {}", args.join(", ")));
                }
                s.join("\n")
            }

            StatementHint::Memcpy { dest, src, size } => {
                format!("- Intent: memcpy\n- Dest: {}\n- Src: {}\n- Size: {}", dest, src, size)
            }
            StatementHint::Memset { dest, value, size } => {
                format!("- Intent: memset\n- Dest: {}\n- Value: {}\n- Size: {}", dest, value, size)
            }
            StatementHint::Exit { code } => {
                format!("- Intent: exit\n- Code: {}", code)
            }
            StatementHint::Rand { store_in } => {
                if let Some(t) = store_in {
                    format!("- Intent: rand\n- Store in: {}", t)
                } else {
                    "- Intent: rand".to_string()
                }
            }
            StatementHint::MathFunc { func, args, store_in } => {
                let f = match func {
                    MathFuncKind::Sqrt => "sqrt",
                    MathFuncKind::Pow => "pow",
                    MathFuncKind::Abs => "abs",
                    MathFuncKind::Sin => "sin",
                    MathFuncKind::Cos => "cos",
                    MathFuncKind::Tan => "tan",
                    MathFuncKind::Exp => "exp",
                    MathFuncKind::Log => "log",
                };
                let mut s = vec![
                    "- Intent: math func".to_string(),
                    format!("- Func: {}", f),
                    format!("- Args: {}", args.join(", ")),
                ];
                if let Some(t) = store_in {
                    s.push(format!("- Store in: {}", t));
                }
                s.join("\n")
            }

            StatementHint::Assert { expression } => {
                format!("- Intent: assert\n- Expression: {}", expression)
            }
            StatementHint::Perror { message } => {
                if let Some(m) = message {
                    format!("- Intent: perror\n- Message: {}", m)
                } else {
                    "- Intent: perror".to_string()
                }
            }
            StatementHint::ErrnoCheck => "- Intent: errno check".to_string(),

            StatementHint::StdLibCall { name, args } => {
                let mut s = vec![
                    "- Intent: stdlib call".to_string(),
                    format!("- Function: {}", name),
                ];
                if !args.is_empty() {
                    s.push(format!("- Args: {}", args.join(", ")));
                }
                s.join("\n")
            }

            StatementHint::StaticAssert { condition, message } => {
                let mut s = vec![
                    "- Intent: Static assert".to_string(),
                    format!("- Condition: {}", condition),
                ];
                if let Some(msg) = message {
                    s.push(format!("- Message: {}", msg));
                }
                s.join("\n")
            }
            StatementHint::CommaExpression { expressions } => {
                if expressions.is_empty() {
                    "- Intent: Comma expression".to_string()
                } else {
                    format!("- Intent: Comma expression\n- Expressions: {}", expressions.join(", "))
                }
            }

            StatementHint::Unknown { original } => {
                format!("- Intent: Unknown (AI should interpret)\n- Original: \"{}\"", original)
            }
            _ => format!("- Intent: {:?}", self),
        }
    }
}

// ============================================================================
// Hint Extraction Keywords
// ============================================================================

const DECLARE_KEYWORDS: &[&str] = &[
    "declare", "define", "create", "make", "set up", "setup", "initialize", "init",
    "turn", "convert", "build", "form",
];

const LOOP_KEYWORDS: &[&str] = &[
    "loop", "for loop", "for each", "foreach", "for every", "for all", "for",
    "while",  // "while" with range syntax like "while i 0 to 10" becomes a for loop
    "iterate", "iterate over", "iterate through", "loop over", "loop through",
    "go through", "go over", "cycle through", "traverse", "walk through",
];

const IF_KEYWORDS: &[&str] = &[
    "if", "when", "whenever", "in case", "provided that", "assuming",
    "in the event", "should",
];

const ELSE_IF_KEYWORDS: &[&str] = &["else if", "otherwise if", "elsewhen", "else when"];

const LIST_KEYWORDS: &[&str] = &["list", "array", "vector", "arr", "collection"];
const QUALIFIER_WORDS: &[&str] = &["const", "volatile", "register", "static", "extern"];
const DECL_TYPE_WORDS: &[&str] = &[
    "int", "integer", "integers",
    "float", "floats",
    "double", "doubles",
    "char", "chars",
    "string", "strings",
    "bool", "boolean", "bools", "booleans",
    "long", "short", "unsigned", "signed", "size_t", "long long", "unsigned long", "unsigned int",
];

const ADD_KEYWORDS: &[&str] = &["add", "sum", "total", "combine", "plus", "tally", "sum up"];
const SUBTRACT_KEYWORDS: &[&str] = &[
    "subtract", "minus", "difference", "difference between", "difference of",
    "remove", "take away", "decrease",
];
const MULTIPLY_KEYWORDS: &[&str] = &["multiply", "product", "product of", "times"];
const DIVIDE_KEYWORDS: &[&str] = &["divide", "quotient", "quotient of", "split", "divide by"];
const MODULO_KEYWORDS: &[&str] = &["mod", "modulo", "remainder", "remainder of"];

const LOGICAL_AND_KEYWORDS: &[&str] = &["logical and", "and", "both"];
const LOGICAL_OR_KEYWORDS: &[&str] = &["logical or", "or", "either"];
const LOGICAL_NOT_KEYWORDS: &[&str] = &["not", "logical not"];

const PRE_IFDEF_KEYWORDS: &[&str] = &["if defined", "ifdef"];
const PRE_IFNDEF_KEYWORDS: &[&str] = &["if not defined", "ifndef"];
const PRE_ENDIF_KEYWORDS: &[&str] = &["end if defined", "end ifdef", "endif", "end if not defined"];
const PRE_UNDEF_KEYWORDS: &[&str] = &["undefine", "undef"];
const PRE_PRAGMA_KEYWORDS: &[&str] = &["pragma"];
const PRE_MACRO_KEYWORDS: &[&str] = &["define macro", "macro"];

// ============================================================================
// Extraction Functions
// ============================================================================

/// Extract structured hints from a translation request.
pub fn extract(request: &TranslateLineRequest) -> StatementHint {
    let normalized = crate::synonyms::normalize(request.english_line.trim());
    let line = normalized.trim();
    let include_context = VariableContext::from_code(&request.code_before).included_headers;

    if line.is_empty() {
        return StatementHint::Unknown {
            original: line.to_string(),
        };
    }

    // Try each extractor in order of specificity
    if let Some(hint) = try_extract_else_if(line) {
        return hint;
    }

    if let Some(hint) = try_extract_else(line) {
        return hint;
    }

    if let Some(hint) = try_extract_end_block(line) {
        return hint;
    }

    if let Some(hint) = try_extract_main(line) {
        return hint;
    }

    if let Some(hint) = try_extract_return(line) {
        return hint;
    }

    if let Some(hint) = try_extract_print(line) {
        return hint;
    }

    if let Some(hint) = try_extract_increment_decrement(line) {
        return hint;
    }

    if let Some(hint) = try_extract_arithmetic(line) {
        return hint;
    }

    if let Some(hint) = try_extract_qualified_declaration(line) {
        return hint;
    }

    if let Some(hint) = try_extract_flexible_assignment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_equals_assignment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_assignment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_bitfield(line) {
        return hint;
    }

    if let Some(hint) = try_extract_vla_declaration(line) {
        return hint;
    }

    if let Some(hint) = try_extract_declaration(line) {
        return hint;
    }

    if let Some(hint) = try_extract_read(line) {
        return hint;
    }

    if let Some(hint) = try_extract_while(line) {
        return hint;
    }

    if let Some(hint) = try_extract_loop(line) {
        return hint;
    }

    if let Some(hint) = try_extract_conditional(line) {
        return hint;
    }

    if let Some(hint) = try_extract_function(line) {
        return hint;
    }

    if let Some(hint) = try_extract_anonymous_struct_union(line) {
        return hint;
    }

    if let Some(hint) = try_extract_struct(line) {
        return hint;
    }

    // New extractors
    if let Some(hint) = try_extract_switch(line) {
        return hint;
    }

    if let Some(hint) = try_extract_case(line) {
        return hint;
    }

    if let Some(hint) = try_extract_default(line) {
        return hint;
    }

    if let Some(hint) = try_extract_do_while(line) {
        return hint;
    }

    if let Some(hint) = try_extract_break_continue(line) {
        return hint;
    }

    if let Some(hint) = try_extract_function_call(line) {
        return hint;
    }

    if let Some(hint) = try_extract_stdlib_call(line, &include_context) {
        return hint;
    }

    if let Some(hint) = try_extract_include(line) {
        return hint;
    }

    if let Some(hint) = try_extract_define(line) {
        return hint;
    }

    if let Some(hint) = try_extract_pointer(line) {
        return hint;
    }

    if let Some(hint) = try_extract_memory_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_enum(line) {
        return hint;
    }

    if let Some(hint) = try_extract_typedef(line) {
        return hint;
    }

    if let Some(hint) = try_extract_string_decl(line) {
        return hint;
    }

    if let Some(hint) = try_extract_comment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_bitwise(line) {
        return hint;
    }

    if let Some(hint) = try_extract_cast(line) {
        return hint;
    }

    if let Some(hint) = try_extract_array_access(line) {
        return hint;
    }

    if let Some(hint) = try_extract_sizeof(line) {
        return hint;
    }

    if let Some(hint) = try_extract_ternary(line) {
        return hint;
    }

    if let Some(hint) = try_extract_bare_declaration(line) {
        return hint;
    }

    if let Some(hint) = try_extract_compound_assign(line) {
        return hint;
    }

    if let Some(hint) = try_extract_logical(line) {
        return hint;
    }

    if let Some(hint) = try_extract_static_assert(line) {
        return hint;
    }

    if let Some(hint) = try_extract_comma_expression(line) {
        return hint;
    }

    if let Some(hint) = try_extract_preprocessor(line) {
        return hint;
    }

    if let Some(hint) = try_extract_advanced_memory(line) {
        return hint;
    }

    if let Some(hint) = try_extract_compound_literal(line) {
        return hint;
    }

    if let Some(hint) = try_extract_designated_init(line) {
        return hint;
    }

    if let Some(hint) = try_extract_multi_array(line) {
        return hint;
    }

    if let Some(hint) = try_extract_struct_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_control_flow_ext(line) {
        return hint;
    }

    if let Some(hint) = try_extract_function_ext(line) {
        return hint;
    }

    if let Some(hint) = try_extract_file_io(line) {
        return hint;
    }

    if let Some(hint) = try_extract_string_funcs(line) {
        return hint;
    }

    if let Some(hint) = try_extract_stdlib_funcs(line) {
        return hint;
    }

    if let Some(hint) = try_extract_error_handling(line) {
        return hint;
    }

    if let Some(hint) = try_extract_try_block(line) {
        return hint;
    }

    if let Some(hint) = try_extract_except_block(line) {
        return hint;
    }

    if let Some(hint) = try_extract_finally_block(line) {
        return hint;
    }

    if let Some(hint) = try_extract_raise(line) {
        return hint;
    }

    if let Some(hint) = try_extract_error_check(line) {
        return hint;
    }

    if let Some(hint) = try_extract_setjmp(line) {
        return hint;
    }

    if let Some(hint) = try_extract_longjmp(line) {
        return hint;
    }

    if let Some(hint) = try_extract_test_function(line) {
        return hint;
    }

    if let Some(hint) = try_extract_test_assert(line) {
        return hint;
    }

    if let Some(hint) = try_extract_test_equal(line) {
        return hint;
    }

    if let Some(hint) = try_extract_labeled_control(line) {
        return hint;
    }

    if let Some(hint) = try_extract_match(line) {
        return hint;
    }

    if let Some(hint) = try_extract_guard(line) {
        return hint;
    }

    if let Some(hint) = try_extract_list_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_stack_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_queue_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_map_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_set_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_class_def(line) {
        return hint;
    }

    if let Some(hint) = try_extract_class_field(line) {
        return hint;
    }

    if let Some(hint) = try_extract_class_method(line) {
        return hint;
    }

    if let Some(hint) = try_extract_constructor(line) {
        return hint;
    }

    if let Some(hint) = try_extract_object_create(line) {
        return hint;
    }

    if let Some(hint) = try_extract_method_call(line) {
        return hint;
    }

    if let Some(hint) = try_extract_lambda(line) {
        return hint;
    }

    if let Some(hint) = try_extract_map_filter_reduce(line) {
        return hint;
    }

    if let Some(hint) = try_extract_thread_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_sync_ops(line) {
        return hint;
    }

    if let Some(hint) = try_extract_async(line) {
        return hint;
    }

    if let Some(hint) = try_extract_doc_comment(line) {
        return hint;
    }

    if let Some(hint) = try_extract_doc_function(line) {
        return hint;
    }

    if let Some(hint) = try_extract_doc_class(line) {
        return hint;
    }

    // Fallback to unknown
    StatementHint::Unknown {
        original: line.to_string(),
    }
}

fn strip_keyword<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let lower = line.to_lowercase();
    if lower.starts_with(keyword) {
        let rest = &line[keyword.len()..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with(':') {
            return Some(rest.trim_start());
        }
    }
    None
}

fn strip_any_keyword<'a>(line: &'a str, keywords: &[&str]) -> Option<&'a str> {
    // Sort by length descending to match longer keywords first
    let mut sorted: Vec<_> = keywords.iter().collect();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));
    
    for keyword in sorted {
        if let Some(rest) = strip_keyword(line, keyword) {
            return Some(rest);
        }
    }
    None
}

fn try_extract_else_if(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, ELSE_IF_KEYWORDS)?;
    let condition = normalize_condition(rest.trim());
    Some(StatementHint::Conditional {
        condition,
        then_action: None,
        else_action: None,
        is_else_if: true,
    })
}

fn try_extract_else(line: &str) -> Option<StatementHint> {
    let trimmed = line.trim().to_lowercase();
    if trimmed == "else" || trimmed == "otherwise" {
        Some(StatementHint::Else)
    } else {
        None
    }
}

fn try_extract_end_block(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower.starts_with("end") || lower == "}" {
        Some(StatementHint::EndBlock)
    } else {
        None
    }
}

fn try_extract_main(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower.starts_with("create main")
        || lower.starts_with("make main")
        || lower.starts_with("write main")
        || lower.starts_with("build main")
        || lower.contains("entry point")
        || lower == "main"
    {
        Some(StatementHint::MainFunction)
    } else {
        None
    }
}

fn try_extract_return(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "return")?;
    let value = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    Some(StatementHint::Return { value })
}

fn try_extract_print(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "print")?;
    let content = rest.trim();
    
    if content.is_empty() {
        return None;
    }
    
    // Determine if content is a literal or variable expression
    let is_literal = if content.starts_with('"') && content.ends_with('"') {
        // Explicitly quoted - definitely a literal
        true
    } else if content.contains(' ') {
        // Multi-word without quotes - treat as implicit literal (e.g., "hello world")
        true
    } else {
        // Single word - check if it looks like a variable name
        // If it's purely alphanumeric/underscore and starts with letter/underscore, it's a variable
        // Otherwise treat as literal text
        let first_char = content.chars().next().unwrap_or('_');
        let looks_like_identifier = (first_char.is_ascii_alphabetic() || first_char == '_')
            && content.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        !looks_like_identifier
    };
    
    Some(StatementHint::Print {
        content: content.to_string(),
        is_literal,
    })
}

fn try_extract_increment_decrement(line: &str) -> Option<StatementHint> {
    for (kw, delta, position) in [
        ("pre increment", 1, IncDecPosition::Pre),
        ("prefix increment", 1, IncDecPosition::Pre),
        ("post increment", 1, IncDecPosition::Post),
        ("postfix increment", 1, IncDecPosition::Post),
        ("pre decrement", -1, IncDecPosition::Pre),
        ("prefix decrement", -1, IncDecPosition::Pre),
        ("post decrement", -1, IncDecPosition::Post),
        ("postfix decrement", -1, IncDecPosition::Post),
    ] {
        if let Some(rest) = strip_keyword(line, kw) {
            let target = sanitize_identifier(rest);
            if !target.is_empty() {
                return Some(StatementHint::PrePostModify {
                    target,
                    delta,
                    position,
                });
            }
        }
    }
    if let Some(rest) = strip_keyword(line, "increment") {
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Modify { target, delta: 1 });
        }
    }
    if let Some(rest) = strip_keyword(line, "decrement") {
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Modify { target, delta: -1 });
        }
    }
    None
}

fn try_extract_arithmetic(line: &str) -> Option<StatementHint> {
    let trimmed = line.trim();

    // Try add
    if let Some(rest) = strip_any_keyword(trimmed, ADD_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" and ", " to "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Add,
                left,
                right,
                target,
            });
        }
    }

    // Try subtract
    if let Some(rest) = strip_any_keyword(trimmed, SUBTRACT_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" from ", " and "]) {
            // For subtract, "subtract X from Y" means Y - X
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Subtract,
                left: right,
                right: left,
                target,
            });
        }
    }

    // Try multiply
    if let Some(rest) = strip_any_keyword(trimmed, MULTIPLY_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" by ", " and "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Multiply,
                left,
                right,
                target,
            });
        }
    }

    // Try divide
    if let Some(rest) = strip_any_keyword(trimmed, DIVIDE_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" by ", " into "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Divide,
                left,
                right,
                target,
            });
        }
    }

    // Try modulo
    if let Some(rest) = strip_any_keyword(trimmed, MODULO_KEYWORDS) {
        if let Some((left, right, target)) = parse_binary_op(rest, &[" by ", " of ", " modulo "]) {
            return Some(StatementHint::Arithmetic {
                operation: ArithmeticOp::Modulo,
                left,
                right,
                target,
            });
        }
    }

    None
}

fn try_extract_flexible_assignment(line: &str) -> Option<StatementHint> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();

    // Avoid clobbering control-flow and structural statements
    for prefix in ["if ", "while ", "loop ", "for ", "switch ", "case ", "else "] {
        if lower.starts_with(prefix) {
            return None;
        }
    }

    // "the variable x is <expr>" or "variable x is <expr>"
    if let Some(rest) = strip_any_keyword(trimmed, &["the variable", "variable"]) {
        if let Some((target_part, value_part)) = split_on_marker(rest, &[" is ", " equals ", " set ", " be "]) {
            return build_declaration_hint(&target_part, &value_part);
        }
    }

    // "<target> variable is <expr>"
    if let Some(idx) = lower.find(" variable ") {
        let target_part = &trimmed[..idx];
        let after = trimmed[idx + " variable ".len()..].trim_start();
        if let Some(value_part) = strip_any_keyword(after, &["is", "equals", "set", "be"]) {
            if let Some(hint) = build_declaration_hint(target_part, value_part) {
                return Some(hint);
            }
        }
    }

    // "store <expr> in <target>" / "put <expr> into <target>"
    if let Some(rest) = strip_any_keyword(trimmed, &["store", "put", "save", "set"]) {
        let rest_lower = rest.to_lowercase();
        for marker in [" into ", " in ", " to "] {
            if let Some(idx) = rest_lower.rfind(marker) {
                let value_part = rest[..idx].trim();
                let target_part = rest[idx + marker.len()..].trim();
                if let Some(hint) = build_assignment_hint(target_part, value_part) {
                    return Some(hint);
                }
            }
        }
    }

    // "let x be <expr>" -> normalized to "set x be <expr>"
    if let Some(rest) = strip_keyword(trimmed, "set") {
        let rest_lower = rest.to_lowercase();
        if let Some(idx) = rest_lower.find(" be ") {
            let target_part = rest[..idx].trim();
            let value_part = rest[idx + 4..].trim();
            if let Some(hint) = build_declaration_hint(target_part, value_part) {
                return Some(hint);
            }
        }
    }

    // "<expr> is <target>"
    if let Some((left, right)) = split_on_marker(trimmed, &[" is ", " equals ", " set "]) {
        if looks_like_expression(&left) && !looks_like_comparison(&left) {
            if let Some(hint) = build_assignment_hint(&right, &left) {
                return Some(hint);
            }
        }
    }

    // "<target> is <expr>"
    if let Some((left, right)) = split_on_marker(trimmed, &[" is ", " equals ", " set "]) {
        if !looks_like_comparison(&right) || looks_like_expression(&right) {
            if let Some(hint) = build_assignment_hint(&left, &right) {
                return Some(hint);
            }
        }
    }

    // "<target> <expr>" without explicit marker (e.g., "sum a plus b")
    let mut tokens = trimmed.split_whitespace();
    if let Some(first) = tokens.next() {
        let target = sanitize_identifier(first);
        let rest = tokens.collect::<Vec<&str>>().join(" ");
        if !target.is_empty() && !rest.is_empty() && looks_like_expression(&rest) {
            if let Some(hint) = build_assignment_hint(&target, &rest) {
                return Some(hint);
            }
        }
    }

    None
}

fn parse_binary_op(text: &str, separators: &[&str]) -> Option<(String, String, Option<String>)> {
    let lower = text.to_lowercase();
    
    // First try with explicit separators like "and", "to", etc.
    for sep in separators {
        if let Some(idx) = lower.find(sep) {
            let left = text[..idx].trim().to_string();
            let rest = &text[idx + sep.len()..];
            
            // Check for target (store in, into, to, as)
            let (right, target) = extract_target(rest);
            
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right, target));
            }
        }
    }
    
    // Fallback: handle "add a b" pattern (space-separated operands)
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() >= 2 {
        let left = tokens[0].to_string();
        let (right, target) = extract_target(&tokens[1..].join(" "));
        if !left.is_empty() && !right.is_empty() {
            return Some((left, right, target));
        }
    }
    
    None
}

fn extract_target(text: &str) -> (String, Option<String>) {
    let lower = text.to_lowercase();
    for marker in [" into ", " store in ", " to ", " as ", " storing in "] {
        if let Some(idx) = lower.find(marker) {
            let value = text[..idx].trim().to_string();
            let target = text[idx + marker.len()..].trim().to_string();
            if !target.is_empty() {
                return (value, Some(target));
            }
        }
    }
    (text.trim().to_string(), None)
}

static EXPR_TOKEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[A-Za-z_][A-Za-z0-9_]*|\d+(?:\.\d+)?|==|!=|<=|>=|&&|\|\||[()+\-*/%<>]")
        .expect("valid expression token regex")
});
static BITFIELD_WIDTH_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(\d+)\s*bits?\b").expect("valid bitfield regex"));

fn normalize_expression_phrases(input: &str) -> String {
    let mut out = input.to_string();
    let replacements = [
        (r"(?i)\bsquare root of\b", "sqrt of"),
        (r"(?i)\bsquare root\b", "sqrt"),
        (r"(?i)\bopen paren\b", "("),
        (r"(?i)\bopen parenthesis\b", "("),
        (r"(?i)\bclose paren\b", ")"),
        (r"(?i)\bclose parenthesis\b", ")"),
        (r"(?i)\bopen bracket\b", "("),
        (r"(?i)\bclose bracket\b", ")"),
    ];

    for (pattern, replacement) in replacements {
        let re = Regex::new(pattern).expect("valid expression phrase regex");
        out = re.replace_all(&out, replacement).into_owned();
    }

    out
}

fn tokenize_expression(input: &str) -> Vec<String> {
    EXPR_TOKEN_RE
        .find_iter(input)
        .map(|m| m.as_str().to_string())
        .collect()
}

fn looks_like_expression(input: &str) -> bool {
    let normalized = normalize_expression_phrases(input);
    let tokens = tokenize_expression(&normalized);
    tokens.iter().any(|token| {
        let lower = token.to_lowercase();
        matches!(
            lower.as_str(),
            "add" | "subtract" | "multiply" | "divide" | "modulo" | "mod" | "remainder"
                | "plus" | "minus" | "times" | "over" | "and" | "or" | "not"
                | "squared" | "cubed" | "doubled" | "halved" | "negated"
                | "absolute" | "rounded" | "floor" | "ceiling"
        ) || matches!(
            token.as_str(),
            "+" | "-" | "*" | "/" | "%" | "&&" | "||" | "==" | "!=" | "<" | ">" | "<=" | ">="
        ) || lower == "of"
    })
}

fn parse_expression(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = normalize_expression_phrases(trimmed);
    let lower = normalized.to_lowercase();
    if let Some(idx) = lower.find(" to the power of ") {
        let base_part = normalized[..idx].trim();
        let exp_part = normalized[idx + " to the power of ".len()..].trim();
        if !base_part.is_empty() && !exp_part.is_empty() {
            let base = parse_expression(base_part).unwrap_or_else(|| base_part.to_string());
            let exponent = parse_expression(exp_part).unwrap_or_else(|| exp_part.to_string());
            return Some(format!("pow({}, {})", base, exponent));
        }
    }

    let tokens = tokenize_expression(&normalized);
    if tokens.is_empty() {
        return None;
    }

    let mut output: Vec<String> = Vec::new();
    let mut iter = tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        let lower = token.to_lowercase();
        match lower.as_str() {
            "add" | "plus" => output.push("+".to_string()),
            "subtract" | "minus" => output.push("-".to_string()),
            "multiply" | "times" => output.push("*".to_string()),
            "divide" | "over" => output.push("/".to_string()),
            "modulo" | "mod" | "remainder" => output.push("%".to_string()),
            "and" => output.push("&&".to_string()),
            "or" => output.push("||".to_string()),
            "not" => output.push("!".to_string()),
            "doubled" => {
                if let Some(prev) = output.pop() {
                    output.push(format!("({} * 2)", prev));
                } else if let Some(next) = iter.next() {
                    output.push(format!("(2 * {})", next));
                }
            }
            "halved" => {
                if let Some(prev) = output.pop() {
                    output.push(format!("({} / 2)", prev));
                } else if let Some(next) = iter.next() {
                    output.push(format!("({} / 2)", next));
                }
            }
            "negated" => {
                if let Some(prev) = output.pop() {
                    output.push(format!("(-{})", prev));
                } else if let Some(next) = iter.next() {
                    output.push(format!("(-{})", next));
                }
            }
            "absolute" => {
                if let Some(next) = iter.peek() {
                    if next != "of" {
                        let arg = iter.next().unwrap();
                        output.push(format!("abs({})", arg));
                    } else {
                        output.push("abs".to_string());
                    }
                } else {
                    output.push("abs".to_string());
                }
            }
            "rounded" => {
                if let Some(next) = iter.peek() {
                    if next != "of" {
                        let arg = iter.next().unwrap();
                        output.push(format!("round({})", arg));
                    } else {
                        output.push("round".to_string());
                    }
                } else {
                    output.push("round".to_string());
                }
            }
            "floor" => {
                if let Some(next) = iter.peek() {
                    if next != "of" {
                        let arg = iter.next().unwrap();
                        output.push(format!("floor({})", arg));
                    } else {
                        output.push("floor".to_string());
                    }
                } else {
                    output.push("floor".to_string());
                }
            }
            "ceiling" => {
                if let Some(next) = iter.peek() {
                    if next != "of" {
                        let arg = iter.next().unwrap();
                        output.push(format!("ceil({})", arg));
                    } else {
                        output.push("ceil".to_string());
                    }
                } else {
                    output.push("ceil".to_string());
                }
            }
            "squared" => {
                let prev = output.pop()?;
                output.push(format!("({} * {})", prev, prev));
            }
            "cubed" => {
                let prev = output.pop()?;
                output.push(format!("({} * {} * {})", prev, prev, prev));
            }
            "of" => {
                let func = output.pop()?;
                let arg = iter.next()?;
                output.push(format!("{}({})", func, arg));
            }
            _ => output.push(token),
        }
    }

    Some(output.join(" "))
}

fn normalize_assignment_value(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if looks_like_expression(trimmed) {
        if let Some(expr) = parse_expression(trimmed) {
            return expr;
        }
    }
    trimmed.to_string()
}

fn looks_like_comparison(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        " greater ", " less ", " equal ", "==", "!=", "<=", ">=", " < ", " > ",
    ]
    .iter()
    .any(|pat| lower.contains(pat))
}

fn split_on_marker(line: &str, markers: &[&str]) -> Option<(String, String)> {
    let lower = line.to_lowercase();
    for marker in markers {
        if let Some(idx) = lower.find(marker) {
            let left = line[..idx].trim().to_string();
            let right = line[idx + marker.len()..].trim().to_string();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn build_assignment_hint(target_part: &str, value_part: &str) -> Option<StatementHint> {
    let target = sanitize_identifier(target_part);
    let value = normalize_assignment_value(value_part);
    if target.is_empty() || value.is_empty() {
        return None;
    }
    Some(StatementHint::Assignment {
        targets: vec![target],
        value,
    })
}

fn build_declaration_hint(target_part: &str, value_part: &str) -> Option<StatementHint> {
    let target = sanitize_identifier(target_part);
    let value = normalize_assignment_value(value_part);
    if target.is_empty() || value.is_empty() {
        return None;
    }
    Some(StatementHint::Declaration {
        names: vec![target],
        type_hint: None,
        qualifiers: Vec::new(),
        initial_value: Some(value),
        is_array: false,
        array_size: None,
    })
}

fn try_extract_bitfield(line: &str) -> Option<StatementHint> {
    let width = BITFIELD_WIDTH_RE
        .captures(line)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))?;

    let rest = strip_any_keyword(line, DECLARE_KEYWORDS).unwrap_or(line);
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut type_hint: Option<String> = None;
    let mut name = "";

    for token in tokens {
        let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let lower = clean.to_lowercase();
        if clean.is_empty() {
            continue;
        }
        if lower == "bit" || lower == "bits" || lower == "bitfield" || lower == "field" || lower == "fields" {
            continue;
        }
        if clean == width {
            continue;
        }
        if QUALIFIER_WORDS.contains(&lower.as_str()) {
            continue;
        }
        if DECL_TYPE_WORDS.contains(&lower.as_str()) {
            let mapped = match lower.as_str() {
                "integers" => "int",
                "floats" => "float",
                "doubles" => "double",
                "chars" => "char",
                "strings" => "string",
                "bools" | "booleans" => "bool",
                other => other,
            };
            type_hint = Some(mapped.to_string());
            continue;
        }
        if clean.parse::<i32>().is_ok() {
            continue;
        }
        name = clean;
    }

    let name = sanitize_identifier(name);
    if name.is_empty() {
        return None;
    }

    Some(StatementHint::BitfieldDecl {
        name,
        type_hint,
        width,
    })
}

fn try_extract_assignment(line: &str) -> Option<StatementHint> {
    // Try "set" or "make" as assignment keywords
    let rest = strip_keyword(line, "set")
        .or_else(|| strip_keyword(line, "make"))?;
    
    // Skip if this looks like "make function" or "make main"
    let rest_lower = rest.to_lowercase();
    if rest_lower.starts_with("function") || rest_lower.starts_with("main") {
        return None;
    }
    
    // Try to parse: "target(s) [to] value"
    // Patterns:
    //   set a 5
    //   set a to 5
    //   set a, b, c 0
    //   set a, b 0
    //   set total a + b * 3
    
    let lower = rest.to_lowercase();
    
    // Check for "to" separator
    if let Some(to_idx) = lower.find(" to ") {
        let targets_part = &rest[..to_idx];
        let value = normalize_assignment_value(&rest[to_idx + 4..]);
        
        let targets: Vec<String> = targets_part
            .split(',')
            .map(|s| sanitize_identifier(s.trim()))
            .filter(|s| !s.is_empty())
            .collect();
        
        if !targets.is_empty() && !value.is_empty() {
            return Some(StatementHint::Assignment { targets, value });
        }
    }
    
    // No "to" - parse as "targets value" or "targets expression"
    // Split on last contiguous identifier/expression
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    
    // Find where targets end and value begins
    // Targets are comma-separated identifiers at the start
    let mut target_end_idx = 0;
    let mut targets = Vec::new();
    
    for (i, token) in tokens.iter().enumerate() {
        let clean = token.trim_matches(',');
        // If it's a valid identifier (possibly with comma), it's a target
        if is_valid_identifier_token(clean) {
            targets.push(sanitize_identifier(clean));
            target_end_idx = i + 1;
            // If this token doesn't have a trailing comma and next exists, 
            // the rest might be the value
            if !token.ends_with(',') && i + 1 < tokens.len() {
                break;
            }
        } else {
            break;
        }
    }
    
    if targets.is_empty() || target_end_idx >= tokens.len() {
        return None;
    }
    
    // Everything after targets is the value
    let value = normalize_assignment_value(&tokens[target_end_idx..].join(" "));
    
    if !value.is_empty() {
        return Some(StatementHint::Assignment { targets, value });
    }
    
    None
}

/// Handle patterns like "x = 5" without keywords.
fn try_extract_equals_assignment(line: &str) -> Option<StatementHint> {
    if !line.contains('=') {
        return None;
    }
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }
    let target = sanitize_identifier(parts[0]);
    let value = normalize_assignment_value(parts[1]);
    if target.is_empty() || value.is_empty() {
        return None;
    }
    Some(StatementHint::Assignment { targets: vec![target], value })
}

fn is_valid_identifier_token(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn try_extract_while(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "while")?;
    
    let lower = rest.to_lowercase();
    
    // Check if this is actually a for-loop style syntax: "while i 0 to 10"
    // If it contains " to " with numbers, treat it as a for loop instead
    if lower.contains(" to ") {
        // Check if the pattern looks like "iterator start to end"
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 4 {
            // Check if second token is a number or second-to-last is "to"
            if tokens.iter().any(|t| t.parse::<f64>().is_ok()) {
                // This looks like a for loop, not a while loop
                // Return None to let try_extract_loop handle it
                return None;
            }
        }
    }
    
    // Find "do" separator for body action
    let (condition_part, body_action) = if let Some(do_idx) = lower.find(" do ") {
        let cond = rest[..do_idx].trim();
        let action = rest[do_idx + 4..].trim();
        (cond, if action.is_empty() { None } else { Some(action.to_string()) })
    } else {
        (rest.trim(), None)
    };
    
    let condition = normalize_condition(condition_part);
    
    Some(StatementHint::While { condition, body_action })
}

fn try_extract_read(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "read")
        .or_else(|| strip_keyword(line, "input"))?;
    
    let variables: Vec<String> = rest
        .split(',')
        .map(|s| sanitize_identifier(s.trim()))
        .filter(|s| !s.is_empty())
        .collect();
    
    if variables.is_empty() {
        return None;
    }
    
    Some(StatementHint::Read { variables })
}

fn try_extract_vla_declaration(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    let rest = if lower.starts_with("variable length array") {
        strip_any_keyword(line, &["variable length array", "variable-length array"])?
    } else if lower.starts_with("vla ") {
        strip_keyword(line, "vla")?
    } else if lower.starts_with("array of ") {
        strip_keyword(line, "array of")?
    } else {
        return None;
    };

    let mut rest = rest.trim();
    if rest.starts_with("of ") {
        rest = rest[3..].trim();
    }

    let mut tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut size_token = tokens.remove(0);
    if size_token.eq_ignore_ascii_case("size") || size_token.eq_ignore_ascii_case("length") {
        size_token = tokens.get(0)?.to_owned();
        tokens.remove(0);
    }

    let size = sanitize_identifier(size_token);
    if size.is_empty() {
        return None;
    }

    let mut qualifiers: Vec<String> = Vec::new();
    let mut type_hint: Option<String> = None;
    let mut name: Option<String> = None;

    for token in tokens {
        let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let lower = clean.to_lowercase();
        if clean.is_empty() {
            continue;
        }
        if lower == "of" || lower == "array" || lower == "values" || lower == "elements" {
            continue;
        }
        if QUALIFIER_WORDS.contains(&lower.as_str()) {
            if !qualifiers.iter().any(|q| q == &lower) {
                qualifiers.push(lower);
            }
            continue;
        }
        if DECL_TYPE_WORDS.contains(&lower.as_str()) {
            let mapped = match lower.as_str() {
                "integers" => "int",
                "floats" => "float",
                "doubles" => "double",
                "chars" => "char",
                "strings" => "string",
                "bools" | "booleans" => "bool",
                other => other,
            };
            type_hint = Some(mapped.to_string());
            continue;
        }
        if name.is_none() {
            let ident = sanitize_identifier(clean);
            if !ident.is_empty() {
                name = Some(ident);
            }
        }
    }

    let name = name.unwrap_or_else(|| "arr".to_string());

    Some(StatementHint::Declaration {
        names: vec![name],
        type_hint,
        qualifiers,
        initial_value: None,
        is_array: true,
        array_size: Some(size),
    })
}

fn try_extract_qualified_declaration(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    let first = lower.split_whitespace().next()?;
    if !QUALIFIER_WORDS.contains(&first) && !DECL_TYPE_WORDS.contains(&first) {
        return None;
    }
    if lower.starts_with("struct ") || lower.starts_with("union ") || lower.contains(" function") {
        return None;
    }
    let is_array = LIST_KEYWORDS.iter().any(|kw| lower.contains(kw));
    let (names, type_hint, qualifiers, initial_value, array_size) =
        parse_declaration_parts(line, is_array);
    if names.is_empty() {
        return None;
    }
    Some(StatementHint::Declaration {
        names,
        type_hint,
        qualifiers,
        initial_value,
        is_array,
        array_size,
    })
}

fn try_extract_declaration(line: &str) -> Option<StatementHint> {
    // Check for list/array keywords first
    let is_array = LIST_KEYWORDS.iter().any(|kw| {
        let lower = line.to_lowercase();
        lower.contains(kw)
    });

    // Try to strip declaration keywords
    let rest = strip_any_keyword(line, DECLARE_KEYWORDS)
        .or_else(|| {
            // Also match lines starting with list keywords
            for kw in LIST_KEYWORDS {
                if let Some(r) = strip_keyword(line, kw) {
                    return Some(r);
                }
            }
            None
        })?;

    // Parse the declaration
    let (names, type_hint, qualifiers, initial_value, array_size) = parse_declaration_parts(rest, is_array);
    
    if names.is_empty() {
        return None;
    }

    Some(StatementHint::Declaration {
        names,
        type_hint,
        qualifiers,
        initial_value,
        is_array,
        array_size,
    })
}

fn parse_declaration_parts(
    text: &str,
    is_array: bool,
) -> (Vec<String>, Option<String>, Vec<String>, Option<String>, Option<String>) {
    let mut names = Vec::new();
    let mut type_hint = None;
    let mut qualifiers: Vec<String> = Vec::new();
    let mut initial_value = None;
    let mut array_size = None;
    let mut decl_text = text;
    let mut value_part: Option<String> = None;

    let lower_full = text.to_lowercase();
    for marker in [" equals ", " = ", " is ", " set to ", " set "] {
        if let Some(idx) = lower_full.find(marker) {
            decl_text = text[..idx].trim();
            let remainder = text[idx + marker.len()..].trim();
            if !remainder.is_empty() {
                value_part = Some(remainder.to_string());
            }
            break;
        }
    }

    // Noise words that are ONLY filtered when used as articles (before another word)
    // NOT filtered when they appear with commas or as standalone identifiers
    let article_noise = ["a", "an", "the"];
    // These are always filtered as they are connecting words
    let connecting_noise = ["of", "with", "named", "called", "as", "that", "is", "be", "to"];
    let tokens: Vec<&str> = decl_text.split_whitespace().collect();
    
    for (i, token) in tokens.iter().enumerate() {
        let clean = token.trim_matches(|c: char| c == ',' || c == ';');
        let lower = clean.to_lowercase();
        
        // Check if this token had a comma (indicating it's part of a list)
        let has_comma = token.contains(',');
        
        // Skip connecting noise words always
        if connecting_noise.contains(&lower.as_str()) {
            continue;
        }
        
        // Skip article noise words ONLY if:
        // - They don't have a comma attached (not part of a list like "a, b")
        // - They are followed by another word (acting as article)
        if article_noise.contains(&lower.as_str()) && !has_comma && i + 1 < tokens.len() {
            continue;
        }
        
        // Skip array keywords (already detected)
        if LIST_KEYWORDS.contains(&lower.as_str()) {
            continue;
        }
        
        // Detect qualifiers
        if QUALIFIER_WORDS.contains(&lower.as_str()) {
            if !qualifiers.iter().any(|q| q == &lower) {
                qualifiers.push(lower.clone());
            }
            continue;
        }

        // Detect type
        if DECL_TYPE_WORDS.contains(&lower.as_str()) {
            let mapped = match lower.as_str() {
                "integers" => "int",
                "floats" => "float",
                "doubles" => "double",
                "chars" => "char",
                "strings" => "string",
                "bools" | "booleans" => "bool",
                other => other,
            };
            type_hint = Some(mapped.to_string());
            continue;
        }
        
        // Check for numeric value (could be initial value or array size)
        if clean.parse::<f64>().is_ok() {
            if is_array && array_size.is_none() {
                array_size = Some(clean.to_string());
            } else if value_part.is_none() {
                initial_value = Some(clean.to_string());
            }
            continue;
        }
        
        // Otherwise it's likely a variable name
        let ident = sanitize_identifier(clean);
        if !ident.is_empty() {
            names.push(ident);
        }
    }

    if let Some(value) = value_part {
        let normalized = normalize_assignment_value(&value);
        if !normalized.is_empty() {
            initial_value = Some(normalized);
        }
    }

    (names, type_hint, qualifiers, initial_value, array_size)
}

fn try_extract_loop(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, LOOP_KEYWORDS)?;
    
    let lower = rest.to_lowercase();

    // "loop until <condition>" / "repeat until <condition>"
    if lower.starts_with("until ") {
        let condition_part = rest[6..].trim();
        if !condition_part.is_empty() {
            let condition = normalize_condition(condition_part);
            return Some(StatementHint::While {
                condition: format!("!({})", condition),
                body_action: None,
            });
        }
    }

    // "loop N times"
    if let Some(idx) = lower.rfind(" times") {
        let count_part = rest[..idx].trim();
        if !count_part.is_empty() {
            let count_expr = normalize_assignment_value(count_part);
            let end_expr = if let Ok(num) = count_expr.parse::<i64>() {
                (num.saturating_sub(1)).to_string()
            } else {
                format!("{} - 1", count_expr)
            };
            return Some(StatementHint::Loop {
                iterator: Some("i".to_string()),
                start: Some("0".to_string()),
                end: Some(end_expr),
                collection: None,
                body_action: None,
            });
        }
    }
    
    // Check for range loop (contains "to")
    if lower.contains(" to ") {
        let (iterator, start, end) = parse_range_loop(rest);
        let body_action = extract_loop_action(rest);
        
        return Some(StatementHint::Loop {
            iterator: Some(iterator),
            start: Some(start),
            end: Some(end),
            collection: None,
            body_action,
        });
    }
    
    // Check for collection loop (contains "in" or "over")
    for marker in [" in ", " over "] {
        if let Some(idx) = lower.find(marker) {
            let before = rest[..idx].trim();
            let after = rest[idx + marker.len()..].trim();
            
            let iterator = if before.is_empty() { "item".to_string() } else { sanitize_identifier(before) };
            let collection = sanitize_identifier(after.split_whitespace().next().unwrap_or(""));
            
            return Some(StatementHint::Loop {
                iterator: Some(iterator),
                start: None,
                end: None,
                collection: Some(collection),
                body_action: None,
            });
        }
    }
    
    // Generic loop without clear structure
    Some(StatementHint::Loop {
        iterator: Some("i".to_string()),
        start: None,
        end: None,
        collection: None,
        body_action: None,
    })
}

fn parse_range_loop(text: &str) -> (String, String, String) {
    let lower = text.to_lowercase();
    let mut iterator = "i".to_string();
    let mut start = "0".to_string();
    let mut end = "10".to_string();
    
    // Try to find "from X to Y" pattern: "i from 0 to 10"
    if let Some(from_idx) = lower.find("from ") {
        let after_from = &text[from_idx + 5..];
        if let Some(to_idx) = after_from.to_lowercase().find(" to ") {
            start = after_from[..to_idx].trim().to_string();
            let after_to = &after_from[to_idx + 4..];
            // End is everything until the next keyword or end
            end = after_to.split_whitespace().next().unwrap_or("10").to_string();
        }
        
        // Iterator is before "from"
        let before_from = text[..from_idx].trim();
        if !before_from.is_empty() {
            iterator = sanitize_identifier(before_from);
        }
    } else if let Some(to_idx) = lower.find(" to ") {
        // Pattern without "from": could be "i 0 to 10" or "0 to 10"
        let before = text[..to_idx].trim();
        let after = &text[to_idx + 4..];
        
        let tokens: Vec<&str> = before.split_whitespace().collect();
        
        match tokens.len() {
            0 => {}
            1 => {
                // Just "0 to 10" - no iterator, first token is start
                let tok = tokens[0];
                if tok.parse::<f64>().is_ok() {
                    start = tok.to_string();
                } else {
                    // It's an identifier, treat as iterator
                    iterator = sanitize_identifier(tok);
                }
            }
            2 => {
                // "i 0 to 10" - iterator and start
                let first = tokens[0];
                let second = tokens[1];
                
                if second.parse::<f64>().is_ok() || is_expression(second) {
                    // First is iterator, second is start
                    iterator = sanitize_identifier(first);
                    start = second.to_string();
                } else {
                    // Fallback: first is iterator, second might be expression
                    iterator = sanitize_identifier(first);
                    start = second.to_string();
                }
            }
            _ => {
                // Multiple tokens before "to"
                // First token is likely iterator, rest is start expression
                iterator = sanitize_identifier(tokens[0]);
                start = tokens[1..].join(" ");
            }
        }
        
        // Parse end - could be a single value or expression like "n-1" or "n - 1"
        let end_str = after.trim();
        // Take until we hit a loop action keyword
        let end_tokens: Vec<&str> = end_str.split_whitespace().collect();
        let mut end_parts = Vec::new();
        for tok in end_tokens {
            let tok_lower = tok.to_lowercase();
            if ["do", "print", "printing", "then"].contains(&tok_lower.as_str()) {
                break;
            }
            end_parts.push(tok);
        }
        if !end_parts.is_empty() {
            end = end_parts.join(" ");
        }
    }
    
    (iterator, start, end)
}

fn is_expression(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '+' | '-' | '*' | '/' | '(' | ')'))
}

fn extract_loop_action(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for marker in ["printing ", "print "] {
        if let Some(idx) = lower.find(marker) {
            let action = text[idx + marker.len()..].trim();
            if !action.is_empty() {
                return Some(format!("print {}", action));
            }
        }
    }
    None
}

fn try_extract_conditional(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_keyword(line, "unless") {
        let (condition, then_action, else_action) = parse_conditional_parts(rest);
        return Some(StatementHint::Conditional {
            condition: format!("!({})", condition),
            then_action,
            else_action,
            is_else_if: false,
        });
    }

    let rest = strip_any_keyword(line, IF_KEYWORDS)?;
    let lower_rest = rest.to_lowercase();
    if lower_rest.contains(" else if ") {
        let mut else_action = None;
        let mut main = rest.to_string();
        if let Some(idx) = lower_rest.find(" else ") {
            else_action = Some(rest[idx + 6..].trim().to_string());
            main = rest[..idx].to_string();
        }
        let parts: Vec<&str> = main.split(" else if ").collect();
        let mut conditions = Vec::new();
        for part in parts {
            let mut seg = part.trim();
            let seg_lower = seg.to_lowercase();
            if seg_lower.starts_with("if ") {
                seg = seg.get(3..).unwrap_or(seg);
            }
            let (cond, then_action, _) = parse_conditional_parts(seg);
            let action = then_action.unwrap_or_else(|| "pass".to_string());
            conditions.push((cond, action));
        }
        return Some(StatementHint::ConditionalChain { conditions, else_action });
    }

    let (condition, then_action, else_action) = parse_conditional_parts(rest);
    
    Some(StatementHint::Conditional {
        condition,
        then_action,
        else_action,
        is_else_if: false,
    })
}

fn parse_conditional_parts(text: &str) -> (String, Option<String>, Option<String>) {
    let mut main = text;
    let mut else_action = None;
    let mut then_action = None;
    
    let lower = text.to_lowercase();
    
    // Extract else action
    for marker in [" otherwise ", " else "] {
        if let Some(idx) = lower.find(marker) {
            else_action = Some(text[idx + marker.len()..].trim().to_string());
            main = &text[..idx];
            break;
        }
    }
    
    // Extract then action - try explicit separators first
    let main_lower = main.to_lowercase();
    for marker in [" then ", " do "] {
        if let Some(idx) = main_lower.find(marker) {
            then_action = Some(main[idx + marker.len()..].trim().to_string());
            main = &main[..idx];
            break;
        }
    }
    
    // If no explicit separator, look for action keywords after a comparison
    // Pattern: "a <= b print a" -> condition="a <= b", then_action="print a"
    if then_action.is_none() {
        if let Some((cond, action)) = split_condition_from_action(main) {
            main = cond;
            then_action = Some(action.to_string());
        }
    }
    
    let condition = normalize_condition(main.trim());
    (condition, then_action, else_action)
}

/// Action keywords that indicate the start of an inline action
const ACTION_KEYWORDS: &[&str] = &[
    "print", "return", "set", "make", "declare", "increment", "decrement",
    "add", "subtract", "multiply", "divide", "call", "break", "continue",
];

/// Split a condition from an inline action when no explicit separator exists
/// Example: "a <= b print a" -> ("a <= b", "print a")
fn split_condition_from_action(text: &str) -> Option<(&str, &str)> {
    let lower = text.to_lowercase();
    
    // Find action keywords directly in the text
    for keyword in ACTION_KEYWORDS {
        // Pattern: look for " keyword " or " keyword" at end
        let patterns = [
            format!(" {} ", keyword),
            format!(" {}", keyword),
        ];
        
        for pattern in &patterns {
            if let Some(idx) = lower.find(pattern) {
                // Make sure this is after a comparison operator
                let before = &lower[..idx];
                let has_comparison = [" <= ", " >= ", " < ", " > ", " == ", " != "]
                    .iter()
                    .any(|op| before.contains(op));
                
                if has_comparison {
                    let condition = text[..idx].trim();
                    let action = text[idx..].trim();
                    if !condition.is_empty() && !action.is_empty() {
                        return Some((condition, action));
                    }
                }
            }
        }
    }
    
    None
}

fn try_extract_function(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "function")
        .or_else(|| strip_keyword(line, "create function"))
        .or_else(|| strip_keyword(line, "make function"))
        .or_else(|| strip_keyword(line, "define function"))?;
    
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    
    let name = sanitize_identifier(tokens[0]);
    if name.is_empty() {
        return None;
    }
    
    // Try to parse parameters and return type
    let (parameters, return_type) = parse_function_signature(&tokens[1..]);
    
    Some(StatementHint::FunctionDef {
        name,
        parameters,
        return_type,
    })
}

fn parse_function_signature(tokens: &[&str]) -> (Vec<(String, String)>, Option<String>) {
    let mut parameters = Vec::new();
    let mut return_type = None;
    
    let type_map = [
        ("int", "int"), ("integer", "int"), ("integers", "int"),
        ("float", "float"), ("floats", "float"),
        ("double", "double"), ("doubles", "double"),
        ("char", "char"), ("chars", "char"),
        ("string", "char *"), ("strings", "char *"),
        ("bool", "bool"), ("boolean", "bool"),
        ("void", "void"),
        ("long", "long"), ("short", "short"),
        ("unsigned", "unsigned"), ("signed", "signed"),
        ("size_t", "size_t"),
        ("longlong", "long long"), ("unsignedlong", "unsigned long"),
    ];
    
    let noise = ["taking", "with", "and", "parameters", "parameter", "returning", "returns"];
    
    let mut i = 0;
    let mut in_return = false;
    
    while i < tokens.len() {
        let token = tokens[i].trim_matches(|c: char| c == ',' || c == ';');
        let lower = token.to_lowercase();
        
        if lower == "returning" || lower == "returns" {
            in_return = true;
            i += 1;
            continue;
        }
        
        if noise.contains(&lower.as_str()) {
            i += 1;
            continue;
        }
        
        // Check if it's a type
        if let Some((_, mapped)) = type_map.iter().find(|(k, _)| *k == lower.as_str()) {
            if in_return {
                return_type = Some(mapped.to_string());
            } else if i + 1 < tokens.len() {
                // Next token should be the parameter name
                let name = sanitize_identifier(tokens[i + 1]);
                if !name.is_empty() {
                    parameters.push((mapped.to_string(), name));
                    i += 1;
                }
            }
        }
        
        i += 1;
    }
    
    (parameters, return_type)
}

fn try_extract_anonymous_struct_union(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    for (kw, is_union) in [
        ("anonymous struct", false),
        ("unnamed struct", false),
        ("anonymous union", true),
        ("unnamed union", true),
    ] {
        if lower.starts_with(kw) {
            let rest = strip_keyword(line, kw)?;
            let lower_rest = rest.to_lowercase();
            let mut parent: Option<String> = None;
            let mut fields_part = rest;

            if lower_rest.starts_with("inside ") {
                let after = rest[7..].trim();
                let lower_after = after.to_lowercase();
                if let Some(idx) = lower_after.find(" with ") {
                    let parent_name = sanitize_identifier(after[..idx].trim());
                    if !parent_name.is_empty() {
                        parent = Some(parent_name);
                    }
                    fields_part = &after[idx + 6..];
                } else {
                    let parent_name = sanitize_identifier(after);
                    if !parent_name.is_empty() {
                        parent = Some(parent_name);
                    }
                    fields_part = "";
                }
            } else if let Some(idx) = lower_rest.find(" with ") {
                fields_part = &rest[idx + 6..];
            }

            let fields = parse_struct_fields(fields_part);
            return Some(if is_union {
                StatementHint::AnonymousUnion { parent, fields }
            } else {
                StatementHint::AnonymousStruct { parent, fields }
            });
        }
    }

    None
}

fn try_extract_struct(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "struct")?;
    
    let lower = rest.to_lowercase();
    let mut name_part = rest;
    let mut fields_part = "";
    
    for marker in [" with ", " having ", " containing "] {
        if let Some(idx) = lower.find(marker) {
            name_part = &rest[..idx];
            fields_part = &rest[idx + marker.len()..];
            break;
        }
    }
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let fields = parse_struct_fields(fields_part);
    
    Some(StatementHint::StructDef { name, fields })
}

fn parse_struct_fields(text: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    
    let type_map = [
        ("int", "int"), ("integer", "int"),
        ("float", "float"), ("double", "double"),
        ("char", "char"), ("string", "char *"),
        ("bool", "bool"),
        ("long", "long"), ("short", "short"),
        ("unsigned", "unsigned"), ("signed", "signed"),
    ];
    
    for chunk in text.split(|c| c == ',' || c == ';').flat_map(|s| s.split(" and ")) {
        let tokens: Vec<&str> = chunk.split_whitespace().collect();
        let bit_width = BITFIELD_WIDTH_RE
            .captures(chunk)
            .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()));
        
        let mut field_type = "int";
        let mut field_name = "";
        
        for token in tokens.iter().rev() {
            let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let lower = clean.to_lowercase();

            if lower == "bit" || lower == "bits" || lower == "bitfield" || lower == "field" || lower == "fields" {
                continue;
            }
            if let Some(width) = &bit_width {
                if clean == width {
                    continue;
                }
            }
            
            if let Some((_, mapped)) = type_map.iter().find(|(k, _)| *k == lower.as_str()) {
                field_type = mapped;
            } else if !clean.is_empty() {
                field_name = clean;
                break;
            }
        }
        
        let name = sanitize_identifier(field_name);
        if !name.is_empty() {
            let final_name = if let Some(width) = bit_width {
                format!("{} : {}", name, width)
            } else {
                name
            };
            fields.push((field_type.to_string(), final_name));
        }
    }
    
    fields
}

fn sanitize_identifier(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .last()
        .unwrap_or(input)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

const GREATER_PATTERNS: &[&str] = &[
    "is greater than",
    "greater than",
    "is bigger than",
    "bigger than",
    "is larger than",
    "larger than",
    "is more than",
    "more than",
    "is higher than",
    "higher than",
    "is above",
    "above",
    "is over",
    "over",
    "exceeds",
    "is exceeding",
    "exceeding",
    "is greater",
    "greater",
    "is bigger",
    "bigger",
    "is larger",
    "larger",
    "is of a bigger size than",
    "is of a larger size than",
    "is of greater size than",
    "is of greater value than",
    "has more than",
    "with more than",
    "is beyond",
    "beyond",
    "is of higher value than",
];

const LESS_PATTERNS: &[&str] = &[
    "is less than",
    "less than",
    "is smaller than",
    "smaller than",
    "is lower than",
    "lower than",
    "is fewer than",
    "fewer than",
    "is below",
    "below",
    "is under",
    "under",
    "is tinier than",
    "tinier than",
    "is beneath",
    "beneath",
    "is less",
    "less",
    "is smaller",
    "smaller",
    "is lower",
    "lower",
    "is of a smaller size than",
    "is of lesser value than",
    "has less than",
    "with less than",
    "is not as much as",
];

const GE_PATTERNS: &[&str] = &[
    "is greater than or equal to",
    "greater than or equal to",
    "is at least",
    "at least",
    "is no less than",
    "no less than",
    "is not less than",
    "not less than",
    "is bigger than or equal to",
    "bigger than or equal to",
    "is larger than or equal to",
    "larger than or equal to",
    "is more than or equal to",
    "more than or equal to",
    "is greater or equal to",
    "greater or equal to",
    ">=",
];

const LE_PATTERNS: &[&str] = &[
    "is less than or equal to",
    "less than or equal to",
    "is at most",
    "at most",
    "is no more than",
    "no more than",
    "is not greater than",
    "not greater than",
    "is smaller than or equal to",
    "smaller than or equal to",
    "is lower than or equal to",
    "lower than or equal to",
    "is fewer than or equal to",
    "fewer than or equal to",
    "<=",
];

const EQ_PATTERNS: &[&str] = &[
    "equals",
    "is equal to",
    "equal to",
    "is same as",
    "same as",
    "is equivalent to",
    "equivalent to",
    "is identical to",
    "identical to",
    "matches",
    "has the same value as",
    "is the same as",
    "approximately",
    "approximately equal to",
    "approx equal to",
    "about equal to",
    "==",
];

const NE_PATTERNS: &[&str] = &[
    "is not equal to",
    "is not equal",
    "is not the same as",
    "not the same as",
    "is different from",
    "different from",
    "differs from",
    "is unlike",
    "unlike",
    "does not equal",
    "doesn't equal",
    "does not match",
    "is not",
    "isnt",
    "isn't",
    "!=",
    "<>",
];

fn build_condition_regex(pattern: &str) -> Regex {
    let trimmed = pattern.trim();
    let has_symbol = trimmed
        .chars()
        .any(|c| !c.is_alphanumeric() && !c.is_whitespace());
    let escaped = regex::escape(trimmed);
    let pat = if has_symbol {
        format!(r"(?i)\s*{}\s*", escaped)
    } else {
        format!(r"(?i)\b{}\b", escaped)
    };
    Regex::new(&pat).expect("valid condition pattern")
}

fn normalize_condition(condition: &str) -> String {
    static REPLACERS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        let mut v: Vec<(Regex, &'static str)> = Vec::new();

        for pat in GE_PATTERNS {
            v.push((build_condition_regex(pat), " >= "));
        }
        for pat in LE_PATTERNS {
            v.push((build_condition_regex(pat), " <= "));
        }
        for pat in NE_PATTERNS {
            v.push((build_condition_regex(pat), " != "));
        }
        for pat in EQ_PATTERNS {
            v.push((build_condition_regex(pat), " == "));
        }
        for pat in GREATER_PATTERNS {
            v.push((build_condition_regex(pat), " > "));
        }
        for pat in LESS_PATTERNS {
            v.push((build_condition_regex(pat), " < "));
        }

        v
    });

    let mut out = condition.to_string();
    // Handle "x between a and b" before generic replacements
    let between_re = Regex::new(r"(?i)\b([A-Za-z_][A-Za-z0-9_]*)\s+between\s+([^\s]+)\s+and\s+([^\s]+)\b")
        .expect("valid between regex");
    out = between_re
        .replace_all(&out, "$1 >= $2 && $1 <= $3")
        .into_owned();
    for (re, replacement) in REPLACERS.iter() {
        out = re.replace_all(&out, *replacement).into_owned();
    }
    out
}

// ============================================================================
// New Extractors
// ============================================================================

fn try_extract_switch(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "switch")?;
    let expression = rest.trim().to_string();
    if expression.is_empty() {
        return None;
    }
    Some(StatementHint::Switch { expression })
}

fn try_extract_case(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "case")?;
    let lower = rest.to_lowercase();
    
    // Check for "do" action
    let (value_part, action) = if let Some(idx) = lower.find(" do ") {
        (&rest[..idx], Some(rest[idx + 4..].trim().to_string()))
    } else {
        (rest, None)
    };
    
    let value = value_part.trim().to_string();
    if value.is_empty() {
        return None;
    }
    
    Some(StatementHint::Case { value, action })
}

fn try_extract_default(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if !lower.starts_with("default") {
        return None;
    }
    
    let rest = &line.trim()[7..]; // Skip "default"
    let action = if rest.is_empty() {
        None
    } else if rest.to_lowercase().starts_with(" do ") {
        Some(rest[4..].trim().to_string())
    } else {
        None
    };
    
    Some(StatementHint::Default { action })
}

fn try_extract_do_while(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "do" alone starts a do-while
    if lower == "do" {
        return Some(StatementHint::DoWhileStart);
    }
    
    // "end do while condition" ends it
    if lower.starts_with("end do while ") {
        let condition = normalize_condition(&line.trim()[13..]);
        return Some(StatementHint::DoWhileEnd { condition });
    }
    
    None
}

fn try_extract_break_continue(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    if lower == "break" || lower == "break out" || lower.starts_with("break out of") {
        return Some(StatementHint::Break);
    }
    
    if lower == "continue" || lower == "skip iteration" || lower == "next iteration" {
        return Some(StatementHint::Continue);
    }
    
    None
}

fn try_extract_function_call(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_any_keyword(line, &["call", "apply", "invoke"]) {
        let lower = rest.to_lowercase();
        let (name_part, args_part) = if let Some(idx) = lower.find(" with ") {
            (&rest[..idx], Some(&rest[idx + 6..]))
        } else if let Some(idx) = lower.find(" on ") {
            (&rest[..idx], Some(&rest[idx + 4..]))
        } else if let Some(idx) = lower.find(" to ") {
            (&rest[..idx], Some(&rest[idx + 4..]))
        } else {
            (rest, None)
        };
        
        let name = sanitize_identifier(name_part.trim());
        if name.is_empty() {
            return None;
        }
        
        let arguments = if let Some(args) = args_part {
            if args.contains(',') {
                args.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                args.split(" and ").map(|s| s.trim().to_string()).collect()
            }
        } else {
            Vec::new()
        };
        
        return Some(StatementHint::FunctionCall { name, arguments });
    }

    // "<func> of <arg>" pattern (e.g., "sqrt of x")
    if let Some((name_part, arg_part)) = split_on_marker(line, &[" of "]) {
        if name_part.split_whitespace().count() != 1 {
            return None;
        }
        let lower_name = name_part.to_lowercase();
        let reserved = [
            "length", "size", "array", "element", "string", "struct", "union", "enum", "pointer",
        ];
        if reserved.iter().any(|w| *w == lower_name) {
            return None;
        }
        let name = sanitize_identifier(name_part.trim());
        if name.is_empty() {
            return None;
        }
        let arguments = if arg_part.contains(',') {
            arg_part.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            vec![arg_part.trim().to_string()]
        };
        return Some(StatementHint::FunctionCall { name, arguments });
    }

    None
}

fn try_extract_include(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "include")?;
    let header = rest.trim();
    
    if header.is_empty() {
        return None;
    }
    
    // Check if user specified quotes (local include)
    let (header, is_system) = if header.starts_with('"') && header.ends_with('"') {
        (header[1..header.len()-1].to_string(), false)
    } else {
        // Map common header names
        let mapped = match header.to_lowercase().as_str() {
            "stdio" => "stdio.h",
            "stdlib" => "stdlib.h",
            "string" => "string.h",
            "math" => "math.h",
            "stdbool" => "stdbool.h",
            "stdint" => "stdint.h",
            "ctype" => "ctype.h",
            "time" => "time.h",
            "assert" => "assert.h",
            _ => header,
        };
        (mapped.to_string(), true)
    };
    
    Some(StatementHint::Include { header, is_system })
}

fn try_extract_define(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "define")?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    
    if tokens.len() < 2 {
        return None;
    }
    
    let name = tokens[0].to_string();
    let value = tokens[1..].join(" ");
    
    Some(StatementHint::Define { name, value })
}

fn try_extract_pointer(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "pointer to int x" or "int pointer x"
    if let Some(rest) = strip_keyword(line, "pointer to") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let mut idx = 0;
            // Skip leading "to" if present ("pointer to pointer to int pp" leaves a stray "to")
            if tokens[0].eq_ignore_ascii_case("to") {
                idx += 1;
            }
            match (tokens.get(idx), tokens.get(idx + 1)) {
                (Some(ty), Some(name_tok)) => {
                    let name = sanitize_identifier(name_tok);
                    let base_type = (*ty).to_string();
                    if !name.is_empty() {
                        return Some(StatementHint::PointerDecl { base_type, name });
                    }
                }
                // Only one token → treat as name, default type to int
                (Some(name_tok), None) => {
                    let name = sanitize_identifier(name_tok);
                    if !name.is_empty() {
                        return Some(StatementHint::PointerDecl { base_type: "int".to_string(), name });
                    }
                }
                _ => {}
            }
        }
    }
    
    // "int pointer p" pattern
    for ty in ["int", "float", "double", "char", "void"] {
        if lower.starts_with(&format!("{} pointer ", ty)) {
            let rest = &line.trim()[ty.len() + 9..];
            let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or(""));
            if !name.is_empty() {
                return Some(StatementHint::PointerDecl { base_type: ty.to_string(), name });
            }
        }
    }
    
    // "dereference p"
    if let Some(rest) = strip_keyword(line, "dereference") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::Dereference { target });
        }
    }

    // "value at p" or "what p points to"
    if let Some(rest) = strip_keyword(line, "value at") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::Dereference { target });
        }
    }
    if lower.starts_with("what ") && lower.contains(" points to") {
        let rest = line.trim()[5..].trim();
        let name = rest.trim_end_matches(" points to").trim();
        let target = sanitize_identifier(name);
        if !target.is_empty() {
            return Some(StatementHint::Dereference { target });
        }
    }
    
    // "address of x"
    if let Some(rest) = strip_keyword(line, "address of") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::AddressOf { target });
        }
    }

    // "where x is stored"
    if lower.starts_with("where ") && lower.contains(" is stored") {
        let rest = line.trim()[6..].trim();
        let name = rest.trim_end_matches(" is stored").trim();
        let target = sanitize_identifier(name);
        if !target.is_empty() {
            return Some(StatementHint::AddressOf { target });
        }
    }
    
    None
}

fn try_extract_memory_ops(line: &str) -> Option<StatementHint> {
    let _lower = line.trim().to_lowercase();
    
    // "free p"
    if let Some(rest) = strip_keyword(line, "free") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::Free { target });
        }
    }
    
    // "allocate n integers" or "allocate memory for x"
    if let Some(rest) = strip_keyword(line, "allocate") {
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" bytes") {
            let count = rest[..idx].trim().to_string();
            if !count.is_empty() {
                return Some(StatementHint::Malloc { count, element_type: "char".to_string() });
            }
        }
        if let Some(idx) = lower_rest.find(" space for ") {
            let after = rest[idx + 11..].trim();
            let tokens: Vec<&str> = after.split_whitespace().collect();
            if tokens.len() >= 2 {
                let count = tokens[0].to_string();
                let element_type = tokens[1].trim_end_matches('s').to_string();
                return Some(StatementHint::Malloc { count, element_type });
            }
        }
        if let Some(idx) = lower_rest.find(" memory for ") {
            let after = rest[idx + 12..].trim();
            let tokens: Vec<&str> = after.split_whitespace().collect();
            if tokens.len() >= 2 {
                let count = tokens[0].to_string();
                let element_type = tokens[1].trim_end_matches('s').to_string();
                return Some(StatementHint::Malloc { count, element_type });
            }
        }
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            let count = tokens[0].to_string();
            let element_type = tokens[1].trim_end_matches('s').to_string(); // "integers" -> "integer"
            return Some(StatementHint::Malloc { count, element_type });
        }
    }
    
    // "malloc n integers"
    if let Some(rest) = strip_keyword(line, "malloc") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let count = tokens[0].to_string();
            let element_type = if tokens.len() > 1 { tokens[1].to_string() } else { "int".to_string() };
            return Some(StatementHint::Malloc { count, element_type });
        }
    }
    
    None
}

fn try_extract_enum(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "enum")?;
    
    let lower = rest.to_lowercase();
    let (name_part, values_part) = if let Some(idx) = lower.find(" with ") {
        (&rest[..idx], Some(&rest[idx + 6..]))
    } else {
        (rest, None)
    };
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let values: Vec<String> = if let Some(vals) = values_part {
        vals.split(',')
            .flat_map(|s| s.split(" and "))
            .map(|s| sanitize_identifier(s.trim()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    
    Some(StatementHint::EnumDef { name, values })
}

fn try_extract_typedef(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "typedef")?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    
    if tokens.len() < 2 {
        return None;
    }
    
    let original_type = tokens[0].to_string();
    let new_name = sanitize_identifier(tokens[1]);
    
    if new_name.is_empty() {
        return None;
    }
    
    Some(StatementHint::Typedef { original_type, new_name })
}

fn try_extract_string_decl(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "string")?;
    
    let lower = rest.to_lowercase();
    let (name_part, value_part) = if let Some(idx) = lower.find(" equals ") {
        (&rest[..idx], Some(&rest[idx + 8..]))
    } else if let Some(idx) = lower.find(" = ") {
        (&rest[..idx], Some(&rest[idx + 3..]))
    } else {
        (rest, None)
    };
    
    let name = sanitize_identifier(name_part.trim());
    if name.is_empty() {
        return None;
    }
    
    let initial_value = value_part.map(|v| v.trim().trim_matches('"').to_string());
    
    Some(StatementHint::StringDecl { name, initial_value, size: None })
}

fn try_extract_comment(line: &str) -> Option<StatementHint> {
    // "comment: ..." or "note: ..."
    if let Some(rest) = strip_keyword(line, "comment") {
        let text = rest.trim_start_matches(':').trim().to_string();
        return Some(StatementHint::Comment { text, is_block: false });
    }
    
    if let Some(rest) = strip_keyword(line, "note") {
        let text = rest.trim_start_matches(':').trim().to_string();
        return Some(StatementHint::Comment { text, is_block: false });
    }
    
    if line.trim().to_lowercase() == "block comment start" {
        return Some(StatementHint::Comment { text: String::new(), is_block: true });
    }
    
    None
}

fn try_extract_bitwise(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "x bitwise and y"
    if lower.contains(" bitwise and ") {
        let idx = lower.find(" bitwise and ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 13..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::And, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.contains(" bitwise or ") {
        let idx = lower.find(" bitwise or ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 12..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Or, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.contains(" xor ") {
        let idx = lower.find(" xor ").unwrap();
        let left = line.trim()[..idx].trim().to_string();
        let right = line.trim()[idx + 5..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Xor, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    // "shift x left by n"
    if lower.starts_with("shift ") && lower.contains(" left by ") {
        let rest = &line.trim()[6..];
        let idx = rest.to_lowercase().find(" left by ").unwrap();
        let left = rest[..idx].trim().to_string();
        let right = rest[idx + 9..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::ShiftLeft, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    if lower.starts_with("shift ") && lower.contains(" right by ") {
        let rest = &line.trim()[6..];
        let idx = rest.to_lowercase().find(" right by ").unwrap();
        let left = rest[..idx].trim().to_string();
        let right = rest[idx + 10..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::ShiftRight, 
            left, 
            right: Some(right),
            target: None 
        });
    }
    
    // "not x" (bitwise NOT)
    if lower.starts_with("bitwise not ") {
        let target = line.trim()[12..].trim().to_string();
        return Some(StatementHint::Bitwise { 
            operation: BitwiseOp::Not, 
            left: target, 
            right: None,
            target: None 
        });
    }
    
    None
}

fn try_extract_cast(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "cast x to int" or "x as int"
    if let Some(rest) = strip_keyword(line, "cast") {
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" to ") {
            let expression = rest[..idx].trim().to_string();
            let target_type = rest[idx + 4..].trim().to_string();
            if !expression.is_empty() && !target_type.is_empty() {
                return Some(StatementHint::Cast { expression, target_type });
            }
        }
    }
    
    // "x as int" pattern
    if lower.contains(" as ") && !lower.contains(" equals ") {
        let idx = lower.find(" as ").unwrap();
        let expression = line.trim()[..idx].trim().to_string();
        let target_type = line.trim()[idx + 4..].trim().to_string();
        if !expression.is_empty() && !target_type.is_empty() {
            return Some(StatementHint::Cast { expression, target_type });
        }
    }
    
    None
}

fn ordinal_to_index(word: &str) -> Option<String> {
    match word.to_lowercase().as_str() {
        "first" | "1st" => Some("0".to_string()),
        "second" | "2nd" => Some("1".to_string()),
        "third" | "3rd" => Some("2".to_string()),
        "fourth" | "4th" => Some("3".to_string()),
        "fifth" | "5th" => Some("4".to_string()),
        "sixth" | "6th" => Some("5".to_string()),
        "seventh" | "7th" => Some("6".to_string()),
        "eighth" | "8th" => Some("7".to_string()),
        "ninth" | "9th" => Some("8".to_string()),
        "tenth" | "10th" => Some("9".to_string()),
        "last" | "final" | "end" => Some("last".to_string()),
        _ => None,
    }
}

fn try_extract_array_access(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();

    // "<ordinal> element of <array>" (e.g., "third element of numbers")
    if let Some(idx) = lower.find(" element of ") {
        let ordinal_part = line.trim()[..idx].trim();
        let array_part = line.trim()[idx + " element of ".len()..].trim();
        let index = ordinal_to_index(ordinal_part).unwrap_or_else(|| ordinal_part.to_string());
        let array = sanitize_identifier(array_part);
        if !array.is_empty() && !index.is_empty() {
            return Some(StatementHint::ArrayAccess { array, index, value: None });
        }
    }

    // "element <i> of <array>"
    if lower.starts_with("element ") && lower.contains(" of ") {
        let rest = &line.trim()[8..];
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" of ") {
            let index_part = rest[..idx].trim();
            let array_part = rest[idx + 4..].trim();
            let index = ordinal_to_index(index_part).unwrap_or_else(|| index_part.to_string());
            let array = sanitize_identifier(array_part);
            if !array.is_empty() && !index.is_empty() {
                return Some(StatementHint::ArrayAccess { array, index, value: None });
            }
        }
    }
    
    // "get a at i"
    if let Some(rest) = strip_keyword(line, "get") {
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" at ") {
            let array = sanitize_identifier(&rest[..idx]);
            let index = rest[idx + 4..].trim().to_string();
            if !array.is_empty() && !index.is_empty() {
                return Some(StatementHint::ArrayAccess { array, index, value: None });
            }
        }
    }
    
    // "array at i"
    if lower.contains(" at ") && !lower.starts_with("set ") {
        if let Some(idx) = lower.find(" at ") {
            let array = sanitize_identifier(&line.trim()[..idx]);
            let index_part = line.trim()[idx + 4..].trim();
            let index = ordinal_to_index(index_part).unwrap_or_else(|| index_part.to_string());
            if !array.is_empty() && !index.is_empty() {
                return Some(StatementHint::ArrayAccess { array, index, value: None });
            }
        }
    }

    // "array sub i"
    if lower.contains(" sub ") {
        if let Some(idx) = lower.find(" sub ") {
            let array = sanitize_identifier(&line.trim()[..idx]);
            let index_part = line.trim()[idx + 5..].trim();
            let index = ordinal_to_index(index_part).unwrap_or_else(|| index_part.to_string());
            if !array.is_empty() && !index.is_empty() {
                return Some(StatementHint::ArrayAccess { array, index, value: None });
            }
        }
    }

    // "set a at 0 to 5"
    if lower.starts_with("set ") && lower.contains(" at ") && lower.contains(" to ") {
        let rest = &line.trim()[4..];
        let lower_rest = rest.to_lowercase();
        if let Some(at_idx) = lower_rest.find(" at ") {
            if let Some(to_idx) = lower_rest.find(" to ") {
                if at_idx < to_idx {
                    let array = sanitize_identifier(&rest[..at_idx]);
                    let index = rest[at_idx + 4..to_idx].trim().to_string();
                    let value = rest[to_idx + 4..].trim().to_string();
                    if !array.is_empty() && !index.is_empty() && !value.is_empty() {
                        return Some(StatementHint::ArrayAccess { array, index, value: Some(value) });
                    }
                }
            }
        }
    }
    
    None
}

fn try_extract_sizeof(line: &str) -> Option<StatementHint> {
    // "size of a" or "sizeof a"
    if let Some(rest) = strip_keyword(line, "size of") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::SizeOf { target });
        }
    }
    
    if let Some(rest) = strip_keyword(line, "sizeof") {
        let target = sanitize_identifier(rest.trim());
        if !target.is_empty() {
            return Some(StatementHint::SizeOf { target });
        }
    }
    
    None
}

fn try_extract_ternary(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    
    // "x when condition else y" → condition ? x : y
    if lower.contains(" when ") && lower.contains(" else ") {
        let when_idx = lower.find(" when ").unwrap();
        let else_idx = lower.find(" else ").unwrap();
        
        if when_idx < else_idx {
            let true_value = line.trim()[..when_idx].trim().to_string();
            let condition = line.trim()[when_idx + 6..else_idx].trim().to_string();
            let false_value = line.trim()[else_idx + 6..].trim().to_string();
            
            if !condition.is_empty() && !true_value.is_empty() && !false_value.is_empty() {
                return Some(StatementHint::Ternary { condition, true_value, false_value });
            }
        }
    }
    
    None
}

/// Heuristic: two-token numeric declaration like "a 0" -> int a = 0
fn try_extract_bare_declaration(line: &str) -> Option<StatementHint> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() != 2 {
        return None;
    }
    let name = sanitize_identifier(tokens[0]);
    if name.is_empty() {
        return None;
    }
    // Require second token to look numeric
    if !tokens[1].parse::<f64>().is_ok() {
        return None;
    }
    Some(StatementHint::Declaration {
        names: vec![name],
        type_hint: Some("int".to_string()),
        qualifiers: Vec::new(),
        initial_value: Some(tokens[1].to_string()),
        is_array: false,
        array_size: None,
    })
}

// ============================================================================
// New extractors (compound ops, logical, preprocessor, memory, arrays, structs,
// control flow, functions, file IO, strings, stdlib, errors)
// ============================================================================

fn try_extract_compound_assign(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();

    // Patterns like "add 5 to x" / "increase x by 5"
    if lower.starts_with("add ") || lower.starts_with("increase ") {
        let rest = line.splitn(2, ' ').nth(1)?.trim();
        if let Some((value, target)) = split_value_target(rest, &[" to ", " into ", " in "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::AddAssign,
                value,
            });
        }
        if let Some((target, value)) = split_target_value(rest, &[" by "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::AddAssign,
                value,
            });
        }
    }

    if lower.starts_with("subtract ") || lower.starts_with("decrease ") {
        let rest = line.splitn(2, ' ').nth(1)?.trim();
        // "subtract 5 from x"
        if let Some(idx) = rest.to_lowercase().find(" from ") {
            let value = rest[..idx].trim().to_string();
            let target = sanitize_identifier(rest[idx + 6..].trim());
            if !target.is_empty() && !value.is_empty() {
                return Some(StatementHint::CompoundAssign {
                    target,
                    operator: CompoundOp::SubAssign,
                    value,
                });
            }
        }
        if let Some((target, value)) = split_target_value(rest, &[" by "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::SubAssign,
                value,
            });
        }
    }

    if lower.starts_with("multiply ") {
        let rest = line.splitn(2, ' ').nth(1)?.trim();
        if let Some((target, value)) = split_target_value(rest, &[" by "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::MulAssign,
                value,
            });
        }
    }

    if lower.starts_with("divide ") {
        let rest = line.splitn(2, ' ').nth(1)?.trim();
        if let Some((target, value)) = split_target_value(rest, &[" by "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::DivAssign,
                value,
            });
        }
    }

    if lower.starts_with("mod ") || lower.starts_with("modulo ") {
        let rest = line.splitn(2, ' ').nth(1)?.trim();
        if let Some((target, value)) = split_target_value(rest, &[" by ", " with "]) {
            return Some(StatementHint::CompoundAssign {
                target,
                operator: CompoundOp::ModAssign,
                value,
            });
        }
    }

    // Bitwise compound
    for (kw, op) in [
        ("shift", CompoundOp::ShlAssign),
        ("bitwise and", CompoundOp::AndAssign),
        ("bitwise or", CompoundOp::OrAssign),
        ("bitwise xor", CompoundOp::XorAssign),
    ] {
        if lower.contains(kw) && lower.contains(" equals ") {
            let parts: Vec<&str> = lower.split(" equals ").collect();
            if parts.len() == 2 {
                let target = sanitize_identifier(parts[0]);
                let value = parts[1].trim().to_string();
                if !target.is_empty() && !value.is_empty() {
                    return Some(StatementHint::CompoundAssign { target, operator: op, value });
                }
            }
        }
    }

    // Operators like "+=", "-=", "*=", "/=", "%="
    for (marker, op) in [
        ("+=", CompoundOp::AddAssign),
        ("-=", CompoundOp::SubAssign),
        ("*=", CompoundOp::MulAssign),
        ("/=", CompoundOp::DivAssign),
        ("%=", CompoundOp::ModAssign),
        ("<<=", CompoundOp::ShlAssign),
        (">>=", CompoundOp::ShrAssign),
        ("&=", CompoundOp::AndAssign),
        ("|=", CompoundOp::OrAssign),
        ("^=", CompoundOp::XorAssign),
    ] {
        if let Some(idx) = line.find(marker) {
            let target = sanitize_identifier(line[..idx].trim());
            let value = line[idx + marker.len()..].trim().to_string();
            if !target.is_empty() && !value.is_empty() {
                return Some(StatementHint::CompoundAssign { target, operator: op, value });
            }
        }
    }

    None
}

fn split_value_target(text: &str, separators: &[&str]) -> Option<(String, String)> {
    for sep in separators {
        if let Some(idx) = text.to_lowercase().find(sep) {
            let value = text[..idx].trim().to_string();
            let target = sanitize_identifier(text[idx + sep.len()..].trim());
            if !value.is_empty() && !target.is_empty() {
                return Some((value, target));
            }
        }
    }
    None
}

fn split_target_value(text: &str, separators: &[&str]) -> Option<(String, String)> {
    for sep in separators {
        if let Some(idx) = text.to_lowercase().find(sep) {
            let target = sanitize_identifier(text[..idx].trim());
            let value = text[idx + sep.len()..].trim().to_string();
            if !target.is_empty() && !value.is_empty() {
                return Some((target, value));
            }
        }
    }
    None
}

fn try_extract_logical(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();

    // not X
    for kw in LOGICAL_NOT_KEYWORDS {
        if lower.starts_with(kw) {
            let expr = line[kw.len()..].trim();
            if !expr.is_empty() {
                return Some(StatementHint::Logical {
                    operation: LogicalOp::Not,
                    left: expr.to_string(),
                    right: None,
                    target: None,
                });
            }
        }
    }

    for kw in LOGICAL_AND_KEYWORDS {
        if let Some(idx) = lower.find(&format!(" {} ", kw)) {
            let left = line[..idx].trim().to_string();
            let right = line[idx + kw.len() + 2..].trim().to_string();
            if !left.is_empty() && !right.is_empty() {
                return Some(StatementHint::Logical {
                    operation: LogicalOp::And,
                    left,
                    right: Some(right),
                    target: None,
                });
            }
        }
    }

    for kw in LOGICAL_OR_KEYWORDS {
        if let Some(idx) = lower.find(&format!(" {} ", kw)) {
            let left = line[..idx].trim().to_string();
            let right = line[idx + kw.len() + 2..].trim().to_string();
            if !left.is_empty() && !right.is_empty() {
                return Some(StatementHint::Logical {
                    operation: LogicalOp::Or,
                    left,
                    right: Some(right),
                    target: None,
                });
            }
        }
    }

    None
}

fn try_extract_static_assert(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    let rest = if lower.starts_with("static assert") {
        strip_keyword(line, "static assert")?
    } else if lower.starts_with("compile time assert") {
        strip_keyword(line, "compile time assert")?
    } else if lower.starts_with("compile-time assert") {
        strip_keyword(line, "compile-time assert")?
    } else {
        return None;
    };

    let mut condition = rest.trim().to_string();
    let mut message = None;
    if let Some((left, right)) = split_on_marker(&condition, &[" message ", " with message ", " saying "]) {
        condition = left;
        let msg = right.trim().trim_matches('"').to_string();
        if !msg.is_empty() {
            message = Some(msg);
        }
    }

    let normalized = normalize_condition(&condition);
    if normalized.trim().is_empty() {
        return None;
    }

    Some(StatementHint::StaticAssert {
        condition: normalized.trim().to_string(),
        message,
    })
}

fn try_extract_comma_expression(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    let rest = if lower.starts_with("sequence ") {
        strip_keyword(line, "sequence")?
    } else if lower.starts_with("comma operator") {
        strip_keyword(line, "comma operator")?
    } else if lower.starts_with("comma expression") {
        strip_keyword(line, "comma expression")?
    } else {
        return None;
    };

    let mut parts: Vec<String> = Vec::new();
    for chunk in rest
        .split(|c| c == ',' || c == ';')
        .flat_map(|s| s.split(" and then "))
        .flat_map(|s| s.split(" then "))
    {
        let piece = chunk.trim();
        if piece.is_empty() {
            continue;
        }
        if let Some((left, right)) = split_on_marker(piece, &[" equals ", " = ", " is "]) {
            let target = sanitize_identifier(&left);
            let value = normalize_assignment_value(&right);
            if !target.is_empty() && !value.is_empty() {
                parts.push(format!("{} = {}", target, value));
                continue;
            }
        }
        let normalized = normalize_assignment_value(piece);
        if normalized.is_empty() {
            parts.push(piece.to_string());
        } else {
            parts.push(normalized);
        }
    }

    if parts.len() < 2 {
        return None;
    }

    Some(StatementHint::CommaExpression { expressions: parts })
}

fn try_extract_preprocessor(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();

    if let Some(rest) = strip_any_keyword(line, PRE_IFDEF_KEYWORDS) {
        let symbol = sanitize_identifier(rest);
        if !symbol.is_empty() {
            return Some(StatementHint::IfDef { symbol });
        }
    }

    if let Some(rest) = strip_any_keyword(line, PRE_IFNDEF_KEYWORDS) {
        let symbol = sanitize_identifier(rest);
        if !symbol.is_empty() {
            return Some(StatementHint::IfNDef { symbol });
        }
    }

    if PRE_ENDIF_KEYWORDS.iter().any(|k| lower.starts_with(k)) {
        return Some(StatementHint::EndIf);
    }

    if let Some(rest) = strip_any_keyword(line, PRE_UNDEF_KEYWORDS) {
        let symbol = sanitize_identifier(rest);
        if !symbol.is_empty() {
            return Some(StatementHint::Undef { symbol });
        }
    }

    if let Some(rest) = strip_any_keyword(line, PRE_PRAGMA_KEYWORDS) {
        let value = rest.trim();
        if !value.is_empty() {
            return Some(StatementHint::Pragma { value: value.to_string() });
        }
    }

    // Macro functions: "define macro ADD x y as x + y"
    if let Some(rest) = strip_any_keyword(line, PRE_MACRO_KEYWORDS) {
        let lower_rest = rest.to_lowercase();
        if let Some(as_idx) = lower_rest.find(" as ") {
            let head = rest[..as_idx].trim();
            let body = rest[as_idx + 4..].trim().to_string();
            let tokens: Vec<&str> = head.split_whitespace().collect();
            if !tokens.is_empty() {
                let name = sanitize_identifier(tokens[0]);
                let params: Vec<String> = tokens.iter().skip(1).map(|p| sanitize_identifier(p)).filter(|p| !p.is_empty()).collect();
                if !name.is_empty() && !body.is_empty() {
                    return Some(StatementHint::MacroFunction { name, params, body });
                }
            }
        }
    }

    None
}

fn try_extract_advanced_memory(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();

    if lower.starts_with("reallocate ") || lower.starts_with("realloc ") {
        let rest = strip_any_keyword(line, &["reallocate", "realloc"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            let pointer = sanitize_identifier(tokens[0]);
            let count = tokens[2].to_string();
            let element_type = tokens.get(3).cloned().unwrap_or("int").to_string();
            if !pointer.is_empty() {
                return Some(StatementHint::Realloc { pointer, count, element_type });
            }
        }
    }

    if lower.starts_with("allocate and zero") || lower.starts_with("calloc") {
        let rest = strip_any_keyword(line, &["allocate and zero", "calloc"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let count = tokens[0].to_string();
            let element_type = tokens.get(1).cloned().unwrap_or("int").trim_end_matches('s').to_string();
            return Some(StatementHint::Calloc { count, element_type, target: None });
        }
    }

    if lower.contains("pointer to pointer") {
        let rest = strip_keyword(line, "pointer to pointer")?;
        let mut tokens: Vec<&str> = rest.split_whitespace().collect();
        // Skip leading "to" if present ("pointer to pointer to int pp")
        if !tokens.is_empty() && tokens[0].eq_ignore_ascii_case("to") {
            tokens.remove(0);
        }
        if tokens.len() >= 2 {
            let base_type = tokens[0].to_string();
            let name = sanitize_identifier(tokens[1]);
            if !name.is_empty() {
                return Some(StatementHint::DoublePointer { base_type, name });
            }
        } else if tokens.len() == 1 {
            // No explicit base type: default to int
            let name = sanitize_identifier(tokens[0]);
            if !name.is_empty() {
                return Some(StatementHint::DoublePointer { base_type: "int".to_string(), name });
            }
        }
    }

    if lower.starts_with("function pointer") {
        let rest = strip_keyword(line, "function pointer")?;
        let mut tokens = rest.split_whitespace();
        let mut return_type = tokens.next().unwrap_or("int").to_string();
        if return_type == "returning" {
            return_type = tokens.next().unwrap_or("int").to_string();
        }
        let name = sanitize_identifier(tokens.next().unwrap_or("fp"));
        let params: Vec<String> = tokens.map(|t| sanitize_identifier(t)).filter(|t| !t.is_empty()).collect();
        if !name.is_empty() {
            return Some(StatementHint::FunctionPointer { return_type, name, params });
        }
    }

    if lower.contains("move pointer") || lower.contains("advance pointer") {
        let rest = strip_any_keyword(line, &["move pointer", "advance pointer"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            let pointer = sanitize_identifier(tokens[0]);
            let direction_word = tokens[1].to_lowercase();
            let offset = tokens[2].to_string();
            let direction = if direction_word.starts_with("back") || direction_word.starts_with("prev") {
                PointerDir::Backward
            } else {
                PointerDir::Forward
            };
            if !pointer.is_empty() && !offset.is_empty() {
                return Some(StatementHint::PointerArithmetic { pointer, offset, direction });
            }
        }
    }

    if lower.starts_with("set ") && lower.contains(" to null") {
        let rest = strip_keyword(line, "set")?;
        let target = sanitize_identifier(rest.trim_end_matches("to null").trim());
        if !target.is_empty() {
            return Some(StatementHint::NullAssign { target });
        }
    }

    None
}

fn try_extract_compound_literal(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();

    let (is_array, rest) = if lower.starts_with("anonymous array") {
        (true, strip_keyword(line, "anonymous array")?)
    } else if lower.starts_with("inline array") {
        (true, strip_keyword(line, "inline array")?)
    } else if lower.starts_with("compound array") {
        (true, strip_keyword(line, "compound array")?)
    } else if lower.starts_with("array literal") {
        (true, strip_keyword(line, "array literal")?)
    } else if lower.starts_with("anonymous ") || lower.starts_with("inline ") || lower.starts_with("compound literal ") {
        (false, strip_any_keyword(line, &["anonymous", "inline", "compound literal"])?)
    } else {
        return None;
    };

    if is_array {
        let lower_rest = rest.to_lowercase();
        let mut type_hint: Option<String> = None;
        let mut values_part = rest.trim();
        if let Some(idx) = lower_rest.find(" of ") {
            let prefix = rest[..idx].trim();
            values_part = rest[idx + 4..].trim();
            for token in prefix.split_whitespace() {
                let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                let lower_tok = clean.to_lowercase();
                if DECL_TYPE_WORDS.contains(&lower_tok.as_str()) {
                    type_hint = Some(lower_tok);
                    break;
                }
            }
        }

        let values: Vec<String> = values_part
            .split(|c| c == ',' || c == ';')
            .flat_map(|s| s.split_whitespace())
            .filter(|v| !v.is_empty() && *v != "values")
            .map(|v| v.to_string())
            .collect();

        if values.is_empty() {
            return None;
        }

        let ty = type_hint.unwrap_or_else(|| "int".to_string());
        return Some(StatementHint::CompoundLiteral {
            type_hint: ty,
            values,
            fields: Vec::new(),
            is_array: true,
        });
    }

    if lower.starts_with("inline function") {
        return None;
    }

    let rest = rest.trim();
    let lower_rest = rest.to_lowercase();
    let mut type_hint = "";
    let mut fields_part = "";
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    type_hint = tokens[0];
    fields_part = rest[type_hint.len()..].trim();
    if lower_rest.starts_with("struct ") {
        let inner = strip_keyword(rest, "struct")?;
        let inner_tokens: Vec<&str> = inner.split_whitespace().collect();
        if inner_tokens.is_empty() {
            return None;
        }
        type_hint = inner_tokens[0];
        fields_part = inner[type_hint.len()..].trim();
    }

    if fields_part.starts_with("with ") {
        fields_part = fields_part[5..].trim();
    } else if fields_part.starts_with("fields ") {
        fields_part = fields_part[7..].trim();
    }

    let mut fields = parse_struct_init_fields(fields_part);
    if fields.is_empty() {
        let remainder_tokens: Vec<&str> = fields_part.split_whitespace().collect();
        if remainder_tokens.len() >= 2 && remainder_tokens.len() % 2 == 0 {
            let mut i = 0;
            while i + 1 < remainder_tokens.len() {
                let name = sanitize_identifier(remainder_tokens[i]);
                let value = remainder_tokens[i + 1].to_string();
                if !name.is_empty() && !value.is_empty() {
                    fields.push((name, value));
                }
                i += 2;
            }
        }
    }

    if fields.is_empty() {
        return None;
    }

    Some(StatementHint::CompoundLiteral {
        type_hint: sanitize_identifier(type_hint),
        values: Vec::new(),
        fields,
        is_array: false,
    })
}

fn try_extract_designated_init(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if !lower.contains("index") || !lower.contains("array") {
        return None;
    }

    let rest = if lower.starts_with("initialize array") {
        strip_keyword(line, "initialize array")?
    } else if lower.starts_with("init array") {
        strip_keyword(line, "init array")?
    } else if lower.starts_with("array ") {
        strip_keyword(line, "array")?
    } else {
        line
    };

    let lower_rest = rest.to_lowercase();
    let mut type_hint: Option<String> = None;
    let mut name: Option<String> = None;
    let mut fields_part = rest;
    if let Some(idx) = lower_rest.find(" with ") {
        let prefix = rest[..idx].trim();
        fields_part = rest[idx + 6..].trim();
        for token in prefix.split_whitespace() {
            let clean = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            let lower_tok = clean.to_lowercase();
            if clean.is_empty() || lower_tok == "array" || lower_tok == "of" {
                continue;
            }
            if DECL_TYPE_WORDS.contains(&lower_tok.as_str()) {
                type_hint = Some(lower_tok);
                continue;
            }
            let ident = sanitize_identifier(clean);
            if !ident.is_empty() {
                name = Some(ident);
            }
        }
    }

    let name = name.unwrap_or_else(|| "arr".to_string());
    let mut designators = Vec::new();
    let source = fields_part;
    for segment in source.split("index").skip(1) {
        let seg = segment.trim();
        if let Some((idx_part, value_part)) = split_on_marker(seg, &[" equals ", " = ", " is "]) {
            let idx = sanitize_identifier(&idx_part);
            let mut value = value_part.trim();
            if let Some((first, _)) = value.split_once(" and ") {
                value = first.trim();
            }
            if let Some((first, _)) = value.split_once(',') {
                value = first.trim();
            }
            let normalized = normalize_assignment_value(value);
            if !idx.is_empty() && !normalized.is_empty() {
                designators.push((format!("[{}]", idx), normalized));
            }
        }
    }

    if designators.is_empty() {
        return None;
    }

    Some(StatementHint::DesignatedInit {
        type_hint,
        name,
        designators,
    })
}

fn try_extract_multi_array(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.contains(" array ") && lower.contains(" by ") {
        // e.g., "2d array arr 10 by 20" or "array matrix 3 by 4"
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let name = tokens.iter().find(|t| !t.to_lowercase().contains("array") && t.chars().all(|c| c.is_ascii_alphanumeric() || c=='_'))?;
        let mut dims = Vec::new();
        let mut iter = tokens.iter().peekable();
        while let Some(tok) = iter.next() {
            if tok.to_lowercase() == "by" {
                if let Some(dim) = iter.next() {
                    dims.push(dim.to_string());
                }
            } else if tok.parse::<i32>().is_ok() && iter.peek().map(|x| x.to_lowercase()=="by").unwrap_or(false) {
                dims.push(tok.to_string());
            }
        }
        if !dims.is_empty() {
            return Some(StatementHint::MultiArrayDecl {
                type_hint: None,
                name: sanitize_identifier(name),
                dimensions: dims,
            });
        }
    }

    // array init: "array a with values 1 2 3"
    if lower.starts_with("array ") && lower.contains(" with ") {
        let rest = strip_keyword(line, "array")?;
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" with ") {
            let name = sanitize_identifier(rest[..idx].trim());
            let values: Vec<String> = rest[idx + 6..].split(|c| c==',' || c==' ').map(|v| v.trim()).filter(|v| !v.is_empty() && v != &"values").map(|v| v.to_string()).collect();
            if !name.is_empty() && !values.is_empty() {
                return Some(StatementHint::ArrayInit { type_hint: None, name, values });
            }
        }
    }

    // multidim access: "get matrix at i j", "set matrix at i j to v"
    if lower.starts_with("get ") || lower.starts_with("set ") {
        let is_set = lower.starts_with("set ");
        let rest = if is_set { &line[4..] } else { &line[4..] };
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" at ") {
            let array = sanitize_identifier(rest[..idx].trim());
            let after = &rest[idx + 4..];
            let parts: Vec<&str> = after.split_whitespace().collect();
            if parts.len() >= 1 {
                let mut indices = Vec::new();
                for p in &parts {
                    if *p == "to" {
                        break;
                    }
                    indices.push(p.to_string());
                }
                let value = if is_set {
                    if let Some(to_idx) = lower_rest.find(" to ") {
                        Some(rest[to_idx + 4..].trim().to_string())
                    } else { None }
                } else { None };
                if !array.is_empty() && !indices.is_empty() {
                    return Some(StatementHint::MultiDimAccess { array, indices, value });
                }
            }
        }
    }

    None
}

fn try_extract_struct_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();

    if lower.contains(" dot ") {
        let parts: Vec<&str> = line.splitn(2, " dot ").collect();
        if parts.len() == 2 {
            let object = sanitize_identifier(parts[0]);
            let field = sanitize_identifier(parts[1]);
            if !object.is_empty() && !field.is_empty() {
                return Some(StatementHint::StructAccess { object, field });
            }
        }
    }

    if lower.contains(" arrow ") || lower.contains(" -> ") {
        if let Some(idx) = lower.find("->") {
            let object = sanitize_identifier(line[..idx].trim());
            let field = sanitize_identifier(line[idx+2..].trim());
            if !object.is_empty() && !field.is_empty() {
                return Some(StatementHint::StructArrow { pointer: object, field });
            }
        } else if let Some(idx) = lower.find(" arrow ") {
            let object = sanitize_identifier(line[..idx].trim());
            let field = sanitize_identifier(line[idx+7..].trim());
            if !object.is_empty() && !field.is_empty() {
                return Some(StatementHint::StructArrow { pointer: object, field });
            }
        }
    }

    if lower.starts_with("initialize ") || lower.starts_with("init ") {
        let rest = strip_any_keyword(line, &["initialize", "init"])?;
        let lower_rest = rest.to_lowercase();
        let mut struct_name = "";
        let mut var_name = "";
        let mut fields_part = "";
        if let Some(idx) = lower_rest.find(" with ") {
            struct_name = rest[..idx].trim();
            fields_part = &rest[idx + 6..];
            let name_tokens: Vec<&str> = struct_name.split_whitespace().collect();
            if name_tokens.len() >= 2 {
                struct_name = name_tokens[0];
                var_name = name_tokens[1];
            }
        } else {
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            if tokens.len() >= 2 {
                struct_name = tokens[0];
                var_name = tokens[1];
            }
        }
        let fields = parse_struct_init_fields(fields_part);
        let struct_name = sanitize_identifier(struct_name);
        let var_name = sanitize_identifier(var_name);
        if !struct_name.is_empty() && !var_name.is_empty() {
            return Some(StatementHint::StructInit { struct_name, var_name, fields });
        }
    }

    if lower.starts_with("union ") {
        let rest = strip_keyword(line, "union")?;
        let lower_rest = rest.to_lowercase();
        let mut name_part = rest;
        let mut fields_part = "";
        if let Some(idx) = lower_rest.find(" with ") {
            name_part = &rest[..idx];
            fields_part = &rest[idx + 6..];
        }
        let name = sanitize_identifier(name_part.trim());
        let fields = parse_struct_fields(fields_part);
        if !name.is_empty() {
            return Some(StatementHint::UnionDef { name, fields });
        }
    }

    if lower.starts_with("array of ") && lower.contains(" structs") {
        let rest = &line[9..];
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            let size = tokens[0].to_string();
            let struct_name = sanitize_identifier(tokens[2]);
            let var_name = if tokens.len() > 3 { sanitize_identifier(tokens[3]) } else { format!("{}_arr", struct_name) };
            if !struct_name.is_empty() {
                return Some(StatementHint::StructArray { struct_name, var_name, size });
            }
        }
    }

    None
}

fn parse_struct_init_fields(text: &str) -> Vec<(String, String)> {
    text.split(|c| c == ',' || c == ';')
        .flat_map(|chunk| chunk.split(" and "))
        .filter_map(|pair| {
            let tokens: Vec<&str> = pair.trim().split_whitespace().collect();
            if tokens.len() >= 2 {
                let name = sanitize_identifier(tokens[0]);
                let value = tokens[1..].join(" ");
                if !name.is_empty() && !value.is_empty() {
                    return Some((name, value));
                }
            }
            None
        })
        .collect()
}

fn try_extract_control_flow_ext(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower.starts_with("goto ") {
        let label = sanitize_identifier(&line[5..]);
        if !label.is_empty() {
            return Some(StatementHint::Goto { label });
        }
    }
    if lower.starts_with("label ") {
        let name = sanitize_identifier(&line[6..]);
        if !name.is_empty() {
            return Some(StatementHint::Label { name });
        }
    }
    if lower.contains("infinite loop") {
        return Some(StatementHint::InfiniteLoop);
    }
    if lower.contains("loop forever") || lower.contains("forever loop") {
        return Some(StatementHint::ForEver);
    }
    None
}

fn try_extract_function_ext(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("declare function") || lower.starts_with("function prototype") {
        let rest = strip_any_keyword(line, &["declare function", "function prototype"])?;
        let (params, ret) = parse_function_signature(&rest.split_whitespace().skip(1).collect::<Vec<&str>>());
        let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or("func"));
        if !name.is_empty() {
            return Some(StatementHint::FunctionPrototype { name, parameters: params, return_type: ret });
        }
    }

    for qualifier in ["inline", "static", "extern"] {
        if lower.starts_with(&format!("{} function", qualifier)) {
            let rest = line[qualifier.len() + 9..].trim();
            let tokens: Vec<&str> = rest.split_whitespace().collect();
            if !tokens.is_empty() {
                let name = sanitize_identifier(tokens[0]);
                let (params, ret) = parse_function_signature(&tokens[1..]);
                return Some(StatementHint::QualifiedFunction {
                    qualifier: qualifier.to_string(),
                    name,
                    parameters: params,
                    return_type: ret,
                });
            }
        }
    }

    None
}

fn try_extract_file_io(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("open file") {
        let rest = line[9..].trim();
        let mut parts = rest.split(" for ");
        let path_part = parts.next()?.trim();
        let mode_part = parts.next().unwrap_or("r");
        let mut into_var = "fp".to_string();
        if let Some(idx) = mode_part.to_lowercase().find(" into ") {
            let mode = mode_part[..idx].trim().to_string();
            into_var = sanitize_identifier(mode_part[idx + 6..].trim());
            return Some(StatementHint::FileOpen { var_name: into_var, path: path_part.trim_matches('"').to_string(), mode });
        }
        return Some(StatementHint::FileOpen { var_name: into_var, path: path_part.trim_matches('"').to_string(), mode: mode_part.to_string() });
    } else if lower.starts_with("close file") {
        let var_name = sanitize_identifier(line[10..].trim());
        if !var_name.is_empty() {
            return Some(StatementHint::FileClose { var_name });
        }
    } else if lower.starts_with("read from") {
        let rest = line[9..].trim();
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 4 {
            let var_name = sanitize_identifier(tokens[0]);
            let buffer = sanitize_identifier(tokens[2]);
            let size = tokens[3].to_string();
            return Some(StatementHint::FileRead { var_name, buffer, size });
        }
    } else if lower.starts_with("write") && lower.contains(" to ") {
        let rest = line[5..].trim();
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" to ") {
            let buffer = sanitize_identifier(rest[..idx].trim());
            let file = sanitize_identifier(rest[idx + 4..].trim());
            if !buffer.is_empty() && !file.is_empty() {
                return Some(StatementHint::FileWrite { var_name: file, buffer, size: "n".to_string() });
            }
        }
    } else if lower.starts_with("read line from") {
        let rest = line[14..].trim();
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 4 {
            let var_name = sanitize_identifier(tokens[0]);
            let buffer = sanitize_identifier(tokens[2]);
            let size = tokens[3].to_string();
            return Some(StatementHint::Fgets { buffer, size, var_name });
        }
    } else if lower.starts_with("write string") {
        let rest = line[12..].trim();
        if let Some(idx) = rest.to_lowercase().find(" to ") {
            let content = rest[..idx].trim().trim_matches('"').to_string();
            let var_name = sanitize_identifier(rest[idx + 4..].trim());
            return Some(StatementHint::Fputs { content, var_name });
        }
    } else if lower.starts_with("print to file") || lower.starts_with("fprintf") {
        let rest = strip_any_keyword(line, &["print to file", "fprintf"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let var_name = sanitize_identifier(tokens[0]);
            let format = rest.splitn(2, ' ').nth(1).unwrap_or("").to_string();
            return Some(StatementHint::Fprintf { var_name, format, args: Vec::new() });
        }
    } else if lower.starts_with("read from file") || lower.starts_with("fscanf") {
        let rest = strip_any_keyword(line, &["read from file", "fscanf"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let var_name = sanitize_identifier(tokens[0]);
            let args: Vec<String> = tokens.iter().skip(1).map(|s| sanitize_identifier(s)).filter(|s| !s.is_empty()).collect();
            return Some(StatementHint::Fscanf { var_name, args });
        }
    }
    None
}

fn try_extract_string_funcs(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("copy string") || lower.starts_with("strcpy") {
        let rest = strip_any_keyword(line, &["copy string", "strcpy"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            return Some(StatementHint::Strcpy { dest: sanitize_identifier(tokens[1]), src: sanitize_identifier(tokens[0]) });
        }
    }
    if lower.starts_with("copy ") && lower.contains(" to ") && !lower.contains(" chars from") {
        let rest = strip_keyword(line, "copy")?.trim();
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" to ") {
            let src = sanitize_identifier(rest[..idx].trim());
            let dest = sanitize_identifier(rest[idx + 4..].trim());
            if !src.is_empty() && !dest.is_empty() {
                return Some(StatementHint::Strcpy { dest, src });
            }
        }
    }
    if lower.starts_with("copy") && lower.contains(" chars from") {
        let rest = line[4..].trim();
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 4 {
            return Some(StatementHint::Strncpy {
                dest: sanitize_identifier(tokens[3]),
                src: sanitize_identifier(tokens[1]),
                count: tokens[0].to_string(),
            });
        }
    }
    if lower.starts_with("concatenate") || lower.starts_with("strcat") {
        let rest = strip_any_keyword(line, &["concatenate", "strcat"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            return Some(StatementHint::Strcat { dest: sanitize_identifier(tokens[1]), src: sanitize_identifier(tokens[0]) });
        }
    }
    if lower.starts_with("append ") {
        let rest = strip_keyword(line, "append")?.trim();
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" to ") {
            let src = sanitize_identifier(rest[..idx].trim());
            let dest = sanitize_identifier(rest[idx + 4..].trim());
            if !src.is_empty() && !dest.is_empty() {
                return Some(StatementHint::Strcat { dest, src });
            }
        }
    }
    if lower.starts_with("compare strings") || lower.starts_with("strcmp") {
        let rest = strip_any_keyword(line, &["compare strings", "strcmp"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            return Some(StatementHint::Strcmp {
                left: sanitize_identifier(tokens[0]),
                right: sanitize_identifier(tokens[1]),
                target: None,
            });
        }
    }
    if lower.starts_with("compare ") && lower.contains(" and ") {
        let rest = strip_keyword(line, "compare")?.trim();
        let parts: Vec<&str> = rest.split(" and ").collect();
        if parts.len() >= 2 {
            let left = sanitize_identifier(parts[0].trim());
            let right = sanitize_identifier(parts[1].trim());
            if !left.is_empty() && !right.is_empty() {
                return Some(StatementHint::Strcmp { left, right, target: None });
            }
        }
    }
    if lower.starts_with("length of string") || lower.starts_with("strlen") {
        let rest = strip_any_keyword(line, &["length of string", "strlen"])?;
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Strlen { target, store_in: None });
        }
    }
    if lower.starts_with("length of ") {
        let rest = strip_keyword(line, "length of")?.trim();
        let target = sanitize_identifier(rest);
        if !target.is_empty() {
            return Some(StatementHint::Strlen { target, store_in: None });
        }
    }
    if lower.starts_with("format into string") || lower.starts_with("sprintf") {
        let rest = strip_any_keyword(line, &["format into string", "sprintf"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let buffer = sanitize_identifier(tokens[0]);
            let format = rest.splitn(2, ' ').nth(1).unwrap_or("").to_string();
            return Some(StatementHint::Sprintf { buffer, format, args: Vec::new() });
        }
    }
    None
}

fn try_extract_stdlib_funcs(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("copy bytes") || lower.starts_with("memcpy") {
        let rest = strip_any_keyword(line, &["copy bytes", "memcpy"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            return Some(StatementHint::Memcpy {
                dest: sanitize_identifier(tokens[2]),
                src: sanitize_identifier(tokens[1]),
                size: tokens[0].to_string(),
            });
        }
    }
    if lower.starts_with("set bytes") || lower.starts_with("memset") {
        let rest = strip_any_keyword(line, &["set bytes", "memset"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            return Some(StatementHint::Memset {
                dest: sanitize_identifier(tokens[0]),
                value: tokens[1].to_string(),
                size: tokens[2].to_string(),
            });
        }
    }
    if lower.starts_with("exit program") || lower.starts_with("exit with") {
        let code = line.split_whitespace().last().unwrap_or("0").to_string();
        return Some(StatementHint::Exit { code });
    }
    if lower.starts_with("random number") || lower.starts_with("rand") {
        return Some(StatementHint::Rand { store_in: None });
    }

    for (kw, kind) in [
        ("square root of", MathFuncKind::Sqrt),
        ("sqrt", MathFuncKind::Sqrt),
        ("power", MathFuncKind::Pow),
        ("pow", MathFuncKind::Pow),
        ("absolute value", MathFuncKind::Abs),
        ("abs", MathFuncKind::Abs),
        ("sin", MathFuncKind::Sin),
        ("cos", MathFuncKind::Cos),
        ("tan", MathFuncKind::Tan),
        ("exp", MathFuncKind::Exp),
        ("log", MathFuncKind::Log),
    ] {
        if lower.starts_with(kw) {
            let mut args: Vec<String> = line[kw.len()..]
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            if args.first().map(|s| s.eq_ignore_ascii_case("of")).unwrap_or(false) {
                args.remove(0);
            }
            return Some(StatementHint::MathFunc { func: kind, args, store_in: None });
        }
    }

    None
}

fn try_extract_error_handling(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("assert ") {
        return Some(StatementHint::Assert { expression: line[7..].trim().to_string() });
    }
    if lower.starts_with("perror") || lower.starts_with("print error") {
        let msg = line.split_whitespace().skip(2).collect::<Vec<&str>>().join(" ");
        let message = if msg.is_empty() { None } else { Some(msg) };
        return Some(StatementHint::Perror { message });
    }
    if lower.contains("errno") {
        return Some(StatementHint::ErrnoCheck);
    }
    None
}

fn try_extract_try_block(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower == "try" || lower.starts_with("try ") || lower.starts_with("attempt") {
        return Some(StatementHint::TryBlock);
    }
    None
}

fn try_extract_except_block(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, &["except", "catch"])?;
    let lower_rest = rest.to_lowercase();
    let mut exception_type = None;
    let mut variable = None;

    if let Some(idx) = lower_rest.find(" as ") {
        exception_type = Some(rest[..idx].trim().to_string());
        variable = Some(sanitize_identifier(&rest[idx + 4..]));
    } else if let Some(idx) = lower_rest.find(" into ") {
        exception_type = Some(rest[..idx].trim().to_string());
        variable = Some(sanitize_identifier(&rest[idx + 6..]));
    } else {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            exception_type = Some(tokens[0].to_string());
            if tokens.len() > 1 {
                let var = sanitize_identifier(tokens[1]);
                if !var.is_empty() {
                    variable = Some(var);
                }
            }
        }
    }

    Some(StatementHint::ExceptBlock { exception_type, variable })
}

fn try_extract_finally_block(line: &str) -> Option<StatementHint> {
    let lower = line.trim().to_lowercase();
    if lower == "finally" || lower.starts_with("finally ") || lower.starts_with("cleanup") {
        return Some(StatementHint::FinallyBlock);
    }
    None
}

fn try_extract_raise(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, &["raise", "throw"])?;
    let rest_trimmed = rest.trim();
    if rest_trimmed.is_empty() {
        return Some(StatementHint::RaiseException { exception_type: "Exception".to_string(), message: None });
    }
    let lower_rest = rest_trimmed.to_lowercase();
    if let Some(idx) = lower_rest.find(" with ") {
        let ex = rest_trimmed[..idx].trim();
        let msg = rest_trimmed[idx + 6..].trim();
        let exception_type = if ex.is_empty() { "Exception".to_string() } else { ex.to_string() };
        let message = if msg.is_empty() { None } else { Some(msg.to_string()) };
        return Some(StatementHint::RaiseException { exception_type, message });
    }
    let tokens: Vec<&str> = rest_trimmed.split_whitespace().collect();
    let exception_type = tokens[0].to_string();
    let message = if tokens.len() > 1 {
        Some(tokens[1..].join(" "))
    } else {
        None
    };
    Some(StatementHint::RaiseException { exception_type, message })
}

fn try_extract_error_check(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("check error") || lower.starts_with("error check") || lower.starts_with("handle error") {
        let rest = line.split_whitespace().skip(2).collect::<Vec<&str>>().join(" ");
        if !rest.trim().is_empty() {
            return Some(StatementHint::ErrorCheck { function_call: rest.trim().to_string(), error_variable: None });
        }
    }
    if lower.starts_with("if error in ") || lower.starts_with("if error for ") {
        let rest = line.split_whitespace().skip(3).collect::<Vec<&str>>().join(" ");
        if !rest.trim().is_empty() {
            return Some(StatementHint::ErrorCheck { function_call: rest.trim().to_string(), error_variable: None });
        }
    }
    None
}

fn try_extract_setjmp(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "setjmp")?;
    let buffer = sanitize_identifier(rest.trim());
    if !buffer.is_empty() {
        return Some(StatementHint::SetJmp { buffer });
    }
    None
}

fn try_extract_longjmp(line: &str) -> Option<StatementHint> {
    let rest = strip_keyword(line, "longjmp")?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.len() >= 2 {
        let buffer = sanitize_identifier(tokens[0]);
        let value = tokens[1..].join(" ");
        if !buffer.is_empty() && !value.is_empty() {
            return Some(StatementHint::LongJmp { buffer, value });
        }
    }
    None
}

fn try_extract_test_function(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_any_keyword(line, &["test function", "unit test", "test case"]) {
        let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or("test"));
        if !name.is_empty() {
            return Some(StatementHint::TestFunction { name, description: None });
        }
    }
    if let Some(rest) = strip_any_keyword(line, &["test setup", "setup"]) {
        let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or("setup"));
        if !name.is_empty() {
            return Some(StatementHint::TestSetup { name });
        }
    }
    if let Some(rest) = strip_any_keyword(line, &["test teardown", "teardown"]) {
        let name = sanitize_identifier(rest.split_whitespace().next().unwrap_or("teardown"));
        if !name.is_empty() {
            return Some(StatementHint::TestTeardown { name });
        }
    }
    if let Some(rest) = strip_any_keyword(line, &["mock", "stub"]) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let name = sanitize_identifier(tokens[0]);
            let mut return_value = "0".to_string();
            if let Some(idx) = rest.to_lowercase().find(" return ") {
                return_value = rest[idx + 8..].trim().to_string();
            }
            if !name.is_empty() {
                return Some(StatementHint::MockFunction { name, return_value });
            }
        }
    }
    None
}

fn try_extract_test_assert(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, &["assert that", "verify that", "check that", "expect"])?;
    let expr = rest.trim();
    if !expr.is_empty() {
        return Some(StatementHint::TestAssert { expression: expr.to_string(), message: None });
    }
    None
}

fn try_extract_test_equal(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("assert ") || lower.starts_with("expect ") || lower.contains(" should ") {
        if let Some((left, right)) = split_on_marker(line, &[" equals ", " equal to ", " to equal "]) {
            if !left.is_empty() && !right.is_empty() {
                return Some(StatementHint::TestAssertEqual { left, right, message: None });
            }
        }
    }
    None
}

fn try_extract_labeled_control(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("break ") {
        let label = sanitize_identifier(&line[6..]);
        if !label.is_empty() && label != "out" {
            return Some(StatementHint::LabeledBreak { label });
        }
    }
    if lower.starts_with("continue ") {
        let label = sanitize_identifier(&line[9..]);
        if !label.is_empty() {
            return Some(StatementHint::LabeledContinue { label });
        }
    }
    if lower.starts_with("labeled loop") || lower.starts_with("label loop") || lower.starts_with("loop label") {
        let name = sanitize_identifier(line.split_whitespace().last().unwrap_or("loop"));
        if !name.is_empty() {
            let loop_hint = StatementHint::Loop { iterator: Some("i".to_string()), start: None, end: None, collection: None, body_action: None };
            return Some(StatementHint::LabeledLoop { label: name, loop_hint: Box::new(loop_hint) });
        }
    }
    None
}

fn try_extract_match(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("match ") || lower.starts_with("pattern match ") {
        let rest = strip_any_keyword(line, &["match", "pattern match"])?;
        if !rest.trim().is_empty() {
            return Some(StatementHint::MatchBlock { expression: rest.trim().to_string() });
        }
    }
    if lower.starts_with("match case ") || lower.starts_with("pattern case ") {
        let rest = strip_any_keyword(line, &["match case", "pattern case"])?;
        let lower_rest = rest.to_lowercase();
        let (pattern_part, action_part) = if let Some(idx) = lower_rest.find(" do ") {
            (&rest[..idx], Some(rest[idx + 4..].trim().to_string()))
        } else {
            (rest, None)
        };
        let (pattern, guard) = if let Some(idx) = pattern_part.to_lowercase().find(" when ") {
            (pattern_part[..idx].trim().to_string(), Some(pattern_part[idx + 6..].trim().to_string()))
        } else {
            (pattern_part.trim().to_string(), None)
        };
        if !pattern.is_empty() {
            return Some(StatementHint::MatchCase { pattern, guard, action: action_part });
        }
    }
    if lower.starts_with("match default") || lower.starts_with("match otherwise") || lower.starts_with("match wildcard") {
        let rest = strip_any_keyword(line, &["match default", "match otherwise", "match wildcard"])?;
        let action = if rest.trim().is_empty() { None } else { Some(rest.trim().to_string()) };
        return Some(StatementHint::MatchWildcard { action });
    }
    None
}

fn try_extract_guard(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("guard ") {
        let rest = strip_keyword(line, "guard")?;
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" else ") {
            let condition = rest[..idx].trim().to_string();
            let action = rest[idx + 6..].trim().to_string();
            if !condition.is_empty() && !action.is_empty() {
                return Some(StatementHint::GuardClause { condition, action });
            }
        }
    }
    if lower.starts_with("early return if ") {
        let condition = line[16..].trim().to_string();
        if !condition.is_empty() {
            return Some(StatementHint::GuardClause { condition, action: "return".to_string() });
        }
    }
    None
}

fn try_extract_list_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create linked list") || lower.starts_with("linked list") {
        let rest = strip_any_keyword(line, &["create linked list", "linked list"])?;
        let name = sanitize_identifier(rest.split_whitespace().last().unwrap_or("list"));
        let element_type = if rest.to_lowercase().contains(" of ") {
            Some(rest.split(" of ").last().unwrap_or("").trim().to_string())
        } else {
            None
        };
        if !name.is_empty() {
            return Some(StatementHint::LinkedListCreate { name, element_type });
        }
    }
    if lower.starts_with("insert ") && lower.contains(" list ") {
        let rest = strip_keyword(line, "insert")?;
        if let Some((value, list)) = split_on_marker(rest, &[" into list ", " in list ", " to list "]) {
            let list_name = sanitize_identifier(&list);
            if !list_name.is_empty() && !value.is_empty() {
                return Some(StatementHint::LinkedListInsert { list: list_name, value, position: None });
            }
        }
    }
    if lower.starts_with("remove ") && lower.contains(" list ") {
        let rest = strip_keyword(line, "remove")?;
        if let Some((position, list)) = split_on_marker(rest, &[" from list ", " in list "]) {
            let list_name = sanitize_identifier(&list);
            if !list_name.is_empty() {
                return Some(StatementHint::LinkedListRemove { list: list_name, position });
            }
        }
    }
    if lower.starts_with("traverse list") || lower.starts_with("iterate list") {
        let rest = strip_any_keyword(line, &["traverse list", "iterate list"])?;
        let list = sanitize_identifier(rest.split_whitespace().next().unwrap_or("list"));
        let iterator = if let Some(idx) = rest.to_lowercase().find(" as ") {
            sanitize_identifier(&rest[idx + 4..])
        } else {
            "item".to_string()
        };
        if !list.is_empty() {
            return Some(StatementHint::LinkedListTraverse { list, iterator });
        }
    }
    None
}

fn try_extract_stack_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create stack") || lower.starts_with("stack ") {
        let rest = strip_any_keyword(line, &["create stack", "stack"])?;
        let name = sanitize_identifier(rest.split_whitespace().last().unwrap_or("stack"));
        if !name.is_empty() {
            return Some(StatementHint::StackCreate { name, element_type: None });
        }
    }
    if lower.starts_with("push ") && lower.contains(" stack ") {
        let rest = strip_keyword(line, "push")?;
        if let Some((value, stack)) = split_on_marker(rest, &[" onto stack ", " to stack "]) {
            let stack_name = sanitize_identifier(&stack);
            if !stack_name.is_empty() {
                return Some(StatementHint::StackPush { stack: stack_name, value });
            }
        }
    }
    if lower.starts_with("pop ") {
        let rest = strip_keyword(line, "pop")?;
        let stack = sanitize_identifier(rest.split_whitespace().last().unwrap_or("stack"));
        if !stack.is_empty() {
            return Some(StatementHint::StackPop { stack, target: None });
        }
    }
    if lower.starts_with("peek ") {
        let rest = strip_keyword(line, "peek")?;
        let stack = sanitize_identifier(rest.split_whitespace().last().unwrap_or("stack"));
        if !stack.is_empty() {
            return Some(StatementHint::StackPeek { stack, target: None });
        }
    }
    if lower.starts_with("stack is empty") || lower.starts_with("is stack empty") {
        let stack = sanitize_identifier(line.split_whitespace().last().unwrap_or("stack"));
        if !stack.is_empty() {
            return Some(StatementHint::StackIsEmpty { stack });
        }
    }
    None
}

fn try_extract_queue_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create queue") || lower.starts_with("queue ") {
        let rest = strip_any_keyword(line, &["create queue", "queue"])?;
        let name = sanitize_identifier(rest.split_whitespace().last().unwrap_or("queue"));
        if !name.is_empty() {
            return Some(StatementHint::QueueCreate { name, element_type: None });
        }
    }
    if lower.starts_with("enqueue ") {
        let rest = strip_keyword(line, "enqueue")?;
        if let Some((value, queue)) = split_on_marker(rest, &[" into queue ", " to queue "]) {
            let queue_name = sanitize_identifier(&queue);
            if !queue_name.is_empty() {
                return Some(StatementHint::QueueEnqueue { queue: queue_name, value });
            }
        }
    }
    if lower.starts_with("dequeue ") {
        let rest = strip_keyword(line, "dequeue")?;
        let queue = sanitize_identifier(rest.split_whitespace().last().unwrap_or("queue"));
        if !queue.is_empty() {
            return Some(StatementHint::QueueDequeue { queue, target: None });
        }
    }
    None
}

fn try_extract_map_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create map") || lower.starts_with("create dictionary") || lower.starts_with("map ") {
        let rest = strip_any_keyword(line, &["create map", "create dictionary", "map"])?;
        let name = sanitize_identifier(rest.split_whitespace().last().unwrap_or("map"));
        if !name.is_empty() {
            return Some(StatementHint::MapCreate { name, key_type: None, value_type: None });
        }
    }
    if lower.starts_with("put ") && lower.contains(" in map ") {
        let rest = strip_keyword(line, "put")?;
        if let Some((value, map_part)) = split_on_marker(rest, &[" in map ", " into map "]) {
            if let Some((key, map_name)) = split_on_marker(&map_part, &[" at key ", " with key "]) {
                let map = sanitize_identifier(&map_name);
                if !map.is_empty() {
                    return Some(StatementHint::MapPut { map, key, value });
                }
            }
        }
    }
    if lower.starts_with("get ") && lower.contains(" from map ") {
        let rest = strip_keyword(line, "get")?;
        if let Some((key, map_name)) = split_on_marker(rest, &[" from map ", " in map "]) {
            let map = sanitize_identifier(&map_name);
            if !map.is_empty() {
                return Some(StatementHint::MapGet { map, key, target: None });
            }
        }
    }
    if lower.starts_with("remove ") && lower.contains(" from map ") {
        let rest = strip_keyword(line, "remove")?;
        if let Some((key, map_name)) = split_on_marker(rest, &[" from map "]) {
            let map = sanitize_identifier(&map_name);
            if !map.is_empty() {
                return Some(StatementHint::MapRemove { map, key });
            }
        }
    }
    if lower.starts_with("map contains") || lower.contains(" in map ") {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() >= 3 {
            let key = tokens[tokens.len() - 1].to_string();
            let map = sanitize_identifier(tokens[tokens.len() - 2]);
            if !map.is_empty() {
                return Some(StatementHint::MapContainsKey { map, key });
            }
        }
    }
    if lower.contains(" map contains ") {
        let parts: Vec<&str> = line.split(" map contains ").collect();
        if parts.len() == 2 {
            let map = sanitize_identifier(parts[0].trim());
            let key = parts[1].trim().to_string();
            if !map.is_empty() && !key.is_empty() {
                return Some(StatementHint::MapContainsKey { map, key });
            }
        }
    }
    if lower.starts_with("map keys") {
        let map = sanitize_identifier(line.split_whitespace().last().unwrap_or("map"));
        if !map.is_empty() {
            return Some(StatementHint::MapKeys { map, target: None });
        }
    }
    if lower.starts_with("map values") {
        let map = sanitize_identifier(line.split_whitespace().last().unwrap_or("map"));
        if !map.is_empty() {
            return Some(StatementHint::MapValues { map, target: None });
        }
    }
    None
}

fn try_extract_set_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create set") || lower.starts_with("set ") {
        let rest = strip_any_keyword(line, &["create set", "set"])?;
        let name = sanitize_identifier(rest.split_whitespace().last().unwrap_or("set"));
        if !name.is_empty() {
            return Some(StatementHint::SetCreate { name, element_type: None });
        }
    }
    if lower.starts_with("add ") && lower.contains(" to set ") {
        let rest = strip_keyword(line, "add")?;
        if let Some((value, set_name)) = split_on_marker(rest, &[" to set "]) {
            let set = sanitize_identifier(&set_name);
            if !set.is_empty() {
                return Some(StatementHint::SetAdd { set, value });
            }
        }
    }
    if lower.starts_with("remove ") && lower.contains(" from set ") {
        let rest = strip_keyword(line, "remove")?;
        if let Some((value, set_name)) = split_on_marker(rest, &[" from set "]) {
            let set = sanitize_identifier(&set_name);
            if !set.is_empty() {
                return Some(StatementHint::SetRemove { set, value });
            }
        }
    }
    if lower.contains(" in set ") {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() >= 3 {
            let value = tokens[tokens.len() - 3].to_string();
            let set = sanitize_identifier(tokens[tokens.len() - 1]);
            if !set.is_empty() {
                return Some(StatementHint::SetContains { set, value });
            }
        }
    }
    if lower.contains(" set contains ") {
        let parts: Vec<&str> = line.split(" set contains ").collect();
        if parts.len() == 2 {
            let set = sanitize_identifier(parts[0].trim());
            let value = parts[1].trim().to_string();
            if !set.is_empty() && !value.is_empty() {
                return Some(StatementHint::SetContains { set, value });
            }
        }
    }
    if lower.starts_with("union ") && lower.contains(" and ") {
        let rest = strip_keyword(line, "union")?;
        let parts: Vec<&str> = rest.split(" and ").collect();
        if parts.len() >= 2 {
            let set1 = sanitize_identifier(parts[0]);
            let set2 = sanitize_identifier(parts[1]);
            if !set1.is_empty() && !set2.is_empty() {
                let target = format!("{}_union_{}", set1, set2);
                return Some(StatementHint::SetUnion { set1, set2, target });
            }
        }
    }
    if lower.starts_with("intersection ") && lower.contains(" and ") {
        let rest = strip_keyword(line, "intersection")?;
        let parts: Vec<&str> = rest.split(" and ").collect();
        if parts.len() >= 2 {
            let set1 = sanitize_identifier(parts[0]);
            let set2 = sanitize_identifier(parts[1]);
            if !set1.is_empty() && !set2.is_empty() {
                let target = format!("{}_intersect_{}", set1, set2);
                return Some(StatementHint::SetIntersection { set1, set2, target });
            }
        }
    }
    None
}

fn parse_visibility(lower: &str) -> Visibility {
    if lower.contains("private") {
        Visibility::Private
    } else if lower.contains("protected") {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

fn try_extract_class_def(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("class ") || lower.starts_with("abstract class ") {
        let is_abstract = lower.starts_with("abstract class");
        let rest = strip_any_keyword(line, &["abstract class", "class"])?;
        let mut name_part = rest;
        let mut parent = None;
        let mut interfaces = Vec::new();
        let lower_rest = rest.to_lowercase();
        if let Some(idx) = lower_rest.find(" extends ") {
            name_part = &rest[..idx];
            parent = Some(sanitize_identifier(&rest[idx + 9..]));
        }
        if let Some(idx) = lower_rest.find(" implements ") {
            name_part = &rest[..idx];
            let impls = rest[idx + 12..].split(',').map(|s| sanitize_identifier(s.trim())).filter(|s| !s.is_empty()).collect();
            interfaces = impls;
        }
        let name = sanitize_identifier(name_part.trim());
        if !name.is_empty() {
            return Some(StatementHint::ClassDef { name, parent, interfaces, is_abstract });
        }
    }
    None
}

fn try_extract_class_field(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.contains(" field ") || lower.contains(" property ") {
        let visibility = parse_visibility(&lower);
        let is_static = lower.contains(" static ");
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let name = sanitize_identifier(tokens.last().unwrap_or(&""));
        if !name.is_empty() {
            return Some(StatementHint::ClassField { name, type_hint: None, visibility, is_static, initial_value: None });
        }
    }
    None
}

fn try_extract_class_method(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.contains(" method ") || lower.starts_with("method ") {
        let visibility = parse_visibility(&lower);
        let is_static = lower.contains(" static ");
        let is_abstract = lower.contains(" abstract ");
        let is_virtual = lower.contains(" virtual ");
        let rest = strip_any_keyword(line, &["method", "public method", "private method", "protected method", "static method"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let name = sanitize_identifier(tokens[0]);
            let (parameters, return_type) = parse_function_signature(&tokens[1..]);
            return Some(StatementHint::ClassMethod { name, parameters, return_type, visibility, is_static, is_abstract, is_virtual });
        }
    }
    None
}

fn try_extract_constructor(line: &str) -> Option<StatementHint> {
    let rest = strip_any_keyword(line, &["constructor", "init", "initialize"])?;
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let (parameters, _) = parse_function_signature(&tokens);
    return Some(StatementHint::Constructor { parameters, body: None });
}

fn try_extract_object_create(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("new ") || lower.starts_with("create object") || lower.starts_with("instantiate ") {
        let rest = strip_any_keyword(line, &["new", "create object", "instantiate"])?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let class_name = sanitize_identifier(tokens[0]);
            let mut variable = class_name.to_lowercase();
            let mut arguments = Vec::new();
            if let Some(idx) = rest.to_lowercase().find(" with ") {
                arguments = rest[idx + 6..].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            if let Some(idx) = rest.to_lowercase().find(" as ") {
                variable = sanitize_identifier(&rest[idx + 4..]);
            }
            if !class_name.is_empty() && !variable.is_empty() {
                return Some(StatementHint::ObjectCreate { class_name, variable, arguments });
            }
        }
    }
    None
}

fn try_extract_method_call(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("call ") && lower.contains(" on ") {
        let rest = strip_keyword(line, "call")?;
        if let Some(idx) = rest.to_lowercase().find(" on ") {
            let method = sanitize_identifier(&rest[..idx]);
            let object = sanitize_identifier(&rest[idx + 4..]);
            if !method.is_empty() && !object.is_empty() {
                return Some(StatementHint::MethodCall { object, method, arguments: Vec::new() });
            }
        }
    }
    if let Some(dot_idx) = line.find('.') {
        let object = sanitize_identifier(&line[..dot_idx]);
        let method = sanitize_identifier(&line[dot_idx + 1..]);
        if !object.is_empty() && !method.is_empty() {
            return Some(StatementHint::MethodCall { object, method, arguments: Vec::new() });
        }
    }
    if lower.starts_with("property ") && lower.contains(" of ") {
        let rest = strip_keyword(line, "property")?;
        if let Some((prop, obj)) = split_on_marker(rest, &[" of "]) {
            let object = sanitize_identifier(&obj);
            if !object.is_empty() && !prop.is_empty() {
                return Some(StatementHint::PropertyAccess { object, property: prop });
            }
        }
    }
    if lower.starts_with("set property ") && lower.contains(" to ") && lower.contains(" of ") {
        let rest = strip_keyword(line, "set property")?;
        let lower_rest = rest.to_lowercase();
        if let Some(of_idx) = lower_rest.find(" of ") {
            if let Some(to_idx) = lower_rest.find(" to ") {
                let property = rest[..of_idx].trim().to_string();
                let object = sanitize_identifier(&rest[of_idx + 4..to_idx]);
                let value = rest[to_idx + 4..].trim().to_string();
                if !object.is_empty() && !property.is_empty() && !value.is_empty() {
                    return Some(StatementHint::PropertyAssign { object, property, value });
                }
            }
        }
    }
    if lower == "this" || lower == "self" {
        return Some(StatementHint::ThisReference);
    }
    if lower.starts_with("super") {
        let rest = strip_keyword(line, "super").unwrap_or("").trim();
        let method = if rest.is_empty() { None } else { Some(sanitize_identifier(rest)) };
        return Some(StatementHint::SuperCall { method, arguments: Vec::new() });
    }
    None
}

fn try_extract_lambda(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("lambda ") {
        let rest = line[7..].trim();
        if let Some(idx) = rest.find(':') {
            let params = rest[..idx].split(',').map(|s| sanitize_identifier(s.trim())).filter(|s| !s.is_empty()).collect();
            let body = rest[idx + 1..].trim().to_string();
            if !body.is_empty() {
                return Some(StatementHint::Lambda { parameters: params, body, captures: Vec::new() });
            }
        }
    }
    if line.contains("=>") {
        let parts: Vec<&str> = line.split("=>").collect();
        if parts.len() == 2 {
            let params = parts[0].split(',').map(|s| sanitize_identifier(s.trim())).filter(|s| !s.is_empty()).collect();
            let body = parts[1].trim().to_string();
            if !body.is_empty() {
                return Some(StatementHint::Lambda { parameters: params, body, captures: Vec::new() });
            }
        }
    }
    None
}

fn try_extract_map_filter_reduce(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("map ") && lower.contains(" over ") {
        let rest = strip_keyword(line, "map")?;
        if let Some((transform, collection)) = split_on_marker(rest, &[" over "]) {
            return Some(StatementHint::MapFunction { collection, transform, target: None });
        }
    }
    if lower.starts_with("filter ") && lower.contains(" where ") {
        let rest = strip_keyword(line, "filter")?;
        if let Some((collection, predicate)) = split_on_marker(rest, &[" where "]) {
            return Some(StatementHint::FilterFunction { collection, predicate, target: None });
        }
    }
    if lower.starts_with("reduce ") && lower.contains(" with ") {
        let rest = strip_keyword(line, "reduce")?;
        if let Some((collection, reducer)) = split_on_marker(rest, &[" with "]) {
            return Some(StatementHint::ReduceFunction { collection, reducer, initial: None, target: None });
        }
    }
    if lower.starts_with("for each ") && lower.contains(" in ") {
        let rest = strip_keyword(line, "for each")?;
        if let Some((item, collection)) = split_on_marker(rest, &[" in "]) {
            return Some(StatementHint::ForEachFunction { collection, action: format!("use {}", item) });
        }
    }
    None
}

fn try_extract_thread_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create thread") || lower.starts_with("start thread") || lower.starts_with("spawn thread") {
        let rest = strip_any_keyword(line, &["create thread", "start thread", "spawn thread"])?;
        let function = rest.split_whitespace().last().unwrap_or("worker").to_string();
        return Some(StatementHint::ThreadCreate { name: None, function, arguments: Vec::new() });
    }
    if lower.starts_with("join thread") {
        let rest = strip_keyword(line, "join thread")?;
        let thread = sanitize_identifier(rest);
        if !thread.is_empty() {
            return Some(StatementHint::ThreadJoin { thread });
        }
    }
    if lower.starts_with("detach thread") {
        let rest = strip_keyword(line, "detach thread")?;
        let thread = sanitize_identifier(rest);
        if !thread.is_empty() {
            return Some(StatementHint::ThreadDetach { thread });
        }
    }
    if lower.starts_with("sleep ") {
        let rest = strip_keyword(line, "sleep")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            return Some(StatementHint::ThreadSleep { duration: tokens[0].to_string(), unit: tokens[1].to_string() });
        }
    }
    None
}

fn try_extract_sync_ops(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("create mutex") {
        let rest = strip_keyword(line, "create mutex")?;
        let name = sanitize_identifier(rest);
        if !name.is_empty() {
            return Some(StatementHint::MutexCreate { name });
        }
    }
    if lower.starts_with("lock mutex") {
        let rest = strip_keyword(line, "lock mutex")?;
        let mutex = sanitize_identifier(rest);
        if !mutex.is_empty() {
            return Some(StatementHint::MutexLock { mutex });
        }
    }
    if lower.starts_with("unlock mutex") {
        let rest = strip_keyword(line, "unlock mutex")?;
        let mutex = sanitize_identifier(rest);
        if !mutex.is_empty() {
            return Some(StatementHint::MutexUnlock { mutex });
        }
    }
    if lower.starts_with("try lock mutex") {
        let rest = strip_keyword(line, "try lock mutex")?;
        let mutex = sanitize_identifier(rest);
        if !mutex.is_empty() {
            return Some(StatementHint::MutexTryLock { mutex });
        }
    }
    if lower.starts_with("create semaphore") {
        let rest = strip_keyword(line, "create semaphore")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let name = sanitize_identifier(tokens[0]);
            let initial = if tokens.len() > 1 { tokens[1].to_string() } else { "1".to_string() };
            if !name.is_empty() {
                return Some(StatementHint::SemaphoreCreate { name, initial });
            }
        }
    }
    if lower.starts_with("wait semaphore") {
        let rest = strip_keyword(line, "wait semaphore")?;
        let semaphore = sanitize_identifier(rest);
        if !semaphore.is_empty() {
            return Some(StatementHint::SemaphoreWait { semaphore });
        }
    }
    if lower.starts_with("signal semaphore") {
        let rest = strip_keyword(line, "signal semaphore")?;
        let semaphore = sanitize_identifier(rest);
        if !semaphore.is_empty() {
            return Some(StatementHint::SemaphoreSignal { semaphore });
        }
    }
    if lower.starts_with("create condition") {
        let rest = strip_keyword(line, "create condition")?;
        let name = sanitize_identifier(rest);
        if !name.is_empty() {
            return Some(StatementHint::ConditionCreate { name });
        }
    }
    if lower.starts_with("wait condition") {
        let rest = strip_keyword(line, "wait condition")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            let condition = sanitize_identifier(tokens[0]);
            let mutex = sanitize_identifier(tokens[1]);
            if !condition.is_empty() && !mutex.is_empty() {
                return Some(StatementHint::ConditionWait { condition, mutex });
            }
        }
    }
    if lower.starts_with("signal condition") {
        let rest = strip_keyword(line, "signal condition")?;
        let condition = sanitize_identifier(rest);
        if !condition.is_empty() {
            return Some(StatementHint::ConditionSignal { condition });
        }
    }
    if lower.starts_with("broadcast condition") {
        let rest = strip_keyword(line, "broadcast condition")?;
        let condition = sanitize_identifier(rest);
        if !condition.is_empty() {
            return Some(StatementHint::ConditionBroadcast { condition });
        }
    }
    if lower.starts_with("atomic ") {
        let rest = strip_keyword(line, "atomic")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 2 {
            let name = sanitize_identifier(tokens[0]);
            let initial = tokens[1..].join(" ");
            if !name.is_empty() && !initial.is_empty() {
                return Some(StatementHint::AtomicCreate { name, initial });
            }
        }
    }
    if lower.starts_with("atomic load") {
        let rest = strip_keyword(line, "atomic load")?;
        let atomic = sanitize_identifier(rest);
        if !atomic.is_empty() {
            return Some(StatementHint::AtomicLoad { atomic, target: None });
        }
    }
    if lower.starts_with("atomic store") {
        let rest = strip_keyword(line, "atomic store")?;
        if let Some((value, atomic)) = split_on_marker(rest, &[" in "]) {
            let name = sanitize_identifier(&atomic);
            if !name.is_empty() {
                return Some(StatementHint::AtomicStore { atomic: name, value });
            }
        }
    }
    if lower.starts_with("compare exchange") {
        let rest = strip_keyword(line, "compare exchange")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() >= 3 {
            let atomic = sanitize_identifier(tokens[0]);
            let expected = tokens[1].to_string();
            let desired = tokens[2].to_string();
            if !atomic.is_empty() {
                return Some(StatementHint::AtomicCompareExchange { atomic, expected, desired });
            }
        }
    }
    if lower.starts_with("atomic increment") {
        let rest = strip_keyword(line, "atomic increment")?;
        let atomic = sanitize_identifier(rest);
        if !atomic.is_empty() {
            return Some(StatementHint::AtomicIncrement { atomic });
        }
    }
    if lower.starts_with("atomic decrement") {
        let rest = strip_keyword(line, "atomic decrement")?;
        let atomic = sanitize_identifier(rest);
        if !atomic.is_empty() {
            return Some(StatementHint::AtomicDecrement { atomic });
        }
    }
    None
}

fn try_extract_async(line: &str) -> Option<StatementHint> {
    let lower = line.to_lowercase();
    if lower.starts_with("async function") {
        let rest = strip_keyword(line, "async function")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let name = sanitize_identifier(tokens[0]);
            let (parameters, return_type) = parse_function_signature(&tokens[1..]);
            return Some(StatementHint::AsyncFunction { name, parameters, return_type });
        }
    }
    if lower.starts_with("await ") {
        let rest = strip_keyword(line, "await")?;
        if let Some((expr, target)) = split_on_marker(rest, &[" into ", " as "]) {
            return Some(StatementHint::AwaitExpression { expression: expr, target: Some(sanitize_identifier(&target)) });
        }
        return Some(StatementHint::AwaitExpression { expression: rest.trim().to_string(), target: None });
    }
    if lower.starts_with("promise ") && lower.contains(" then ") {
        let rest = strip_keyword(line, "promise")?;
        if let Some((promise, handler)) = split_on_marker(rest, &[" then "]) {
            return Some(StatementHint::PromiseThen { promise, handler });
        }
    }
    if lower.starts_with("promise ") && lower.contains(" catch ") {
        let rest = strip_keyword(line, "promise")?;
        if let Some((promise, handler)) = split_on_marker(rest, &[" catch "]) {
            return Some(StatementHint::PromiseCatch { promise, handler });
        }
    }
    if lower.starts_with("promise all ") {
        let rest = strip_keyword(line, "promise all")?;
        if let Some((list, target)) = split_on_marker(rest, &[" into ", " as "]) {
            let promises: Vec<String> = list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let target_name = sanitize_identifier(&target);
            if !promises.is_empty() && !target_name.is_empty() {
                return Some(StatementHint::PromiseAll { promises, target: target_name });
            }
        }
    }
    if lower.starts_with("promise race ") {
        let rest = strip_keyword(line, "promise race")?;
        if let Some((list, target)) = split_on_marker(rest, &[" into ", " as "]) {
            let promises: Vec<String> = list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let target_name = sanitize_identifier(&target);
            if !promises.is_empty() && !target_name.is_empty() {
                return Some(StatementHint::PromiseRace { promises, target: target_name });
            }
        }
    }
    if lower.starts_with("create promise") {
        let rest = strip_keyword(line, "create promise")?;
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if !tokens.is_empty() {
            let name = sanitize_identifier(tokens[0]);
            let executor = if tokens.len() > 1 { tokens[1..].join(" ") } else { "executor".to_string() };
            if !name.is_empty() {
                return Some(StatementHint::PromiseCreate { name, executor });
            }
        }
    }
    None
}

fn parse_doc_type(lower: &str) -> DocType {
    if lower.contains("param") {
        DocType::Param
    } else if lower.contains("return") {
        DocType::Return
    } else if lower.contains("throw") || lower.contains("raises") {
        DocType::Throws
    } else if lower.contains("detail") {
        DocType::Detailed
    } else {
        DocType::Brief
    }
}

fn try_extract_doc_comment(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_any_keyword(line, &["document", "docstring", "describe", "explain"]) {
        let doc_type = parse_doc_type(&line.to_lowercase());
        let text = rest.trim().to_string();
        if !text.is_empty() {
            return Some(StatementHint::DocComment { text, doc_type });
        }
    }
    None
}

fn try_extract_doc_function(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_any_keyword(line, &["document function", "doc function"]) {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        let brief = if tokens.is_empty() { "function".to_string() } else { format!("Function {}", tokens[0]) };
        return Some(StatementHint::DocFunction { brief, params: Vec::new(), returns: None, throws: Vec::new(), examples: Vec::new() });
    }
    None
}

fn try_extract_doc_class(line: &str) -> Option<StatementHint> {
    if let Some(rest) = strip_any_keyword(line, &["document class", "doc class"]) {
        let brief = format!("Class {}", rest.trim());
        return Some(StatementHint::DocClass { brief, detailed: None, author: None, version: None });
    }
    None
}

fn try_extract_stdlib_call(line: &str, included_headers: &std::collections::HashSet<String>) -> Option<StatementHint> {
    let db = FunctionDatabase::core();
    let FunctionMatch { name, args } = match_function_call(line, included_headers, db)?;
    Some(StatementHint::StdLibCall { name, args })
}

// ============================================================================
// Context Analysis - Variable Declaration Tracking
// ============================================================================

use std::collections::HashSet;

/// Tracks declared variables from preceding code.
/// Used to determine whether to declare new variables or just assign.
#[derive(Debug, Clone)]
pub struct VariableContext {
    /// Set of declared variable names
    pub declared: HashSet<String>,
    /// Headers that were included before the current line
    pub included_headers: HashSet<String>,
}

impl VariableContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            declared: HashSet::new(),
            included_headers: HashSet::new(),
        }
    }

    /// Extract declared variables from preceding code
    pub fn from_code(code_before: &str) -> Self {
        let mut declared = HashSet::new();
        let mut included_headers = HashSet::new();
        
        for line in code_before.lines() {
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Track includes
            if let Some(header) = parse_include_line(trimmed) {
                included_headers.insert(header);
                continue;
            }
            
            // Look for C-style declarations: "type name" or "type name = value"
            // Pattern: int x; int x = 5; int x, y, z;
            for type_keyword in ["int", "float", "double", "char", "bool", "long", "short", "unsigned", "void"] {
                if trimmed.starts_with(type_keyword) {
                    let rest = &trimmed[type_keyword.len()..];
                    // Must have space or * after type
                    if rest.starts_with(' ') || rest.starts_with('*') {
                        // Extract identifiers after the type
                        for part in rest.split(',') {
                            let part = part.trim().trim_end_matches(';').trim_end_matches('{');
                            // Handle "x = 5" or just "x"
                            let name = part.split('=').next().unwrap_or("").trim();
                            // Handle arrays like "x[10]"
                            let name = name.split('[').next().unwrap_or(name).trim();
                            // Handle pointers like "*x"
                            let name = name.trim_start_matches('*').trim();
                            // Handle function params - skip if contains '('
                            if name.contains('(') {
                                continue;
                            }
                            if !name.is_empty() && is_valid_identifier(name) {
                                declared.insert(name.to_string());
                            }
                        }
                    }
                }
            }
            
            // Look for scanf declarations: scanf("%d", &x) means x is declared
            if trimmed.starts_with("scanf(") {
                // Extract variables from &var patterns
                for part in trimmed.split('&') {
                    if part.starts_with("scanf") {
                        continue;
                    }
                    let name = part.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("");
                    if !name.is_empty() && is_valid_identifier(name) {
                        declared.insert(name.to_string());
                    }
                }
            }
            
            // Look for for-loop iterators: for (int i = ...) or for (i = ...)
            if trimmed.starts_with("for") && trimmed.contains('(') {
                if let Some(paren_content) = trimmed.split('(').nth(1) {
                    let init_part = paren_content.split(';').next().unwrap_or("");
                    // Handle "int i = 0" or "i = 0"
                    let init_trimmed = init_part.trim();
                    for type_keyword in ["int", "float", "double"] {
                        if init_trimmed.starts_with(type_keyword) {
                            let rest = init_trimmed[type_keyword.len()..].trim();
                            let name = rest.split('=').next().unwrap_or("").trim();
                            if !name.is_empty() && is_valid_identifier(name) {
                                declared.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        Self { declared, included_headers }
    }

    /// Check if a variable is declared
    pub fn is_declared(&self, name: &str) -> bool {
        self.declared.contains(name)
    }

    /// Get all declared variables
    pub fn get_declared(&self) -> Vec<String> {
        self.declared.iter().cloned().collect()
    }
}

fn parse_include_line(line: &str) -> Option<String> {
    if !line.starts_with("#include") {
        return None;
    }
    let rest = line.trim_start_matches("#include").trim();
    if rest.starts_with('<') && rest.ends_with('>') {
        return Some(rest.trim_matches(['<', '>'].as_ref()).to_string());
    }
    if rest.starts_with('"') && rest.ends_with('"') {
        return Some(rest.trim_matches('"').to_string());
    }
    None
}

/// Check if a string is a valid C identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Result of validating read variables against context.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed (all reads are declared)
    pub is_valid: bool,
    /// Undeclared variables that were read
    pub undeclared_reads: Vec<String>,
    /// Variables that need to be declared (write targets not in context)
    pub needs_declaration: Vec<String>,
    /// Variables that already exist (write targets in context)
    pub already_declared: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            is_valid: true,
            undeclared_reads: Vec::new(),
            needs_declaration: Vec::new(),
            already_declared: Vec::new(),
        }
    }

    pub fn error(undeclared: Vec<String>) -> Self {
        Self {
            is_valid: false,
            undeclared_reads: undeclared,
            needs_declaration: Vec::new(),
            already_declared: Vec::new(),
        }
    }

    pub fn error_message(&self) -> Option<String> {
        if self.is_valid {
            None
        } else {
            Some(format!(
                "unknown identifiers: {}",
                self.undeclared_reads.join(", ")
            ))
        }
    }
}

/// Information about read and write variables in a hint
#[derive(Debug, Clone, Default)]
pub struct ReadWriteInfo {
    /// Variables being written to (can be auto-declared)
    pub writes: Vec<String>,
    /// Variables being read from (must already exist)
    pub reads: Vec<String>,
}

/// Extract read/write variable information from a hint
pub fn get_read_write_info(hint: &StatementHint) -> ReadWriteInfo {
    match hint {
        StatementHint::Declaration { names, initial_value, .. } => {
            let mut info = ReadWriteInfo::default();
            info.writes = names.clone();
            // If there's an initial value that's an expression, extract reads
            if let Some(val) = initial_value {
                if !val.parse::<f64>().is_ok() && !val.starts_with('"') {
                    info.reads = extract_identifiers_from_expression(val);
                }
            }
            info
        }
        
        StatementHint::Assignment { targets, value } => {
            let mut info = ReadWriteInfo::default();
            info.writes = targets.clone();
            // Parse value for read variables
            if !value.parse::<f64>().is_ok() && !value.starts_with('"') {
                info.reads = extract_identifiers_from_expression(value);
            }
            info
        }
        
        StatementHint::Modify { target, .. } => {
            // Increment/decrement reads AND writes the target
            ReadWriteInfo {
                writes: vec![],  // Not a new declaration
                reads: vec![target.clone()],  // Must exist to modify
            }
        }

        StatementHint::PrePostModify { target, .. } => {
            ReadWriteInfo {
                writes: vec![],
                reads: vec![target.clone()],
            }
        }
        
        StatementHint::Arithmetic { left, right, target, .. } => {
            let mut info = ReadWriteInfo::default();
            info.reads = vec![left.clone(), right.clone()];
            if let Some(t) = target {
                info.writes.push(t.clone());
            }
            info
        }
        
        // Conditional is handled by the case below that returns default()
        // We don't validate reads for conditionals since the user might be experimenting
        
        StatementHint::Loop { iterator, start, end, collection, body_action, .. } => {
            let mut info = ReadWriteInfo::default();
            // Iterator is a write target (gets declared by the loop)
            if let Some(iter) = iterator {
                info.writes.push(iter.clone());
            }
            // Start, end, collection are reads
            if let Some(s) = start {
                if !s.parse::<f64>().is_ok() {
                    info.reads.extend(extract_identifiers_from_expression(s));
                }
            }
            if let Some(e) = end {
                if !e.parse::<f64>().is_ok() {
                    info.reads.extend(extract_identifiers_from_expression(e));
                }
            }
            if let Some(c) = collection {
                info.reads.push(c.clone());
            }
            if let Some(action) = body_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            info
        }
        
        StatementHint::Print { content: _, is_literal: _ } => {
            // Print is special: we don't validate reads here because
            // an undeclared identifier in print becomes a literal string at code gen time
            // (handled in translate_with_context_c)
            ReadWriteInfo::default()
        }
        
        StatementHint::Return { value } => {
            let mut info = ReadWriteInfo::default();
            if let Some(val) = value {
                if !val.parse::<f64>().is_ok() {
                    info.reads = extract_identifiers_from_expression(val);
                }
            }
            info
        }
        
        StatementHint::While { condition, body_action } => {
            let mut info = ReadWriteInfo::default();
            
            // Parse the condition to identify loop variable vs bound
            let (loop_var, bound, is_counter) = parse_while_condition_for_rw(condition);
            
            if let Some(var) = loop_var {
                if is_counter {
                    // For counter patterns (i < 10), the loop var can be auto-declared
                    info.writes.push(var);
                } else {
                    // For general comparisons (a < b), both must exist
                    info.reads.push(var);
                }
            }
            
            if let Some(b) = bound {
                // Only add as read if it's not a literal number
                if b.parse::<f64>().is_err() {
                    info.reads.push(b);
                }
            }
            
            if let Some(action) = body_action {
                info.reads.extend(extract_identifiers_from_expression(action));
            }
            info
        }
        
        StatementHint::Read { variables } => {
            // Read introduces new variables (scanf declares them)
            ReadWriteInfo {
                writes: variables.clone(),
                reads: vec![],
            }
        }
        
        StatementHint::Conditional { .. } => {
            // For conditionals, we don't validate reads strictly.
            // The user might be typing a condition with variables they'll declare later,
            // or they might be experimenting. The C compiler will catch real errors.
            // This allows patterns like "if a <= b print a else print b" to work
            // without requiring a and b to be declared first.
            ReadWriteInfo::default()
        }

        StatementHint::StdLibCall { .. } => ReadWriteInfo::default(),

        _ => ReadWriteInfo::default(),
    }
}

/// Validate that all read variables are declared in context
pub fn validate_reads(hint: &StatementHint, context: &VariableContext) -> ValidationResult {
    let rw_info = get_read_write_info(hint);
    
    let mut result = ValidationResult::ok();
    
    // Check each read variable
    for var in &rw_info.reads {
        // Skip numbers and constants
        if var.parse::<f64>().is_ok() {
            continue;
        }
        let var_lower = var.to_lowercase();
        if ["true", "false", "null", "nullptr", "none"].contains(&var_lower.as_str()) {
            continue;
        }
        // Check if declared
        if !context.is_declared(var) {
            result.is_valid = false;
            if !result.undeclared_reads.contains(var) {
                result.undeclared_reads.push(var.clone());
            }
        }
    }
    
    // Classify write variables
    for var in &rw_info.writes {
        if context.is_declared(var) {
            result.already_declared.push(var.clone());
        } else {
            result.needs_declaration.push(var.clone());
        }
    }
    
    result
}

// Legacy compatibility - keep old function name
pub fn analyze_context(hint: &StatementHint, code_before: &str) -> ContextAnalysis {
    let context = VariableContext::from_code(code_before);
    let validation = validate_reads(hint, &context);
    
    ContextAnalysis {
        warnings: validation.undeclared_reads.iter()
            .map(|v| format!("Variable '{}' may not be declared", v))
            .collect(),
        undeclared_vars: validation.undeclared_reads,
    }
}

/// Legacy struct for backward compatibility
#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub warnings: Vec<String>,
    pub undeclared_vars: Vec<String>,
}

impl ContextAnalysis {
    pub fn empty() -> Self {
        Self {
            warnings: Vec::new(),
            undeclared_vars: Vec::new(),
        }
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Parse a while condition to extract the loop variable, bound, and whether it's a counter pattern
/// Returns (loop_var, bound, is_counter_pattern)
fn parse_while_condition_for_rw(condition: &str) -> (Option<String>, Option<String>, bool) {
    let condition = condition.trim();
    
    // Look for comparison operators
    for op in [" < ", " <= ", " > ", " >= ", " != ", " == "] {
        if let Some(idx) = condition.find(op) {
            let left = condition[..idx].trim();
            let right = condition[idx + op.len()..].trim();
            
            // Left side is the loop variable if it's a simple identifier
            let loop_var = if left.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') 
               && left.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
                Some(left.to_string())
            } else {
                None
            };
            
            // Right side is the bound
            let bound = if !right.is_empty() {
                Some(right.to_string())
            } else {
                None
            };
            
            // Determine if this is a counter pattern:
            // - Variable is a common iterator name (i, j, k, n, count, counter, idx, index)
            // - OR bound is a numeric literal
            let is_counter_var = loop_var.as_ref().map(|v| {
                matches!(v.to_lowercase().as_str(), 
                    "i" | "j" | "k" | "n" | "count" | "counter" | "idx" | "index" | "iter")
            }).unwrap_or(false);
            let bound_is_numeric = bound.as_ref().map(|b| {
                b.chars().all(|c| c.is_ascii_digit() || c == '-')
            }).unwrap_or(false);
            let is_counter = is_counter_var || bound_is_numeric;
            
            return (loop_var, bound, is_counter);
        }
    }
    
    // Couldn't parse
    (None, None, false)
}

/// Extract identifier-like tokens from an expression string.
fn extract_identifiers_from_expression(expr: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut current = String::new();
    
    for ch in expr.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                // Check if it looks like an identifier (not a number)
                if current.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
                    identifiers.push(current.clone());
                }
                current.clear();
            }
        }
    }
    
    if !current.is_empty() && current.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        identifiers.push(current);
    }
    
    identifiers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(line: &str) -> TranslateLineRequest {
        TranslateLineRequest {
            english_line: line.to_string(),
            code_before: String::new(),
            code_after: String::new(),
            language: "c".to_string(),
            line_index: 0,
            api_key: None,
            model: None,
            max_lines: None,
        }
    }

    #[test]
    fn test_declaration() {
        let hint = extract(&make_request("declare x"));
        assert!(matches!(hint, StatementHint::Declaration { .. }));
    }

    #[test]
    fn test_assignment() {
        let hint = extract(&make_request("set x to 5"));
        assert!(matches!(hint, StatementHint::Assignment { targets, value } if targets == vec!["x"] && value == "5"));
    }

    #[test]
    fn test_flexible_assignment_patterns() {
        let hint = extract(&make_request("sum a plus b"));
        assert!(matches!(hint, StatementHint::Assignment { targets, value } if targets == vec!["sum"] && value == "a + b"));

        let hint = extract(&make_request("a plus b is sum"));
        assert!(matches!(hint, StatementHint::Assignment { targets, value } if targets == vec!["sum"] && value == "a + b"));

        let hint = extract(&make_request("the variable sum is a plus b"));
        assert!(matches!(hint, StatementHint::Declaration { names, initial_value: Some(val), .. } if names == vec!["sum"] && val == "a + b"));

        let hint = extract(&make_request("sum variable is a plus b"));
        assert!(matches!(hint, StatementHint::Declaration { names, initial_value: Some(val), .. } if names == vec!["sum"] && val == "a + b"));

        let hint = extract(&make_request("let sum be a plus b"));
        assert!(matches!(hint, StatementHint::Declaration { names, initial_value: Some(val), .. } if names == vec!["sum"] && val == "a + b"));

        let hint = extract(&make_request("store a plus b in sum"));
        assert!(matches!(hint, StatementHint::Assignment { targets, value } if targets == vec!["sum"] && value == "a + b"));
    }

    #[test]
    fn test_expression_chaining() {
        let hint = extract(&make_request("set total to a plus b plus c"));
        assert!(matches!(hint, StatementHint::Assignment { targets, value } if targets == vec!["total"] && value == "a + b + c"));
    }

    #[test]
    fn test_loop_times_and_until() {
        let hint = extract(&make_request("loop 3 times"));
        assert!(matches!(hint, StatementHint::Loop { start: Some(s), end: Some(e), .. } if s == "0" && e == "2"));

        let hint = extract(&make_request("repeat until x equals 10"));
        assert!(matches!(hint, StatementHint::While { condition, .. } if condition == "!(x == 10)"));
    }

    #[test]
    fn test_array_access_patterns() {
        let hint = extract(&make_request("third element of numbers"));
        assert!(matches!(hint, StatementHint::ArrayAccess { array, index, value: None } if array == "numbers" && index == "2"));

        let hint = extract(&make_request("last element of values"));
        assert!(matches!(hint, StatementHint::ArrayAccess { array, index, value: None } if array == "values" && index == "last"));

        let hint = extract(&make_request("numbers at i"));
        assert!(matches!(hint, StatementHint::ArrayAccess { array, index, value: None } if array == "numbers" && index == "i"));
    }

    #[test]
    fn test_function_call_patterns() {
        let hint = extract(&make_request("call sqrt on x"));
        assert!(matches!(hint, StatementHint::FunctionCall { name, arguments } if name == "sqrt" && arguments == vec!["x"]));

        let hint = extract(&make_request("apply abs to n"));
        assert!(matches!(hint, StatementHint::FunctionCall { name, arguments } if name == "abs" && arguments == vec!["n"]));
    }

    #[test]
    fn test_string_and_memory_patterns() {
        let hint = extract(&make_request("append src to dest"));
        assert!(matches!(hint, StatementHint::Strcat { dest, src } if dest == "dest" && src == "src"));

        let hint = extract(&make_request("length of name"));
        assert!(matches!(hint, StatementHint::Strlen { target, .. } if target == "name"));

        let hint = extract(&make_request("copy source to dest"));
        assert!(matches!(hint, StatementHint::Strcpy { dest, src } if dest == "dest" && src == "source"));

        let hint = extract(&make_request("allocate 100 bytes"));
        assert!(matches!(hint, StatementHint::Malloc { count, element_type } if count == "100" && element_type == "char"));
    }

    #[test]
    fn test_loop() {
        let hint = extract(&make_request("loop from 0 to 10"));
        assert!(matches!(hint, StatementHint::Loop { .. }));
    }

    #[test]
    fn test_conditional() {
        let hint = extract(&make_request("if x > 5"));
        assert!(matches!(hint, StatementHint::Conditional { .. }));
    }

    #[test]
    fn test_print() {
        let hint = extract(&make_request("print hello world"));
        assert!(matches!(hint, StatementHint::Print { .. }));
    }

    #[test]
    fn test_error_handling_ext() {
        let hint = extract(&make_request("try"));
        assert!(matches!(hint, StatementHint::TryBlock));

        let hint = extract(&make_request("except IOError as e"));
        assert!(matches!(hint, StatementHint::ExceptBlock { exception_type: Some(t), variable: Some(v) } if t == "IOError" && v == "e"));

        let hint = extract(&make_request("raise ValueError with bad"));
        assert!(matches!(hint, StatementHint::RaiseException { exception_type, message: Some(msg) } if exception_type == "ValueError" && msg == "bad"));
    }

    #[test]
    fn test_testing_patterns() {
        let hint = extract(&make_request("test function add"));
        assert!(matches!(hint, StatementHint::TestFunction { name, .. } if name == "add"));

        let hint = extract(&make_request("assert x equals y"));
        assert!(matches!(hint, StatementHint::TestAssertEqual { left, right, .. } if left == "x" && right == "y"));
    }

    #[test]
    fn test_advanced_control_flow() {
        let hint = extract(&make_request("break outer"));
        assert!(matches!(hint, StatementHint::LabeledBreak { label } if label == "outer"));

        let hint = extract(&make_request("match value"));
        assert!(matches!(hint, StatementHint::MatchBlock { expression } if expression == "value"));

        let hint = extract(&make_request("guard x > 0 else return"));
        assert!(matches!(hint, StatementHint::GuardClause { condition, action } if condition == "x > 0" && action == "return"));
    }

    #[test]
    fn test_data_structures() {
        let hint = extract(&make_request("create stack mystack"));
        assert!(matches!(hint, StatementHint::StackCreate { name, .. } if name == "mystack"));

        let hint = extract(&make_request("put value in map mymap at key k"));
        assert!(matches!(hint, StatementHint::MapPut { map, key, value } if map == "mymap" && key == "k" && value == "value"));
    }

    #[test]
    fn test_oop_and_lambda() {
        let hint = extract(&make_request("class Foo"));
        assert!(matches!(hint, StatementHint::ClassDef { name, .. } if name == "Foo"));

        let hint = extract(&make_request("new Foo as bar"));
        assert!(matches!(hint, StatementHint::ObjectCreate { class_name, variable, .. } if class_name == "Foo" && variable == "bar"));

        let hint = extract(&make_request("x => x + 1"));
        assert!(matches!(hint, StatementHint::Lambda { .. }));
    }

    #[test]
    fn test_concurrency_and_async() {
        let hint = extract(&make_request("create thread worker"));
        assert!(matches!(hint, StatementHint::ThreadCreate { function, .. } if function == "worker"));

        let hint = extract(&make_request("async function fetch"));
        assert!(matches!(hint, StatementHint::AsyncFunction { name, .. } if name == "fetch"));
    }

    #[test]
    fn test_documentation() {
        let hint = extract(&make_request("document this is a test"));
        assert!(matches!(hint, StatementHint::DocComment { .. }));
    }
}

