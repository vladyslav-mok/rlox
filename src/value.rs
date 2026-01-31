use crate::chunk::Chunk;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Obj(Rc<Obj>),
}

impl Value {
    pub fn is_instance(&self) -> bool {
        matches!(self, Value::Obj(obj) if matches!(**obj, Obj::Instance(_)))
    }

    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(b) => !b,
            _ => false,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => match (&**a, &**b) {
                (Obj::String(s1), Obj::String(s2)) => Rc::ptr_eq(s1, s2),
                _ => Rc::ptr_eq(a, b),
            },
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::Obj(obj) => write!(f, "{}", obj),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Obj {
    String(Rc<str>),
    Function(Rc<Function>),
    Native(Rc<Native>),
    Closure(Rc<Closure>),
    Class(Rc<Class>),
    Instance(Rc<Instance>),
    BoundMethod(Rc<BoundMethod>),
}

impl fmt::Display for Obj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Obj::String(s) => write!(f, "{}", s),
            Obj::Function(func) => {
                if let Some(name) = &func.name {
                    write!(f, "<fn {}>", name)
                } else {
                    write!(f, "<script>")
                }
            }
            Obj::Native(_) => write!(f, "<native fn>"),
            Obj::Closure(closure) => {
                if let Some(name) = &closure.function.name {
                    write!(f, "<fn {}>", name)
                } else {
                    write!(f, "<script>")
                }
            }
            Obj::Class(class) => write!(f, "{}", class.name),
            Obj::Instance(instance) => {
                if let Some(class) = instance.class.upgrade() {
                    write!(f, "{} instance", class.name)
                } else {
                    write!(f, "<dropped class> instance")
                }
            }
            Obj::BoundMethod(bound) => {
                if let Some(name) = &bound.method.function.name {
                    write!(f, "<fn {}>", name)
                } else {
                    write!(f, "<script>")
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct StringInterner {
    strings: HashSet<Rc<str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: HashSet::new(),
        }
    }

    pub fn intern(&mut self, s: &str) -> Rc<str> {
        if let Some(existing) = self.strings.get(s) {
            return Rc::clone(existing);
        }
        let rc: Rc<str> = Rc::from(s);
        self.strings.insert(Rc::clone(&rc));
        rc
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub arity: usize,
    pub upvalue_count: usize,
    pub chunk: Chunk,
    pub name: Option<Rc<str>>,
}

impl Function {
    pub fn new() -> Self {
        Function {
            arity: 0,
            upvalue_count: 0,
            chunk: Chunk::new(),
            name: None,
        }
    }
}

impl Default for Function {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Native {
    pub function: fn(arg_count: usize, args: &[Value]) -> Value,
}

impl fmt::Debug for Native {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjNative")
            .field("function", &"<native fn>")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub function: Rc<Function>,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub location: usize,
    pub closed: Option<Value>,
}

impl Upvalue {
    pub fn get_value(&self, stack: &[Value]) -> Value {
        if let Some(closed) = &self.closed {
            closed.clone()
        } else {
            stack[self.location].clone()
        }
    }

    pub fn set_value(&mut self, value: Value, stack: &mut [Value]) {
        if self.closed.is_some() {
            self.closed = Some(value);
        } else {
            stack[self.location] = value;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: Rc<str>,
    pub methods: RefCell<HashMap<Rc<str>, Value>>,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub class: Weak<Class>,
    pub fields: RefCell<HashMap<Rc<str>, Value>>,
}

#[derive(Debug, Clone)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: Rc<Closure>,
}
