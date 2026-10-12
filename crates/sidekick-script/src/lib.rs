//! ZX Sidekick's scripts: a definition's [Rhai](https://rhai.rs) script, sandboxed. It reads
//! the game (memory, named values, the state) and returns facts the panel shows; it writes
//! only through `writable` values, and only from an action.
//!
//! A script has no files, network or clock, and every call is limited in operations, depth and
//! size. One that goes over a limit, or fails, is turned off: it says why, and stays off.

use rhai::{Array, CallFnOptions, Dynamic, Engine, EvalAltResult, Map, Position, Scope, AST};
use std::{cell::RefCell, collections::BTreeMap, fmt, rc::Rc};

/// Operations a call may take: enough for a search over a few thousand rooms.
const MOST_OPERATIONS: u64 = 1_000_000;
/// How deep calls may go.
const MOST_CALLS: usize = 32;
/// The longest string, and the most entries in an array or a map.
const MOST_STRING: usize = 4_096;
const MOST_ENTRIES: usize = 65_536;

/// A fact: what a script is given and what it returns, as plain data.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Fact {
    /// Nothing.
    #[default]
    Nothing,
    /// True or false.
    Flag(bool),
    /// A whole number.
    Number(i64),
    /// A number with a fraction.
    Real(f64),
    /// Text.
    Text(String),
    /// A list.
    List(Vec<Self>),
    /// Named facts.
    Map(BTreeMap<String, Self>),
}

impl Fact {
    fn into_dynamic(self) -> Dynamic {
        match self {
            Self::Nothing => Dynamic::UNIT,
            Self::Flag(b) => b.into(),
            Self::Number(n) => n.into(),
            Self::Real(r) => r.into(),
            Self::Text(t) => t.into(),
            Self::List(list) => list
                .into_iter()
                .map(Self::into_dynamic)
                .collect::<Array>()
                .into(),
            Self::Map(map) => map
                .into_iter()
                .map(|(k, v)| (k.into(), v.into_dynamic()))
                .collect::<Map>()
                .into(),
        }
    }

    fn from_dynamic(value: Dynamic) -> Self {
        if value.is_unit() {
            return Self::Nothing;
        }
        if let Ok(b) = value.as_bool() {
            return Self::Flag(b);
        }
        if let Ok(n) = value.as_int() {
            return Self::Number(n);
        }
        if let Ok(r) = value.as_float() {
            return Self::Real(r);
        }
        if value.is_string() {
            return Self::Text(value.into_string().unwrap_or_default());
        }
        if value.is_array() {
            let list = value.cast::<Array>();
            return Self::List(list.into_iter().map(Self::from_dynamic).collect());
        }
        if value.is_map() {
            let map = value.cast::<Map>();
            return Self::Map(
                map.into_iter()
                    .map(|(k, v)| (k.to_string(), Self::from_dynamic(v)))
                    .collect(),
            );
        }
        Self::Text(value.to_string())
    }
}

/// What a script sees of the game at a call: memory, the values read, the state, the
/// guidance level, and the kept lists.
#[derive(Clone, Debug, Default)]
pub struct View {
    /// The 64K the processor sees.
    pub memory: Vec<u8>,
    /// Every named value, as read this frame: one that couldn't be read is `Nothing`. A name
    /// that isn't here isn't a value, and asking for it is an error.
    pub values: BTreeMap<String, Fact>,
    /// The game's state.
    pub state: Option<String>,
    /// The guidance level.
    pub level: i64,
    /// The `[persist]` lists.
    pub kept: BTreeMap<String, Vec<String>>,
}

/// The writes an action asks for, by value name, in order.
type Writes = Rc<RefCell<Vec<(String, i64)>>>;

/// `mem`, as a script has it.
#[derive(Clone)]
struct Mem {
    memory: Rc<[u8]>,
    values: Rc<BTreeMap<String, Fact>>,
    state: Option<Rc<str>>,
    /// Writes asked for, by value name; `None` where writing isn't allowed.
    writes: Option<Writes>,
}

impl Mem {
    /// A named value; a name the definition doesn't have is an error, so a typo shows.
    fn value(&self, name: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.values
            .get(name)
            .map(|v| v.clone().into_dynamic())
            .ok_or_else(|| format!("no value named {name}").into())
    }

    fn byte(&self, addr: i64) -> i64 {
        usize::try_from(addr)
            .ok()
            .and_then(|a| self.memory.get(a))
            .map_or(0, |&b| i64::from(b))
    }
}

/// A script that couldn't be compiled: where, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileError {
    /// The line, when known.
    pub line: Option<usize>,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for CompileError {}

/// What an action function returned, and the writes it asked for.
pub type ActionResult = (BTreeMap<String, Fact>, Vec<(String, i64)>);

/// A definition's script, compiled.
pub struct Script {
    engine: Engine,
    ast: AST,
    /// What `on_load` returned, and what the last call did.
    loaded: Dynamic,
    prev: Dynamic,
    /// Why it was turned off, once it was.
    off: Option<String>,
}

impl Clone for Script {
    /// A copy with an engine of its own: engines aren't shared.
    fn clone(&self) -> Self {
        Self {
            engine: engine(),
            ast: self.ast.clone(),
            loaded: self.loaded.clone(),
            prev: self.prev.clone(),
            off: self.off.clone(),
        }
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script")
            .field("off", &self.off)
            .finish_non_exhaustive()
    }
}

/// The engine every script runs in: its limits, nothing it mustn't have, and `mem`.
fn engine() -> Engine {
    let mut engine = Engine::new();
    engine
        .set_max_operations(MOST_OPERATIONS)
        .set_max_call_levels(MOST_CALLS)
        .set_max_string_size(MOST_STRING)
        .set_max_array_size(MOST_ENTRIES)
        .set_max_map_size(MOST_ENTRIES)
        .set_max_expr_depths(64, 32);
    engine.disable_symbol("eval");
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
        .register_type_with_name::<Mem>("Mem")
        .register_fn("peek", |m: &mut Mem, addr: i64| m.byte(addr))
        .register_fn("peek16", |m: &mut Mem, addr: i64| {
            m.byte(addr) | (m.byte(addr + 1) << 8)
        })
        .register_fn("value", |m: &mut Mem, name: &str| m.value(name))
        .register_fn("table", |m: &mut Mem, name: &str| m.value(name))
        .register_get("state", |m: &mut Mem| {
            m.state
                .as_ref()
                .map_or(Dynamic::UNIT, |s| Dynamic::from(s.to_string()))
        })
        .register_fn(
            "write",
            |m: &mut Mem, name: &str, value: i64| -> Result<(), Box<EvalAltResult>> {
                let Some(writes) = &m.writes else {
                    return Err("mem.write is only for action functions".into());
                };
                writes.borrow_mut().push((name.to_owned(), value));
                Ok(())
            },
        );
    engine
}

impl Script {
    /// Compiles a script.
    ///
    /// # Errors
    /// A syntax error, with its line.
    pub fn compile(source: &str) -> Result<Self, CompileError> {
        let engine = engine();
        let ast = engine.compile(source).map_err(|e| CompileError {
            line: e.position().line(),
            message: e.err_type().to_string(),
        })?;
        Ok(Self {
            engine,
            ast,
            loaded: Dynamic::UNIT,
            prev: Dynamic::UNIT,
            off: None,
        })
    }

    /// Why the script was turned off, if it was.
    #[must_use]
    pub fn off(&self) -> Option<&str> {
        self.off.as_deref()
    }

    /// How many parameters a function of the script takes, if it has it.
    fn arity(&self, name: &str) -> Option<usize> {
        self.ast
            .iter_functions()
            .find(|f| f.name == name)
            .map(|f| f.params.len())
    }

    /// Whether the script has a function of this name.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.arity(name).is_some()
    }

    /// Calls a function with `mem`, then what it's given; a failure turns the script off.
    fn call(
        &mut self,
        name: &str,
        view: &View,
        writable: bool,
    ) -> Result<(Dynamic, Vec<(String, i64)>), String> {
        if let Some(why) = &self.off {
            return Err(why.clone());
        }
        let writes = writable.then(|| Rc::new(RefCell::new(Vec::new())));
        let mem = Mem {
            memory: Rc::from(view.memory.as_slice()),
            values: Rc::new(view.values.clone()),
            state: view.state.as_deref().map(Rc::from),
            writes: writes.clone(),
        };
        let kept: Map = view
            .kept
            .iter()
            .map(|(k, list)| {
                let list: Array = list.iter().cloned().map(Dynamic::from).collect();
                (k.into(), list.into())
            })
            .collect();
        // As many of mem, loaded, prev, kept and level as the function takes, in that order.
        let mut args = vec![
            Dynamic::from(mem),
            self.loaded.clone(),
            self.prev.clone(),
            Dynamic::from(kept),
            Dynamic::from(view.level),
        ];
        args.truncate(self.arity(name).unwrap_or(args.len()));
        let mut scope = Scope::new();
        let options = CallFnOptions::new().eval_ast(false);
        let result = self
            .engine
            .call_fn_with_options::<Dynamic>(options, &mut scope, &self.ast, name, args);
        match result {
            Ok(value) => Ok((value, writes.map(|w| w.take()).unwrap_or_default())),
            Err(e) => {
                let why = format!("{name}: {}", describe(&e));
                self.off = Some(why.clone());
                Err(why)
            }
        }
    }

    /// Runs `on_load`, if the script has it: once, when the tape has loaded. What it returns
    /// is given to every later call as `loaded`.
    ///
    /// # Errors
    /// The script failed, or went over a limit; it's off from now on.
    pub fn on_load(&mut self, view: &View) -> Result<(), String> {
        if self.has("on_load") {
            self.loaded = self.call("on_load", view, false)?.0;
        }
        Ok(())
    }

    /// Runs `on_frame`, if the script has it, and returns its results by name.
    ///
    /// # Errors
    /// The script failed, went over a limit, or didn't return a map; it's off from now on.
    pub fn on_frame(&mut self, view: &View) -> Result<BTreeMap<String, Fact>, String> {
        if !self.has("on_frame") {
            return Ok(BTreeMap::new());
        }
        let (value, _) = self.call("on_frame", view, false)?;
        self.results(value, "on_frame")
    }

    /// Runs an action function, which may write `writable` values: its results, and the
    /// writes it asked for, by value name, in order.
    ///
    /// # Errors
    /// No such function, or the script failed, went over a limit, or didn't return a map.
    pub fn action(&mut self, name: &str, view: &View) -> Result<ActionResult, String> {
        if !self.has(name) {
            return Err(format!("the script has no function {name}"));
        }
        let (value, writes) = self.call(name, view, true)?;
        Ok((self.results(value, name)?, writes))
    }

    fn results(&mut self, value: Dynamic, name: &str) -> Result<BTreeMap<String, Fact>, String> {
        if value.is_unit() {
            return Ok(BTreeMap::new());
        }
        self.prev = value.clone();
        if let Fact::Map(map) = Fact::from_dynamic(value) {
            Ok(map)
        } else {
            let why = format!("{name}: should return a map of results");
            self.off = Some(why.clone());
            Err(why)
        }
    }
}

/// An error as a line for the panel.
fn describe(error: &EvalAltResult) -> String {
    let line = match error.position() {
        Position::NONE => String::new(),
        p => p
            .line()
            .map_or_else(String::new, |l| format!(" (line {l})")),
    };
    let what = match error {
        EvalAltResult::ErrorTooManyOperations(_) => "took too many steps".to_owned(),
        EvalAltResult::ErrorStackOverflow(_) => "called too deep".to_owned(),
        EvalAltResult::ErrorDataTooLarge(what, _) => format!("{what} too large"),
        other => other.to_string(),
    };
    format!("{what}{line}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> View {
        let mut memory = vec![0; 0x1_0000];
        memory[0x8000] = 0x34;
        memory[0x8001] = 0x12;
        View {
            memory,
            values: [
                ("lives".to_owned(), Fact::Number(3)),
                (
                    "items".to_owned(),
                    Fact::List(vec![Fact::Map(BTreeMap::from([(
                        "room".to_owned(),
                        Fact::Number(7),
                    )]))]),
                ),
            ]
            .into_iter()
            .collect(),
            state: Some("play".to_owned()),
            level: 2,
            kept: BTreeMap::from([("codes".to_owned(), vec!["TOPAZ".to_owned()])]),
        }
    }

    #[test]
    fn a_frame_sees_the_game_and_returns_its_results() {
        let mut script = Script::compile(
            r#"
            fn on_frame(mem, loaded, prev, kept, level) {
                #{
                    word: mem.peek16(0x8000),
                    byte: mem.peek(0x8001),
                    lives: mem.value("lives"),
                    room: mem.table("items")[0].room,
                    state: mem.state,
                    level: level,
                    code: kept.codes[0],
                    calls: if prev == () { 1 } else { prev.calls + 1 },
                }
            }
            "#,
        )
        .unwrap();
        let first = script.on_frame(&view()).unwrap();
        let number = |r: &BTreeMap<String, Fact>, k: &str| r[k].clone();
        assert_eq!(number(&first, "word"), Fact::Number(0x1234));
        assert_eq!(number(&first, "byte"), Fact::Number(0x12));
        assert_eq!(number(&first, "lives"), Fact::Number(3));
        assert_eq!(number(&first, "room"), Fact::Number(7));
        assert_eq!(number(&first, "state"), Fact::Text("play".to_owned()));
        assert_eq!(number(&first, "level"), Fact::Number(2));
        assert_eq!(number(&first, "code"), Fact::Text("TOPAZ".to_owned()));
        let second = script.on_frame(&view()).unwrap();
        assert_eq!(
            number(&second, "calls"),
            Fact::Number(2),
            "prev is the last result"
        );
    }

    #[test]
    fn what_on_load_returns_is_given_to_every_frame() {
        let mut script = Script::compile(
            r"
            fn on_load(mem, loaded, prev, kept, level) { [1, 2, 3] }
            fn on_frame(mem, loaded, prev, kept, level) { #{ total: loaded[0] + loaded[2] } }
            ",
        )
        .unwrap();
        script.on_load(&view()).unwrap();
        assert_eq!(script.on_frame(&view()).unwrap()["total"], Fact::Number(4));
    }

    #[test]
    fn going_over_a_limit_turns_the_script_off() {
        let mut endless =
            Script::compile(r"fn on_frame(mem, loaded, prev, kept, level) { loop { } }").unwrap();
        let why = endless.on_frame(&view()).unwrap_err();
        assert_eq!(why, "on_frame: took too many steps");
        assert_eq!(endless.off(), Some(why.as_str()));
        assert!(endless.on_frame(&view()).is_err(), "and stays off");

        let mut deep = Script::compile(
            r"
            fn down(n) { down(n + 1) }
            fn on_frame(mem, loaded, prev, kept, level) { down(0) }
            ",
        )
        .unwrap();
        assert!(deep
            .on_frame(&view())
            .unwrap_err()
            .contains("called too deep"));

        let mut long = Script::compile(
            r#"fn on_frame(mem, loaded, prev, kept, level) { let s = "x"; loop { s += s; } }"#,
        )
        .unwrap();
        assert!(long.on_frame(&view()).unwrap_err().contains("too large"));
    }

    #[test]
    fn writes_are_only_for_actions() {
        let mut script = Script::compile(
            r#"
            fn on_frame(mem, loaded, prev, kept, level) { mem.write("lives", 9); #{} }
            fn more_lives(mem, loaded, prev, kept, level) {
                mem.write("lives", mem.value("lives") + 1);
                #{ done: true }
            }
            "#,
        )
        .unwrap();
        let (results, writes) = script.action("more_lives", &view()).unwrap();
        assert_eq!(results["done"], Fact::Flag(true));
        assert_eq!(writes, [("lives".to_owned(), 4)]);
        assert!(script
            .on_frame(&view())
            .unwrap_err()
            .contains("mem.write is only for action functions"));
        assert!(script.action("no_such", &view()).is_err());
    }

    #[test]
    fn a_function_gets_as_many_arguments_as_it_takes() {
        let mut script = Script::compile(
            r"
            fn on_load(mem) { mem.peek(0x8000) }
            fn on_frame(mem, loaded) { #{ loaded: loaded } }
            ",
        )
        .unwrap();
        script.on_load(&view()).unwrap();
        assert_eq!(
            script.on_frame(&view()).unwrap()["loaded"],
            Fact::Number(0x34)
        );
    }

    #[test]
    fn a_name_that_isn_t_a_value_stops_the_script() {
        let mut script =
            Script::compile(r#"fn on_frame(mem) { #{ lives: mem.value("lifes") } }"#).unwrap();
        assert!(script
            .on_frame(&view())
            .unwrap_err()
            .contains("no value named lifes"));
        let mut unread =
            Script::compile(r#"fn on_frame(mem) { #{ none: mem.value("unread") == () } }"#)
                .unwrap();
        let mut seen = view();
        seen.values.insert("unread".to_owned(), Fact::Nothing);
        assert_eq!(unread.on_frame(&seen).unwrap()["none"], Fact::Flag(true));
    }

    #[test]
    fn a_syntax_error_says_where() {
        let error = Script::compile("fn on_frame( {").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.to_string().starts_with("line 1: "));
        assert!(
            Script::compile(r#"fn f() { eval("1") }"#).is_err(),
            "no eval"
        );
    }

    #[test]
    fn a_frame_that_returns_no_map_is_an_error() {
        let mut script =
            Script::compile(r"fn on_frame(mem, loaded, prev, kept, level) { 42 }").unwrap();
        assert_eq!(
            script.on_frame(&view()).unwrap_err(),
            "on_frame: should return a map of results"
        );
    }
}
